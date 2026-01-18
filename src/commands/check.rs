//! Check command - validates the task graph.

use crate::error_fmt::AppError;
use crate::MontContext;

/// Validate the task graph, optionally checking a specific task.
pub fn check(ctx: &MontContext, id: Option<&str>) -> Result<(), AppError> {
    let graph = ctx.graph();

    if graph.is_empty() {
        println!("No tasks found");
        return Ok(());
    }

    match id {
        Some(task_id) => {
            if !graph.contains(task_id) {
                return Err(AppError::TaskNotFound {
                    task_id: task_id.to_string(),
                    tasks_dir: ctx.tasks_dir().display().to_string(),
                });
            }
            println!("ok: task '{}' is valid", task_id);
        }
        None => {
            println!("ok: {} tasks validated", graph.len());
        }
    }

    Ok(())
}
