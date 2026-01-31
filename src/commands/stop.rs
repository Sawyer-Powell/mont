//! Stop command - clear in-progress status from a task.

use crate::error_fmt::AppError;
use crate::MontContext;

/// Stop working on a task, making it ready for work again.
///
/// Validates that the task exists and is in-progress,
/// then clears the in-progress status.
pub fn stop(ctx: &MontContext, id: &str) -> Result<(), AppError> {
    // Validate task exists
    let graph = ctx.graph();
    let task = graph.get(id).ok_or_else(|| AppError::TaskNotFound {
        task_id: id.to_string(),
        tasks_dir: ctx.tasks_dir().display().to_string(),
    })?;

    // Check if task is in progress
    if !task.is_in_progress() {
        return Err(AppError::TaskNotInProgress(id.to_string()));
    }

    // Clear the in-progress status
    let mut updated_task = task.clone();
    updated_task.status = None;
    drop(graph);

    ctx.update(id, updated_task)?;

    println!("Stopped task '{}'", id);
    Ok(())
}
