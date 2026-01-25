//! Done command - mark a task as complete and commit.

use owo_colors::OwoColorize;

use crate::error_fmt::AppError;
use crate::{jj, GateStatus, MontContext, Status};

/// Complete a task.
///
/// If no task ID is provided, detects the in-progress task from the task graph.
///
/// If a message is provided, uses it for the commit. Otherwise opens
/// the default editor via `jj commit`.
pub fn done(ctx: &MontContext, id: Option<&str>, message: Option<&str>) -> Result<(), AppError> {
    // Determine which task to complete
    let task_id = match id {
        Some(id) => id.to_string(),
        None => detect_in_progress_task(ctx)?,
    };

    let graph = ctx.graph();
    let task = graph.get(&task_id).ok_or_else(|| AppError::TaskNotFound {
        task_id: task_id.clone(),
        tasks_dir: ctx.tasks_dir().display().to_string(),
    })?;

    // Check task is in progress
    if !task.is_in_progress() {
        return Err(AppError::TaskNotInProgress(task_id.clone()));
    }

    // Jots cannot be completed - they must be distilled into tasks first
    if task.is_jot() {
        return Err(AppError::CannotCompleteJot(task_id.clone()));
    }

    // Check all gates are passed or skipped
    let all_gate_ids = ctx.all_gate_ids(task);
    let mut blocking_gates: Vec<(String, GateStatus)> = Vec::new();

    for gate_id in &all_gate_ids {
        // Find gate status - check task's gates list first
        let status = task
            .gates
            .iter()
            .find(|g| &g.id == gate_id)
            .map(|g| g.status)
            .unwrap_or(GateStatus::Pending);

        match status {
            GateStatus::Passed | GateStatus::Skipped => {}
            _ => blocking_gates.push((gate_id.clone(), status)),
        }
    }

    if !blocking_gates.is_empty() {
        return Err(AppError::GatesNotPassed {
            task_id: task_id.clone(),
            blocking: blocking_gates,
        });
    }

    // Mark task as complete
    let mut updated_task = task.clone();
    updated_task.status = Some(Status::Complete);
    drop(graph);

    ctx.update(&task_id, updated_task)?;

    println!("Marked '{}' as complete", task_id.green());
    println!();

    // Run jj commit (skip if jj is disabled)
    let jj_enabled = ctx.config().jj.enabled;
    if jj_enabled {
        match message {
            Some(msg) => {
                jj::commit(msg, &[]).map_err(|e| AppError::JJError(e.to_string()))?;
            }
            None => {
                jj::commit_interactive().map_err(|e| AppError::JJError(e.to_string()))?;
            }
        }
    }

    Ok(())
}

/// Detect the in-progress task from the task graph.
///
/// Queries the graph for tasks with status: inprogress.
fn detect_in_progress_task(ctx: &MontContext) -> Result<String, AppError> {
    let graph = ctx.graph();
    let in_progress: Vec<String> = graph
        .values()
        .filter(|t| t.is_in_progress())
        .map(|t| t.id.clone())
        .collect();

    match in_progress.as_slice() {
        [] => Err(AppError::NoInProgressTask),
        [single] => Ok(single.clone()),
        _ => Err(AppError::MultipleInProgressTasks(in_progress)),
    }
}
