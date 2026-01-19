//! LLM commands - generate prompts and manage LLM-assisted workflows.

use minijinja::{context, Environment};

use crate::error_fmt::AppError;
use crate::{jj, GateStatus, MontContext, Task};

// Embed templates at compile time (numbered by state machine order)
const TEMPLATE_NO_TASK: &str = include_str!("../prompts/00_no-task-in-progress.md");
const TEMPLATE_NO_CODE_CHANGES: &str = include_str!("../prompts/01_no-code-changes.md");
const TEMPLATE_HAS_CODE_CHANGES: &str = include_str!("../prompts/02_has-code-changes.md");
const TEMPLATE_SOME_GATES_UNLOCKED: &str = include_str!("../prompts/03_some-gates-unlocked.md");
const TEMPLATE_ALL_GATES_UNLOCKED: &str = include_str!("../prompts/04_all-gates-unlocked.md");

/// State of the task graph from the LLM's perspective.
#[derive(Debug)]
pub enum TaskGraphState {
    /// No task is currently in progress.
    NoTaskInProgress {
        has_uncommitted_changes: bool,
    },
    /// A task is in progress with the given sub-state.
    TaskInProgress {
        task: Box<Task>,
        state: InProgressState,
    },
}

/// State of an in-progress task.
#[derive(Debug)]
pub enum InProgressState {
    /// No code changes outside .tasks/ - need to start implementation.
    NoCodeChanges,
    /// Has code changes but no gates unlocked yet.
    HasCodeChanges {
        first_gate: Option<GateInfo>,
    },
    /// Some gates unlocked but not all.
    SomeGatesUnlocked {
        unlocked: Vec<String>,
        pending: Vec<String>,
        next_gate: Option<GateInfo>,
    },
    /// All gates unlocked - ready for mont done.
    AllGatesUnlocked,
}

/// Information about a gate for templating.
#[derive(Debug, Clone)]
pub struct GateInfo {
    pub id: String,
    pub title: Option<String>,
    pub description: String,
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
    let task = in_progress[0].clone();

    // Determine the in-progress state
    let state = detect_in_progress_state(ctx, &task)?;

    Ok(TaskGraphState::TaskInProgress { task: Box::new(task), state })
}

/// Detect the state of an in-progress task.
fn detect_in_progress_state(ctx: &MontContext, task: &Task) -> Result<InProgressState, AppError> {
    let graph = ctx.graph();
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

    // Helper to get gate info
    let get_gate_info = |gate_id: &str| -> Option<GateInfo> {
        graph.get(gate_id).map(|gate_task| GateInfo {
            id: gate_id.to_string(),
            title: gate_task.title.clone(),
            description: gate_task.description.clone(),
        })
    };

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
        let first_gate = pending.first().and_then(|id| get_gate_info(id));
        Ok(InProgressState::HasCodeChanges { first_gate })
    } else {
        let next_gate = pending.first().and_then(|id| get_gate_info(id));
        Ok(InProgressState::SomeGatesUnlocked { unlocked, pending, next_gate })
    }
}

/// Generate a prompt based on the current state.
pub fn generate_prompt(_ctx: &MontContext, state: &TaskGraphState) -> Result<String, AppError> {
    let mut env = Environment::new();

    // Add templates
    env.add_template("no-task", TEMPLATE_NO_TASK)
        .map_err(|e| AppError::TemplateError(e.to_string()))?;
    env.add_template("no-code-changes", TEMPLATE_NO_CODE_CHANGES)
        .map_err(|e| AppError::TemplateError(e.to_string()))?;
    env.add_template("has-code-changes", TEMPLATE_HAS_CODE_CHANGES)
        .map_err(|e| AppError::TemplateError(e.to_string()))?;
    env.add_template("some-gates-unlocked", TEMPLATE_SOME_GATES_UNLOCKED)
        .map_err(|e| AppError::TemplateError(e.to_string()))?;
    env.add_template("all-gates-unlocked", TEMPLATE_ALL_GATES_UNLOCKED)
        .map_err(|e| AppError::TemplateError(e.to_string()))?;

    match state {
        TaskGraphState::NoTaskInProgress { has_uncommitted_changes } => {
            let tmpl = env.get_template("no-task")
                .map_err(|e| AppError::TemplateError(e.to_string()))?;
            tmpl.render(context! { has_uncommitted_changes })
                .map_err(|e| AppError::TemplateError(e.to_string()))
        }
        TaskGraphState::TaskInProgress { task, state } => {
            render_in_progress_prompt(&env, task, state)
        }
    }
}

fn render_in_progress_prompt(
    env: &Environment,
    task: &Task,
    state: &InProgressState,
) -> Result<String, AppError> {
    let task_id = &task.id;
    let task_title = task.title.as_deref().unwrap_or("");
    let task_description = &task.description;

    match state {
        InProgressState::NoCodeChanges => {
            let tmpl = env.get_template("no-code-changes")
                .map_err(|e| AppError::TemplateError(e.to_string()))?;
            tmpl.render(context! {
                task_id,
                task_title,
                task_description,
            })
            .map_err(|e| AppError::TemplateError(e.to_string()))
        }

        InProgressState::HasCodeChanges { first_gate } => {
            let tmpl = env.get_template("has-code-changes")
                .map_err(|e| AppError::TemplateError(e.to_string()))?;

            let (gate_id, gate_title, gate_description) = match first_gate {
                Some(g) => (g.id.as_str(), g.title.as_deref().unwrap_or(""), g.description.as_str()),
                None => ("", "", ""),
            };

            tmpl.render(context! {
                task_id,
                task_title,
                task_description,
                gate_id,
                gate_title,
                gate_description,
            })
            .map_err(|e| AppError::TemplateError(e.to_string()))
        }

        InProgressState::SomeGatesUnlocked { unlocked, pending, next_gate } => {
            let tmpl = env.get_template("some-gates-unlocked")
                .map_err(|e| AppError::TemplateError(e.to_string()))?;

            let (gate_id, gate_title, gate_description) = match next_gate {
                Some(g) => (g.id.as_str(), g.title.as_deref().unwrap_or(""), g.description.as_str()),
                None => ("", "", ""),
            };

            tmpl.render(context! {
                task_id,
                task_title,
                gates_unlocked => unlocked.join(", "),
                gates_pending => pending.join(", "),
                gate_id,
                gate_title,
                gate_description,
            })
            .map_err(|e| AppError::TemplateError(e.to_string()))
        }

        InProgressState::AllGatesUnlocked => {
            let tmpl = env.get_template("all-gates-unlocked")
                .map_err(|e| AppError::TemplateError(e.to_string()))?;
            tmpl.render(context! {
                task_id,
                task_title,
            })
            .map_err(|e| AppError::TemplateError(e.to_string()))
        }
    }
}

/// Run the `mont prompt` command.
pub fn prompt(ctx: &MontContext) -> Result<(), AppError> {
    let state = detect_state(ctx)?;
    let prompt = generate_prompt(ctx, &state)?;
    print!("{}", prompt);
    Ok(())
}

/// System prompt for Claude Code sessions.
const CLAUDE_SYSTEM_PROMPT: &str = r#"After completing your current work, always run `mont prompt` to get the next task or instructions. This ensures you stay synchronized with the task graph and receive appropriate guidance for your next steps."#;

/// Run the `mont claude` command.
/// Launches Claude Code with a generated prompt based on current task state.
///
/// - `task_id`: The task to work on
/// - `ignore`: If true, bypass the uncommitted changes validation
pub fn claude(ctx: &MontContext, task_id: &str, ignore: bool) -> Result<(), AppError> {
    let graph = ctx.graph();

    // Check if the task exists
    let task = graph
        .get(task_id)
        .ok_or_else(|| AppError::TaskNotFound {
            task_id: task_id.to_string(),
            tasks_dir: ctx.tasks_dir().to_string_lossy().to_string(),
        })?;

    // Check for uncommitted changes
    let has_changes = !jj::is_working_copy_empty()
        .map_err(|e| AppError::JJError(e.to_string()))?;

    if has_changes && !ignore {
        // Check if the requested task is the one in progress
        let in_progress_task = graph.values().find(|t| t.is_in_progress());

        match in_progress_task {
            Some(in_progress) if in_progress.id == task_id => {
                // The requested task is already in progress, proceed
            }
            Some(in_progress) => {
                return Err(AppError::CommandFailed(format!(
                    "There are uncommitted changes, but task '{}' is in progress (not '{}').\n\
                     Either commit your changes first, or run:\n  \
                     mont claude {} (to continue the in-progress task)\n  \
                     mont claude {} --ignore (to start anyway)",
                    in_progress.id, task_id, in_progress.id, task_id
                )));
            }
            None => {
                return Err(AppError::CommandFailed(format!(
                    "There are uncommitted changes but no task is in progress.\n\
                     Either commit your changes first, or run:\n  \
                     mont claude {} --ignore (to start anyway)",
                    task_id
                )));
            }
        }
    }

    // Start the task if it's not already in progress
    if !task.is_in_progress() {
        crate::commands::start(ctx, task_id)?;
    }

    // Generate prompt based on current state
    let state = detect_state(ctx)?;
    let prompt = generate_prompt(ctx, &state)?;

    let status = std::process::Command::new("claude")
        .arg("--permission-mode=acceptEdits")
        .arg("--append-system-prompt")
        .arg(CLAUDE_SYSTEM_PROMPT)
        .arg(&prompt)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .map_err(|e| AppError::CommandFailed(format!("failed to launch claude: {}", e)))?;

    if !status.success() {
        return Err(AppError::CommandFailed("claude exited with error".to_string()));
    }

    Ok(())
}
