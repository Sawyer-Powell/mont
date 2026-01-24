//! Unified task command - creates, edits, and deletes tasks in one editor session.

use std::io::{self, Write};
use std::path::PathBuf;

use owo_colors::OwoColorize;

use super::shared::{
    build_multiedit_comment, find_most_recent_temp_file, make_temp_file, MultiEditMode,
    parse_multi_task_content, remove_temp_file, resolve_ids, TaskFilter,
};
use crate::error_fmt::AppError;
use crate::multieditor::{apply_diff, compute_diff, ApplyResult};
use crate::{resolve_editor, MontContext, Task, TaskType};

/// Arguments for the unified task command.
pub struct TaskArgs {
    /// Task IDs to edit (if any). If empty, opens empty multieditor.
    pub ids: Vec<String>,
    /// Task type template (task, jot, gate)
    pub task_type: Option<TaskType>,
    /// Resume editing the most recent temp file
    pub resume: bool,
    /// Resume editing a specific temp file
    pub resume_path: Option<PathBuf>,
    /// Skip editor, use content directly (LLM/scripting)
    pub content: Option<String>,
    /// JSON patch to merge into task (requires single ID)
    pub patch: Option<String>,
    /// Append text to task description (requires single ID)
    pub append: Option<String>,
    /// Editor name override
    pub editor: Option<String>,
}

/// Run the unified task command.
pub fn task(ctx: &MontContext, args: TaskArgs) -> Result<(), AppError> {
    // Resume mode: --resume or --resume-path
    if args.resume || args.resume_path.is_some() {
        return resume_mode(ctx, args);
    }

    // Content mode: --content (skip editor)
    if let Some(content) = args.content {
        return content_mode(ctx, &content, &args.ids);
    }

    // Resolve `?` placeholders in IDs via interactive picker
    let ids = if args.ids.iter().any(|id| id == "?") {
        resolve_ids(&ctx.graph(), &args.ids, TaskFilter::Active)?
    } else {
        args.ids.clone()
    };

    // Patch mode: --patch (JSON merge, single ID)
    if let Some(patch) = args.patch {
        return patch_mode(ctx, &ids, &patch);
    }

    // Append mode: --append (add to description, single ID)
    if let Some(text) = args.append {
        return append_mode(ctx, &ids, &text);
    }

    // Editor mode: open editor for creating/editing tasks
    if ids.is_empty() {
        // Empty multieditor - create new tasks
        create_mode(ctx, args.task_type, args.editor.as_deref())
    } else {
        // Edit specific tasks
        edit_mode(ctx, &ids, args.editor.as_deref())
    }
}

/// Resume editing from a temp file.
fn resume_mode(ctx: &MontContext, args: TaskArgs) -> Result<(), AppError> {
    let temp_path = if let Some(path) = args.resume_path {
        path
    } else {
        // Find most recent temp file
        find_most_recent_temp_file("task")
            .ok_or_else(|| AppError::TempFileNotFound("No recent task temp file found".to_string()))?
    };

    if !temp_path.exists() {
        return Err(AppError::TempFileNotFound(temp_path.display().to_string()));
    }

    // Parse the temp file to get original tasks (if any)
    // For resume, we can't determine original tasks, so treat all as new
    run_editor_workflow(ctx, &temp_path, &[], args.editor.as_deref())
}

/// Create tasks from direct content (skip editor).
fn content_mode(ctx: &MontContext, content: &str, ids: &[String]) -> Result<(), AppError> {
    let temp_path = std::env::temp_dir().join("content_mode.md");
    std::fs::write(&temp_path, content)
        .map_err(|e| AppError::Io {
            context: "failed to write content".to_string(),
            source: e,
        })?;

    let edited = parse_multi_task_content(content, &temp_path)?;

    // Get original tasks if editing
    let original: Vec<Task> = ids.iter()
        .filter_map(|id| ctx.graph().get(id).cloned())
        .collect();

    // Compute and apply diff
    let diff = compute_diff(&original, &edited);

    if diff.is_empty() {
        println!("No changes.");
        return Ok(());
    }

    // Apply changes directly (no confirmation in content mode)
    let result = apply_diff(ctx, diff)?;
    print_result(ctx, &result);

    Ok(())
}

/// YAML patch struct for merging into tasks.
#[derive(serde::Deserialize, Default)]
struct TaskPatch {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    before: Option<Vec<String>>,
    #[serde(default)]
    after: Option<Vec<String>>,
    #[serde(default)]
    gates: Option<Vec<String>>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    r#type: Option<String>,
}

/// Apply a YAML patch to a single task.
fn patch_mode(ctx: &MontContext, ids: &[String], patch_yaml: &str) -> Result<(), AppError> {
    // Require exactly one ID
    if ids.len() != 1 {
        return Err(AppError::InvalidArgs(
            "--patch requires exactly one task ID".to_string(),
        ));
    }
    let original_id = &ids[0];

    // Parse the patch
    let patch: TaskPatch = serde_yaml::from_str(patch_yaml)
        .map_err(|e| AppError::InvalidArgs(format!("invalid YAML patch: {}", e)))?;

    // Get the task
    let graph = ctx.graph();
    let mut task = graph.get(original_id)
        .ok_or_else(|| AppError::TaskNotFound {
            task_id: original_id.clone(),
            tasks_dir: ctx.tasks_dir().display().to_string(),
        })?
        .clone();
    drop(graph);

    // Check if ID is being changed
    let new_id = patch.id.clone();
    let id_changed = new_id.as_ref().is_some_and(|new| new != original_id);

    // Confirm rename operation
    if let Some(new) = new_id.as_ref().filter(|_| id_changed) {
        print!(
            "Rename {} -> {}? [y/N] ",
            original_id.bright_yellow(),
            new.bright_green()
        );
        io::stdout().flush().ok();

        let mut input = String::new();
        io::stdin().read_line(&mut input).ok();
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Rename cancelled.");
            return Ok(());
        }
    }

    // Apply patch fields
    if let Some(new) = new_id {
        task.id = new;
    }
    if let Some(title) = patch.title {
        task.title = Some(title);
    }
    if let Some(desc) = patch.description {
        task.description = desc;
    }
    if let Some(before) = patch.before {
        task.before = before;
    }
    if let Some(after) = patch.after {
        task.after = after;
    }
    if let Some(gates) = patch.gates {
        use crate::{GateItem, GateStatus};
        task.gates = gates.into_iter().map(|g| GateItem {
            id: g,
            status: GateStatus::Pending,
        }).collect();
    }
    if let Some(status) = patch.status {
        use crate::Status;
        task.status = match status.to_lowercase().as_str() {
            "inprogress" | "in-progress" | "in_progress" => Some(Status::InProgress),
            "stopped" => Some(Status::Stopped),
            "complete" | "done" => Some(Status::Complete),
            "" | "pending" | "ready" => None,
            _ => return Err(AppError::InvalidArgs(format!("invalid status: {}", status))),
        };
    }
    if let Some(task_type) = patch.r#type {
        task.task_type = match task_type.to_lowercase().as_str() {
            "task" => TaskType::Task,
            "jot" => TaskType::Jot,
            "gate" => TaskType::Gate,
            _ => return Err(AppError::InvalidArgs(format!("invalid type: {}", task_type))),
        };
    }

    // Update the task (this handles reference rewriting if ID changed)
    ctx.update(original_id, task.clone())?;

    if id_changed {
        println!(
            "renamed: {} -> {}",
            original_id.bright_yellow(),
            task.id.bright_green()
        );
    } else {
        let file_path = ctx.tasks_dir().join(format!("{}.md", original_id));
        println!("updated: {}", file_path.display().to_string().bright_blue());
    }

    Ok(())
}

/// Append text to a task's description.
fn append_mode(ctx: &MontContext, ids: &[String], text: &str) -> Result<(), AppError> {
    // Require exactly one ID
    if ids.len() != 1 {
        return Err(AppError::InvalidArgs(
            "--append requires exactly one task ID".to_string(),
        ));
    }
    let id = &ids[0];

    // Get the task
    let graph = ctx.graph();
    let mut task = graph.get(id)
        .ok_or_else(|| AppError::TaskNotFound {
            task_id: id.clone(),
            tasks_dir: ctx.tasks_dir().display().to_string(),
        })?
        .clone();
    drop(graph);

    // Append to description
    if !task.description.is_empty() && !task.description.ends_with('\n') {
        task.description.push('\n');
    }
    if !task.description.is_empty() {
        task.description.push('\n');
    }
    task.description.push_str(text);

    // Update the task
    ctx.update(id, task)?;

    let file_path = ctx.tasks_dir().join(format!("{}.md", id));
    println!("updated: {}", file_path.display().to_string().bright_blue());

    Ok(())
}

/// Create new tasks in empty multieditor.
fn create_mode(
    ctx: &MontContext,
    task_type: Option<TaskType>,
    editor_name: Option<&str>,
) -> Result<(), AppError> {
    let mode = match task_type {
        Some(tt) => MultiEditMode::CreateWithType(tt),
        None => MultiEditMode::Create,
    };

    let comment = build_multiedit_comment(mode);

    // Create starter task based on type
    let starter = match task_type {
        Some(TaskType::Gate) => {
            Task {
                id: "new-gate".to_string(),
                new_id: None,
                title: Some("New Gate".to_string()),
                description: "Description here.".to_string(),
                before: vec![],
                after: vec![],
                gates: vec![],
                task_type: TaskType::Gate,
                status: None,
                deleted: false,
            }
        }
        Some(TaskType::Jot) => {
            Task {
                id: "new-jot".to_string(),
                new_id: None,
                title: Some("New Jot".to_string()),
                description: "Quick idea here.".to_string(),
                before: vec![],
                after: vec![],
                gates: vec![],
                task_type: TaskType::Jot,
                status: None,
                deleted: false,
            }
        }
        _ => {
            Task {
                id: "new-task".to_string(),
                new_id: None,
                title: Some("New Task".to_string()),
                description: "Description here.".to_string(),
                before: vec![],
                after: vec![],
                gates: vec![],
                task_type: TaskType::Task,
                status: None,
                deleted: false,
            }
        }
    };

    let temp_path = make_temp_file("task", &[starter], Some(&comment))?;

    // No original tasks - all edits are inserts
    run_editor_workflow(ctx, &temp_path, &[], editor_name)
}

/// Edit existing tasks.
fn edit_mode(
    ctx: &MontContext,
    ids: &[String],
    editor_name: Option<&str>,
) -> Result<(), AppError> {
    // Collect tasks to edit
    let mut original_tasks = Vec::new();
    for id in ids {
        let task = ctx
            .graph()
            .get(id)
            .ok_or_else(|| AppError::TaskNotFound {
                task_id: id.clone(),
                tasks_dir: ctx.tasks_dir().display().to_string(),
            })?
            .clone();
        original_tasks.push(task);
    }

    let comment = build_multiedit_comment(MultiEditMode::Edit);
    let temp_path = make_temp_file("task", &original_tasks, Some(&comment))?;

    run_editor_workflow(ctx, &temp_path, &original_tasks, editor_name)
}

/// Core editor workflow: open editor, parse result, compute diff, confirm, apply.
fn run_editor_workflow(
    ctx: &MontContext,
    temp_path: &PathBuf,
    original_tasks: &[Task],
    editor_name: Option<&str>,
) -> Result<(), AppError> {
    let temp_path_str = temp_path.display().to_string();

    // Open editor
    let mut cmd = resolve_editor(editor_name, temp_path)?;
    cmd.status().map_err(|e| AppError::Io {
        context: "failed to run editor".to_string(),
        source: e,
    })?;

    // Parse edited content
    let edited = match parse_multi_task_content(
        &std::fs::read_to_string(temp_path).map_err(|e| AppError::Io {
            context: "failed to read temp file".to_string(),
            source: e,
        })?,
        temp_path,
    ) {
        Ok(tasks) => tasks,
        Err(e) => {
            return Err(AppError::TempValidationFailed {
                error: Box::new(e),
                temp_path: temp_path_str,
                editor_name: editor_name.map(String::from),
            });
        }
    };

    // Handle empty result (user deleted everything)
    if edited.is_empty() && original_tasks.is_empty() {
        println!("No tasks defined, aborting.");
        remove_temp_file(temp_path)?;
        return Ok(());
    }

    // Compute diff
    let diff = compute_diff(original_tasks, &edited);

    if diff.is_empty() {
        println!("No changes.");
        remove_temp_file(temp_path)?;
        return Ok(());
    }

    // Show summary and confirm
    print_diff_summary(&diff, original_tasks, &edited);

    if !confirm_changes()? {
        println!(
            "Changes not applied. Resume with: {}",
            format!("mont task --resume-path {}", temp_path_str).cyan()
        );
        return Ok(());
    }

    // Apply changes
    match apply_diff(ctx, diff) {
        Ok(result) => {
            print_result(ctx, &result);
            remove_temp_file(temp_path)?;
            Ok(())
        }
        Err(e) => {
            Err(AppError::TempValidationFailed {
                error: Box::new(e),
                temp_path: temp_path_str,
                editor_name: editor_name.map(String::from),
            })
        }
    }
}

/// Print a summary of changes to be made.
fn print_diff_summary(
    diff: &crate::multieditor::MultiEditDiff,
    original: &[Task],
    _edited: &[Task],
) {
    println!("\n{}", "Changes detected:".bold());

    // Created tasks
    if !diff.inserts.is_empty() {
        let ids: Vec<_> = diff.inserts.iter().map(|t| t.id.as_str()).collect();
        println!("  {}: {}", "created".bright_green(), ids.join(", "));
    }

    // Updated tasks
    let mut renamed = Vec::new();
    let mut updated = Vec::new();
    for (original_id, new_task) in &diff.updates {
        if original_id != &new_task.id {
            renamed.push(format!("{} -> {}", original_id, new_task.id));
        } else {
            updated.push(original_id.as_str());
        }
    }
    if !updated.is_empty() {
        println!("  {}: {}", "updated".bright_blue(), updated.join(", "));
    }
    if !renamed.is_empty() {
        println!("  {}: {}", "renamed".bright_yellow(), renamed.join(", "));
    }

    // Deleted tasks
    if !diff.deletes.is_empty() {
        // Get titles for deleted tasks
        let deleted_info: Vec<String> = diff.deletes.iter().map(|id| {
            if let Some(task) = original.iter().find(|t| &t.id == id) {
                if let Some(title) = &task.title {
                    format!("{} ({})", id, title)
                } else {
                    id.clone()
                }
            } else {
                id.clone()
            }
        }).collect();
        println!("  {}: {}", "deleted".bright_red(), deleted_info.join(", "));
    }

    // Warning when creates == deletes (potential forgotten rename)
    if !diff.inserts.is_empty()
        && !diff.deletes.is_empty()
        && diff.inserts.len() == diff.deletes.len()
    {
        println!(
            "  {}: Did you mean to rename? Use {} field instead of changing id.",
            "warning".yellow().bold(),
            "new_id".cyan()
        );
    }

    println!();
}

/// Prompt user to confirm changes.
fn confirm_changes() -> Result<bool, AppError> {
    print!("Apply these changes? [y/N] ");
    io::stdout().flush().map_err(|e| AppError::Io {
        context: "failed to flush stdout".to_string(),
        source: e,
    })?;

    let mut input = String::new();
    io::stdin().read_line(&mut input).map_err(|e| AppError::Io {
        context: "failed to read input".to_string(),
        source: e,
    })?;

    let input = input.trim().to_lowercase();
    Ok(input == "y" || input == "yes")
}

/// Print the result of applying changes.
fn print_result(ctx: &MontContext, result: &ApplyResult) {
    for id in &result.created {
        let file_path = ctx.tasks_dir().join(format!("{}.md", id));
        println!("created: {}", file_path.display().to_string().bright_green());
    }

    for (original_id, new_id, id_changed) in &result.updated {
        if *id_changed {
            println!(
                "renamed: {} -> {}",
                original_id.bright_yellow(),
                new_id.bright_green()
            );
        } else {
            let file_path = ctx.tasks_dir().join(format!("{}.md", new_id));
            println!("updated: {}", file_path.display().to_string().bright_blue());
        }
    }

    for id in &result.deleted {
        println!("deleted: {}", id.bright_red());
    }
}
