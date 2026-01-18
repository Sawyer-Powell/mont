//! New command - creates a new task.

use std::path::{Path, PathBuf};

use owo_colors::OwoColorize;

use super::shared::{create_via_editor, make_temp_file};
use crate::error_fmt::AppError;
use crate::{MontContext, Task, TaskType, ValidationItem, ValidationStatus};

/// Arguments for creating a new task.
pub struct NewArgs {
    pub id: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub before: Vec<String>,
    pub after: Vec<String>,
    pub validations: Vec<String>,
    pub task_type: Option<TaskType>,
    pub editor: Option<Option<String>>,
    pub resume: Option<PathBuf>,
}

/// Create a new task.
pub fn new(ctx: &MontContext, args: NewArgs) -> Result<(), AppError> {
    // Resume mode: re-open a temp file from a previous failed validation
    if let Some(temp_path) = args.resume {
        let editor_name = args.editor.flatten();
        return resume_from_temp(ctx, &temp_path, editor_name.as_deref());
    }

    // Build a Task from arguments
    let task = build_task_from_args(
        args.id.unwrap_or_default(),
        args.title,
        args.description,
        args.before,
        args.after,
        args.validations,
        args.task_type,
    );

    // Editor mode: create temp file, open editor, validate on save
    if let Some(editor_opt) = args.editor {
        let editor_name = editor_opt.as_deref();
        return create_with_editor(ctx, &task, editor_name);
    }

    // Non-editor mode: require id or title
    if task.id.is_empty() && task.title.is_none() {
        return Err(AppError::IdOrTitleRequired);
    }

    // Insert the task
    let task_id = ctx.insert(task)?;

    let file_path = ctx.tasks_dir().join(format!("{}.md", task_id));
    println!("created: {}", file_path.display().to_string().bright_green());

    Ok(())
}

fn build_task_from_args(
    id: String,
    title: Option<String>,
    description: Option<String>,
    before: Vec<String>,
    after: Vec<String>,
    validations: Vec<String>,
    task_type: Option<TaskType>,
) -> Task {
    Task {
        id,
        title,
        description: description.unwrap_or_default(),
        before,
        after,
        validations: validations
            .into_iter()
            .map(|id| ValidationItem {
                id,
                status: ValidationStatus::Pending,
            })
            .collect(),
        task_type: task_type.unwrap_or(TaskType::Task),
        status: None,
        deleted: false,
    }
}

fn create_with_editor(
    ctx: &MontContext,
    task: &Task,
    editor_name: Option<&str>,
) -> Result<(), AppError> {
    let suffix = if task.id.is_empty() { "new" } else { &task.id };
    let suffix = format!("new_{}", suffix);
    let path = make_temp_file(&suffix, std::slice::from_ref(task), None)?;

    run_editor_workflow(ctx, &path, editor_name)
}

fn resume_from_temp(
    ctx: &MontContext,
    temp_path: &Path,
    editor_name: Option<&str>,
) -> Result<(), AppError> {
    if !temp_path.exists() {
        return Err(AppError::TempFileNotFound(temp_path.display().to_string()));
    }

    run_editor_workflow(ctx, temp_path, editor_name)
}

fn run_editor_workflow(
    ctx: &MontContext,
    path: &std::path::Path,
    editor_name: Option<&str>,
) -> Result<(), AppError> {
    let created_ids = create_via_editor(ctx, path, editor_name)?;

    if created_ids.is_empty() {
        println!("No tasks defined, aborting.");
        return Ok(());
    }

    for task_id in &created_ids {
        let file_path = ctx.tasks_dir().join(format!("{}.md", task_id));
        println!("created: {}", file_path.display().to_string().bright_green());
    }

    Ok(())
}
