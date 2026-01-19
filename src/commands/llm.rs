//! LLM commands - generate prompts and manage LLM-assisted workflows.

use crate::error_fmt::AppError;
use crate::{jj, GateStatus, MontContext, Task};

/// State of the task graph from the LLM's perspective.
#[derive(Debug)]
pub enum TaskGraphState {
    /// No task is currently in progress.
    NoTaskInProgress {
        has_uncommitted_changes: bool,
    },
    /// A task is in progress with the given sub-state.
    TaskInProgress {
        task: Task,
        state: InProgressState,
    },
}

/// State of an in-progress task.
#[derive(Debug)]
pub enum InProgressState {
    /// No code changes outside .tasks/ - need to start implementation.
    NoCodeChanges,
    /// Has code changes but no gates unlocked yet.
    HasCodeChanges,
    /// Some gates unlocked but not all.
    SomeGatesUnlocked {
        unlocked: Vec<String>,
        pending: Vec<String>,
    },
    /// All gates unlocked - ready for mont done.
    AllGatesUnlocked,
}

/// Detect the current state of the task graph for LLM prompting.
pub fn detect_state(ctx: &MontContext) -> Result<TaskGraphState, AppError> {
    let graph = ctx.graph();

    // Find in-progress task
    let in_progress: Vec<_> = graph.values().filter(|t| t.is_in_progress()).collect();

    if in_progress.is_empty() {
        let has_changes = !jj::is_working_copy_empty()
            .map_err(|e| AppError::JJError(e.to_string()))?;
        return Ok(TaskGraphState::NoTaskInProgress {
            has_uncommitted_changes: has_changes,
        });
    }

    // For now, just use the first in-progress task
    // (could error if multiple, but let's keep it simple)
    let task = in_progress[0].clone();

    // Determine the in-progress state
    let state = detect_in_progress_state(ctx, &task)?;

    Ok(TaskGraphState::TaskInProgress { task, state })
}

/// Detect the state of an in-progress task.
fn detect_in_progress_state(ctx: &MontContext, task: &Task) -> Result<InProgressState, AppError> {
    // Get all gate IDs for this task (task gates + default gates)
    let all_gate_ids = ctx.all_gate_ids(task);

    // Categorize gates by status
    let mut unlocked: Vec<String> = Vec::new();
    let mut pending: Vec<String> = Vec::new();

    for gate_id in &all_gate_ids {
        let status = task
            .gates
            .iter()
            .find(|g| &g.id == gate_id)
            .map(|g| g.status)
            .unwrap_or(GateStatus::Pending);

        match status {
            GateStatus::Passed | GateStatus::Skipped => unlocked.push(gate_id.clone()),
            GateStatus::Pending | GateStatus::Failed => pending.push(gate_id.clone()),
        }
    }

    // If all gates unlocked, ready for mont done
    if pending.is_empty() {
        return Ok(InProgressState::AllGatesUnlocked);
    }

    // Check for code changes
    let has_code_changes = jj::has_code_changes()
        .map_err(|e| AppError::JJError(e.to_string()))?;

    if !has_code_changes {
        return Ok(InProgressState::NoCodeChanges);
    }

    // Has code changes - check if any gates unlocked
    if unlocked.is_empty() {
        Ok(InProgressState::HasCodeChanges)
    } else {
        Ok(InProgressState::SomeGatesUnlocked { unlocked, pending })
    }
}

/// Generate a prompt based on the current state.
pub fn generate_prompt(ctx: &MontContext, state: &TaskGraphState) -> Result<String, AppError> {
    match state {
        TaskGraphState::NoTaskInProgress { has_uncommitted_changes } => {
            generate_no_task_prompt(*has_uncommitted_changes)
        }
        TaskGraphState::TaskInProgress { task, state } => {
            generate_in_progress_prompt(ctx, task, state)
        }
    }
}

fn generate_no_task_prompt(has_uncommitted_changes: bool) -> Result<String, AppError> {
    let mut prompt = String::new();

    prompt.push_str("There is currently no task marked as in progress.\n\n");

    if has_uncommitted_changes {
        prompt.push_str("However, this revision has uncommitted changes.\n\n");
        prompt.push_str("Please review the current changes with `jj diff` and either:\n");
        prompt.push_str("1. Commit them with `jj commit -m \"message\"` if they are complete\n");
        prompt.push_str("2. Start a task that relates to this work with `mont start <task-id>`\n");
        prompt.push_str("3. Abandon the changes with `jj abandon` if they are not needed\n");
    } else {
        prompt.push_str("To begin work, start a task with `mont start <task-id>`.\n\n");
        prompt.push_str("You can see available tasks with `mont ready`.\n");
    }

    Ok(prompt)
}

fn generate_in_progress_prompt(
    ctx: &MontContext,
    task: &Task,
    state: &InProgressState,
) -> Result<String, AppError> {
    let mut prompt = String::new();

    // Always include task info header
    prompt.push_str(&format!("# Task: {}\n\n", task.id));
    if let Some(title) = &task.title {
        prompt.push_str(&format!("**{}**\n\n", title));
    }

    match state {
        InProgressState::NoCodeChanges => {
            prompt.push_str("## Status: Ready to implement\n\n");
            prompt.push_str("No code changes have been made yet. Read the task description below and begin implementation.\n\n");
            prompt.push_str("### Task Description\n\n");
            if task.description.is_empty() {
                prompt.push_str("*No description provided.*\n");
            } else {
                prompt.push_str(&task.description);
                prompt.push('\n');
            }
            prompt.push_str("\n### Guidelines\n\n");
            prompt.push_str("1. Implement the task as described\n");
            prompt.push_str("2. Keep changes focused and minimal\n");
            prompt.push_str("3. When implementation is complete, run `mont llm prompt` for next steps\n");
        }

        InProgressState::HasCodeChanges => {
            prompt.push_str("## Status: Implementation in progress\n\n");
            prompt.push_str("Code changes have been made. Review the implementation against the task requirements.\n\n");
            prompt.push_str("### Task Description\n\n");
            if task.description.is_empty() {
                prompt.push_str("*No description provided.*\n");
            } else {
                prompt.push_str(&task.description);
                prompt.push('\n');
            }

            // Get first gate info
            let all_gate_ids = ctx.all_gate_ids(task);
            if let Some(first_gate_id) = all_gate_ids.iter().next() {
                prompt.push_str(&format!("\n### Next Step: Verify gate `{}`\n\n", first_gate_id));

                // Try to get gate task description
                let graph = ctx.graph();
                if let Some(gate_task) = graph.get(first_gate_id) {
                    if let Some(title) = &gate_task.title {
                        prompt.push_str(&format!("**{}**\n\n", title));
                    }
                    if !gate_task.description.is_empty() {
                        prompt.push_str(&gate_task.description);
                        prompt.push('\n');
                    }
                }

                prompt.push_str("\nOnce verified, mark the gate as passed:\n");
                prompt.push_str(&format!("`mont unlock {} --passed {}`\n", task.id, first_gate_id));
            }
        }

        InProgressState::SomeGatesUnlocked { unlocked, pending } => {
            prompt.push_str("## Status: Verification in progress\n\n");
            prompt.push_str(&format!("Gates passed: {}\n", unlocked.join(", ")));
            prompt.push_str(&format!("Gates remaining: {}\n\n", pending.join(", ")));

            // Get next pending gate info
            if let Some(next_gate_id) = pending.first() {
                prompt.push_str(&format!("### Next Step: Verify gate `{}`\n\n", next_gate_id));

                let graph = ctx.graph();
                if let Some(gate_task) = graph.get(next_gate_id) {
                    if let Some(title) = &gate_task.title {
                        prompt.push_str(&format!("**{}**\n\n", title));
                    }
                    if !gate_task.description.is_empty() {
                        prompt.push_str(&gate_task.description);
                        prompt.push('\n');
                    }
                }

                prompt.push_str("\nOnce verified, mark the gate as passed:\n");
                prompt.push_str(&format!("`mont unlock {} --passed {}`\n", task.id, next_gate_id));
                prompt.push_str("\nOnce the gate is passed, run mont llm prompt again:\n");
            }
        }

        InProgressState::AllGatesUnlocked => {
            prompt.push_str("## Status: Ready to complete\n\n");
            prompt.push_str("All gates have been unlocked. The task is ready to be marked as complete.\n\n");
            prompt.push_str("### Next Steps\n\n");
            prompt.push_str("1. Review the changes with `jj diff`\n");
            prompt.push_str("2. Construct an appropriate commit message summarizing the work\n");
            prompt.push_str("3. Complete the task with `mont done -m \"your commit message\"`\n");
        }
    }

    Ok(prompt)
}

/// Run the `mont llm prompt` command.
pub fn prompt(ctx: &MontContext) -> Result<(), AppError> {
    let state = detect_state(ctx)?;
    let prompt = generate_prompt(ctx, &state)?;
    print!("{}", prompt);
    Ok(())
}

/// Run the `mont llm start` command.
pub fn start(ctx: &MontContext, task_id: &str) -> Result<(), AppError> {
    // First, start the task using the regular start command
    crate::commands::start(ctx, task_id)?;

    // Then generate and print the initial prompt
    let state = detect_state(ctx)?;
    let prompt = generate_prompt(ctx, &state)?;

    println!();
    print!("{}", prompt);

    Ok(())
}
