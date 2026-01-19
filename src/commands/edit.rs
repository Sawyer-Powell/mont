//! Edit command - edits an existing task.

use std::path::{Path, PathBuf};

use owo_colors::OwoColorize;

use super::shared::{make_temp_file, update_via_editor, UpdateResult};
use crate::error_fmt::AppError;
use crate::{MontContext, Task, TaskType, GateItem, GateStatus};

/// Arguments for editing a task.
pub struct EditArgs {
    pub id: String,
    pub new_id: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub before: Vec<String>,
    pub after: Vec<String>,
    pub gates: Vec<String>,
    pub task_type: Option<TaskType>,
    pub editor: Option<Option<String>>,
    pub resume: Option<PathBuf>,
}

/// Edit an existing task.
pub fn edit(ctx: &MontContext, args: EditArgs) -> Result<(), AppError> {
    // Resume mode: re-open a temp file from a previous failed validation
    if let Some(temp_path) = args.resume {
        let editor_name = args.editor.flatten();
        return resume_edit_from_temp(ctx, &args.id, &temp_path, editor_name.as_deref());
    }

    // Get the original task
    let original = ctx
        .graph()
        .get(&args.id)
        .ok_or_else(|| AppError::TaskNotFound {
            task_id: args.id.clone(),
            tasks_dir: ctx.tasks_dir().display().to_string(),
        })?
        .clone();

    let final_id = args.new_id.as_deref().unwrap_or(&args.id);

    // Editor mode: open in editor
    if let Some(editor_opt) = args.editor {
        let editor_name = editor_opt.as_deref();
        return edit_with_editor(ctx, &args.id, &original, editor_name);
    }

    // Check if any fields were provided (non-editor mode requires at least one change)
    let has_changes = args.new_id.is_some()
        || args.title.is_some()
        || args.description.is_some()
        || !args.before.is_empty()
        || !args.after.is_empty()
        || !args.gates.is_empty()
        || args.task_type.is_some();

    if !has_changes {
        return Err(AppError::NoChangesProvided);
    }

    // Build updated task by merging fields
    let updated = merge_task(&original, final_id, &args);

    // Update the task (this validates, updates references, and saves)
    ctx.update(&args.id, updated)?;

    if args.new_id.is_some() && final_id != args.id {
        println!(
            "renamed: {} -> {}",
            args.id.bright_yellow(),
            final_id.bright_green()
        );
    } else {
        let file_path = ctx.tasks_dir().join(format!("{}.md", final_id));
        println!("updated: {}", file_path.display().to_string().bright_green());
    }

    Ok(())
}

fn merge_task(original: &Task, new_id: &str, args: &EditArgs) -> Task {
    Task {
        id: new_id.to_string(),
        title: args.title.clone().or_else(|| original.title.clone()),
        description: args
            .description
            .clone()
            .unwrap_or_else(|| original.description.clone()),
        before: if args.before.is_empty() {
            original.before.clone()
        } else {
            args.before.clone()
        },
        after: if args.after.is_empty() {
            original.after.clone()
        } else {
            args.after.clone()
        },
        gates: if args.gates.is_empty() {
            original.gates.clone()
        } else {
            args.gates
                .iter()
                .map(|id| GateItem {
                    id: id.clone(),
                    status: GateStatus::Pending,
                })
                .collect()
        },
        task_type: args.task_type.unwrap_or(original.task_type),
        status: original.status,
        deleted: false,
    }
}

fn edit_with_editor(
    ctx: &MontContext,
    original_id: &str,
    task: &Task,
    editor_name: Option<&str>,
) -> Result<(), AppError> {
    let suffix = format!("edit_{}", original_id);
    let path = make_temp_file(&suffix, std::slice::from_ref(task), None)?;

    run_editor_workflow(ctx, original_id, &path, editor_name)
}

fn resume_edit_from_temp(
    ctx: &MontContext,
    original_id: &str,
    temp_path: &Path,
    editor_name: Option<&str>,
) -> Result<(), AppError> {
    if !temp_path.exists() {
        return Err(AppError::TempFileNotFound(temp_path.display().to_string()));
    }

    // Verify the original task exists
    if !ctx.graph().contains(original_id) {
        return Err(AppError::TaskNotFound {
            task_id: original_id.to_string(),
            tasks_dir: ctx.tasks_dir().display().to_string(),
        });
    }

    run_editor_workflow(ctx, original_id, temp_path, editor_name)
}

fn run_editor_workflow(
    ctx: &MontContext,
    original_id: &str,
    path: &Path,
    editor_name: Option<&str>,
) -> Result<(), AppError> {
    match update_via_editor(ctx, original_id, path, editor_name)? {
        UpdateResult::Updated { new_id, id_changed } => {
            if id_changed {
                println!(
                    "renamed: {} -> {}",
                    original_id.bright_yellow(),
                    new_id.bright_green()
                );
            } else {
                let file_path = ctx.tasks_dir().join(format!("{}.md", new_id));
                println!("updated: {}", file_path.display().to_string().bright_green());
            }
        }
        UpdateResult::Aborted => {
            println!("No task defined, aborting edit.");
        }
    }

    Ok(())
}
