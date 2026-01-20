//! Start command - begin working on a task.

use crate::error_fmt::AppError;
use crate::{jj, MontContext, Status};

/// Start working on a task.
///
/// Validates that the task exists, that the working copy is empty,
/// and marks the task as in-progress.
pub fn start(ctx: &MontContext, id: &str) -> Result<(), AppError> {
    // Validate task exists
    let graph = ctx.graph();
    let task = graph.get(id).ok_or_else(|| AppError::TaskNotFound {
        task_id: id.to_string(),
        tasks_dir: ctx.tasks_dir().display().to_string(),
    })?;

    // Check if task is already complete
    if task.is_complete() {
        return Err(AppError::TaskAlreadyComplete(id.to_string()));
    }

    // Check if task is already in progress
    if task.is_in_progress() {
        return Err(AppError::TaskAlreadyInProgress(id.to_string()));
    }

    // Check if working copy is empty (skip if jj is disabled)
    let jj_enabled = ctx.config().jj.enabled;
    if jj_enabled {
        let is_empty = jj::is_working_copy_empty().map_err(|e| AppError::JJError(e.to_string()))?;
        if !is_empty {
            return Err(AppError::WorkingCopyNotEmpty);
        }
    }

    // Update task status to in-progress
    let mut updated_task = task.clone();
    updated_task.status = Some(Status::InProgress);
    drop(graph);

    ctx.update(id, updated_task)?;

    println!("Started task '{}'", id);
    Ok(())
}
