//! Distill command - converts a jot into one or more proper tasks.

use owo_colors::OwoColorize;
use serde::Deserialize;

use super::shared::{find_temp_files, make_temp_file, parse_temp_file, remove_temp_file};
use crate::error_fmt::{AppError, IoResultExt};
use crate::{jj, resolve_editor, MontContext, Task, TaskType};

/// Distill a jot into one or more proper tasks.
///
/// If `tasks_yaml` is provided, uses those task definitions directly (LLM-friendly mode).
/// Otherwise, opens the editor for interactive task definition.
pub fn distill(ctx: &MontContext, id: &str, tasks_yaml: Option<&str>) -> Result<(), AppError> {
    // Get the jot task
    let jot_task = ctx
        .graph()
        .get(id)
        .ok_or_else(|| AppError::TaskNotFound {
            task_id: id.to_string(),
            tasks_dir: ctx.tasks_dir().display().to_string(),
        })?
        .clone();

    // Verify this is a jot
    if jot_task.task_type != TaskType::Jot {
        return Err(AppError::NotAJot(id.to_string()));
    }

    // Get new tasks either from YAML parameter or editor workflow
    let (new_tasks, temp_path) = if let Some(yaml) = tasks_yaml {
        // LLM-friendly mode: parse tasks from YAML parameter
        let tasks = parse_tasks_yaml(yaml)?;
        (tasks, None)
    } else {
        // Editor mode: create temp file with template
        let suffix = format!("distill_{}", id);
        let comment = build_distill_comment(&jot_task);
        let starter_task = build_starter_task(&jot_task);
        let path = make_temp_file(&suffix, &[starter_task], Some(&comment))?;

        let mut cmd = resolve_editor(None, &path)?;
        cmd.status().with_context("failed to run editor")?;

        // Parse the edited content
        let tasks = parse_temp_file(&path)?;
        (tasks, Some(path))
    };

    if new_tasks.is_empty() {
        println!("No tasks defined, aborting distill.");
        if let Some(ref path) = temp_path {
            remove_temp_file(path)?;
        }
        return Ok(());
    }

    // Build a single transaction for atomicity
    let mut txn = ctx.begin();
    let mut created_ids = Vec::new();

    // Add all inserts to the transaction
    for task in &new_tasks {
        created_ids.push(task.id.clone());
        txn.insert(task.clone());
    }

    // Add delete for the original jot (and rewrite any references)
    let graph = ctx.graph();
    txn.rewrite_references(&*graph, id, None);
    drop(graph);
    txn.delete(id);

    // Commit atomically
    ctx.commit(txn)?;

    // Print results after successful commit
    for task_id in &created_ids {
        let file_path = ctx.tasks_dir().join(format!("{}.md", task_id));
        println!("created: {}", file_path.display().to_string().bright_green());
    }
    println!("{} {}", "deleted jot:".yellow(), id.bright_yellow());

    // Clean up temp file (if editor mode was used)
    if let Some(ref path) = temp_path {
        remove_temp_file(path)?;
    }

    // Auto-commit with jj (skip if jj is disabled)
    let jj_enabled = ctx.config().jj.enabled;
    if jj_enabled {
        let commit_msg = format!("Distilled jot '{}' into tasks: {}", id, created_ids.join(", "));
        match jj::commit(&commit_msg) {
            Ok(_) => println!("{} {}", "committed:".bright_green(), commit_msg),
            Err(e) => eprintln!("{} {}", "warning: jj commit failed:".yellow(), e),
        }
    }

    Ok(())
}

/// Find the most recent distill temp file for resuming.
#[allow(dead_code)]
pub fn find_distill_temp_file(id: &str) -> Option<std::path::PathBuf> {
    let suffix = format!("distill_{}", id);
    find_temp_files(&suffix).into_iter().next()
}

/// Build the instruction comment for distill temp file.
fn build_distill_comment(jot: &Task) -> String {
    format!(
        r#"Distill: {}

Define one or more tasks below. Each task starts with --- and ends with ---
After saving, the jot will be deleted and these tasks will be created.
Tasks without an id: field will get an auto-generated ID.

Example:
---
id: first-task
title: First Task Title
---
Description of the first task

---
id: second-task
title: Second Task Title
after:
  - first-task
---
Description of the second task"#,
        jot.title.as_deref().unwrap_or(&jot.id)
    )
}

/// Build a starter task from the jot for the distill template.
fn build_starter_task(jot: &Task) -> Task {
    Task {
        id: "new-task".to_string(),
        title: Some(jot.title.clone().unwrap_or_else(|| "New Task".to_string())),
        description: jot.description.clone(),
        before: vec![],
        after: vec![],
        gates: vec![],
        task_type: TaskType::Task,
        status: None,
        deleted: false,
    }
}

/// Task definition from YAML input for LLM-friendly distill.
#[derive(Debug, Deserialize)]
struct TaskDef {
    id: String,
    title: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    before: Vec<String>,
    #[serde(default)]
    after: Vec<String>,
}

/// Parse tasks from YAML input (LLM-friendly mode).
fn parse_tasks_yaml(yaml: &str) -> Result<Vec<Task>, AppError> {
    let defs: Vec<TaskDef> = serde_yaml::from_str(yaml)
        .map_err(|e| AppError::CommandFailed(format!("invalid tasks YAML: {}", e)))?;

    let tasks = defs
        .into_iter()
        .map(|def| Task {
            id: def.id,
            title: Some(def.title),
            description: def.description,
            before: def.before,
            after: def.after,
            gates: vec![],
            task_type: TaskType::Task,
            status: None,
            deleted: false,
        })
        .collect();

    Ok(tasks)
}
