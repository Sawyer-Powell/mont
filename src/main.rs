use clap::{Parser, Subcommand};
use std::path::PathBuf;

use mont::commands;
use mont::commands::shared::{pick_task, TaskFilter};
use mont::error_fmt::AppError;
use mont::TaskType;

#[derive(Parser)]
#[command(name = "mont")]
#[command(about = "Task management and agent coordination")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Show status of in-progress tasks (default command)
    Status,
    /// List all tasks in the task graph
    List {
        /// Show completed tasks (hidden by default)
        #[arg(long)]
        show_completed: bool,
    },
    /// Show tasks ready to work on
    Ready,
    /// Validate the task graph
    Check {
        /// Specific task ID to validate (validates entire graph if not provided)
        id: Option<String>,
    },
    /// Create or edit tasks (opens multieditor)
    Task {
        /// Task ID(s) to edit (comma-separated). If empty, opens empty multieditor.
        #[arg(value_delimiter = ',')]
        ids: Vec<String>,
        /// Task type template: task, jot, gate
        #[arg(long, short = 'T', value_parser = parse_task_type)]
        r#type: Option<TaskType>,
        /// Resume editing the most recent temp file
        #[arg(long, conflicts_with_all = ["ids", "type", "content", "patch", "append"])]
        resume: bool,
        /// Resume editing a specific temp file
        #[arg(long, conflicts_with_all = ["ids", "type", "content", "resume", "patch", "append"])]
        resume_path: Option<PathBuf>,
        /// Skip editor, use content directly (LLM/scripting)
        #[arg(long, conflicts_with_all = ["type", "resume", "resume_path", "patch", "append"])]
        content: Option<String>,
        /// YAML patch to merge into task (requires single ID)
        #[arg(long, conflicts_with_all = ["type", "resume", "resume_path", "content", "append"])]
        patch: Option<String>,
        /// Append text to task description (requires single ID)
        #[arg(long, conflicts_with_all = ["type", "resume", "resume_path", "content", "patch"])]
        append: Option<String>,
        /// Editor command to use
        #[arg(long, short)]
        editor: Option<String>,
    },
    /// Delete a task and remove all references to it
    Delete {
        /// Task ID to delete. If not provided, opens interactive picker.
        id: Option<String>,
        /// Force deletion without confirmation prompt
        #[arg(long, short)]
        force: bool,
    },
    /// Show details for a single task
    Show {
        /// Task ID to show. If not provided, opens interactive picker.
        id: Option<String>,
        /// Show shortened version (omit description)
        #[arg(long, short)]
        short: bool,
    },
    /// Mark gates as passed or skipped
    Unlock {
        /// Task ID. If not provided, opens interactive picker.
        id: Option<String>,
        /// Gates to mark as passed (comma-separated)
        #[arg(long, short, value_delimiter = ',')]
        passed: Vec<String>,
        /// Gates to mark as skipped (comma-separated)
        #[arg(long, short, value_delimiter = ',')]
        skipped: Vec<String>,
    },
    /// Reset gates back to pending
    Lock {
        /// Task ID. If not provided, opens interactive picker.
        id: Option<String>,
        /// Gates to reset to pending (comma-separated)
        #[arg(long, short, value_delimiter = ',')]
        gates: Vec<String>,
    },
    /// Start working on a task
    Start {
        /// Task ID to start. If not provided, opens interactive picker.
        id: Option<String>,
    },
    /// Complete a task and commit
    Done {
        /// Task ID to complete. If not provided, detects from current revision.
        id: Option<String>,
        /// Commit message (opens editor if not provided)
        #[arg(long, short)]
        message: Option<String>,
    },
    /// Generate a prompt based on current task state
    Prompt,
    /// Launch Claude Code with generated prompt
    Claude {
        /// Task ID to work on. If not provided, opens interactive picker.
        id: Option<String>,
        /// Ignore uncommitted changes validation and proceed anyway
        #[arg(long, short)]
        ignore: bool,
    },
}

fn parse_task_type(s: &str) -> Result<TaskType, String> {
    match s.to_lowercase().as_str() {
        "task" => Ok(TaskType::Task),
        "jot" => Ok(TaskType::Jot),
        "gate" => Ok(TaskType::Gate),
        _ => Err(format!(
            "invalid task type '{}', must be one of: task, jot, gate",
            s
        )),
    }
}

fn main() {
    let cli = Cli::parse();

    if let Err(e) = run(cli) {
        eprint!("{}", e);
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<(), AppError> {
    // Load context once for all commands
    let ctx = mont::MontContext::load(PathBuf::from(".tasks"))?;

    // Default to Status if no command is provided
    let command = cli.command.unwrap_or(Commands::Status);

    match command {
        Commands::Status => {
            commands::status(&ctx);
            Ok(())
        }
        Commands::List { show_completed } => {
            commands::list(&ctx, show_completed);
            Ok(())
        }
        Commands::Ready => {
            commands::ready(&ctx);
            Ok(())
        }
        Commands::Check { id } => commands::check(&ctx, id.as_deref()),
        Commands::Task {
            ids,
            r#type,
            resume,
            resume_path,
            content,
            patch,
            append,
            editor,
        } => commands::task(
            &ctx,
            commands::task_cmd::TaskArgs {
                ids,
                task_type: r#type,
                resume,
                resume_path,
                content,
                patch,
                append,
                editor,
            },
        ),
        Commands::Delete { id, force } => {
            let resolved_id = match id {
                Some(id) => id,
                None => pick_task(&ctx.graph(), TaskFilter::Active)?,
            };
            commands::delete(&ctx, &resolved_id, force)
        }
        Commands::Show { id, short } => {
            let resolved_id = match id {
                Some(id) => id,
                None => pick_task(&ctx.graph(), TaskFilter::All)?,
            };
            commands::show(&ctx, &resolved_id, short)
        }
        Commands::Unlock { id, passed, skipped } => {
            let resolved_id = match id {
                Some(id) => id,
                None => pick_task(&ctx.graph(), TaskFilter::InProgress)?,
            };
            commands::unlock(
                &ctx,
                commands::unlock::UnlockArgs {
                    id: resolved_id,
                    passed,
                    skipped,
                },
            )
        }
        Commands::Lock { id, gates } => {
            let resolved_id = match id {
                Some(id) => id,
                None => pick_task(&ctx.graph(), TaskFilter::InProgress)?,
            };
            commands::unlock::lock(
                &ctx,
                commands::unlock::LockArgs {
                    id: resolved_id,
                    gates,
                },
            )
        }
        Commands::Start { id } => {
            let resolved_id = match id {
                Some(id) => id,
                None => pick_task(&ctx.graph(), TaskFilter::Active)?,
            };
            commands::start(&ctx, &resolved_id)
        }
        Commands::Done { id, message } => commands::done(&ctx, id.as_deref(), message.as_deref()),
        Commands::Prompt => commands::prompt(&ctx),
        Commands::Claude { id, ignore } => {
            if ignore {
                // --ignore: bypass all validation, just spawn claude with current prompt
                commands::claude_ignore(&ctx)
            } else {
                // Need a task id - from arg or picker
                let resolved_id = match id {
                    Some(id) => id,
                    None => {
                        // Pre-validate before showing picker to avoid wasting user time
                        commands::claude_pre_validate(&ctx)?;
                        pick_task(&ctx.graph(), TaskFilter::Ready)?
                    }
                };
                commands::claude(&ctx, &resolved_id)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use mont::commands;
    use mont::{MontContext, Task, TaskType};

    fn create_temp_context() -> (TempDir, MontContext) {
        let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
        let ctx = MontContext::load(temp_dir.path().to_path_buf()).expect("failed to load context");
        (temp_dir, ctx)
    }

    // Tests for mont task (content mode for non-interactive testing)
    #[test]
    fn test_task_creates_file_via_content() {
        let (temp_dir, ctx) = create_temp_context();

        let content = r#"---
id: test-task
title: Test task
---
Description here.
"#;

        let args = commands::task_cmd::TaskArgs {
            ids: vec![],
            task_type: None,
            resume: false,
            resume_path: None,
            content: Some(content.to_string()),
            patch: None,
            append: None,
            editor: None,
        };

        let result = commands::task(&ctx, args);
        assert!(result.is_ok());

        // Verify task was created
        assert!(temp_dir.path().join("test-task.md").exists());
    }

    #[test]
    fn test_task_edits_existing_via_content() {
        let (temp_dir, ctx) = create_temp_context();

        // Create a task directly with a known ID
        let task = Task {
            id: "edit-test".to_string(),
            new_id: None,
            title: Some("Original title".to_string()),
            description: String::new(),
            before: vec![],
            after: vec![],
            gates: vec![],
            task_type: TaskType::Task,
            status: None,
            deleted: false,
        };
        ctx.insert(task).unwrap();

        // Edit via content mode
        let content = r#"---
id: edit-test
title: Updated title
---
"#;

        let args = commands::task_cmd::TaskArgs {
            ids: vec!["edit-test".to_string()],
            task_type: None,
            resume: false,
            resume_path: None,
            content: Some(content.to_string()),
            patch: None,
            append: None,
            editor: None,
        };

        let result = commands::task(&ctx, args);
        assert!(result.is_ok());

        let file_content = std::fs::read_to_string(temp_dir.path().join("edit-test.md")).unwrap();
        assert!(file_content.contains("title: Updated title"));
    }

    #[test]
    fn test_task_rename_propagates_references() {
        let (temp_dir, ctx) = create_temp_context();

        // Create parent task directly with a known ID
        let parent = Task {
            id: "parent-task".to_string(),
            new_id: None,
            title: Some("Parent".to_string()),
            description: String::new(),
            before: vec![],
            after: vec![],
            gates: vec![],
            task_type: TaskType::Task,
            status: None,
            deleted: false,
        };
        ctx.insert(parent).unwrap();

        // Create child task with before reference
        let child = Task {
            id: "child-task".to_string(),
            new_id: None,
            title: Some("Child".to_string()),
            description: String::new(),
            before: vec!["parent-task".to_string()],
            after: vec![],
            gates: vec![],
            task_type: TaskType::Task,
            status: None,
            deleted: false,
        };
        ctx.insert(child).unwrap();

        // Rename parent via content mode using new_id field
        let content = r#"---
id: parent-task
new_id: renamed-parent
title: Parent
---
"#;

        let args = commands::task_cmd::TaskArgs {
            ids: vec!["parent-task".to_string()],
            task_type: None,
            resume: false,
            resume_path: None,
            content: Some(content.to_string()),
            patch: None,
            append: None,
            editor: None,
        };

        let result = commands::task(&ctx, args);
        assert!(result.is_ok());

        // Verify old file removed, new file exists
        assert!(!temp_dir.path().join("parent-task.md").exists());
        assert!(temp_dir.path().join("renamed-parent.md").exists());

        // Verify child's before reference was updated
        let child_content =
            std::fs::read_to_string(temp_dir.path().join("child-task.md")).unwrap();
        assert!(child_content.contains("- renamed-parent"));
        assert!(!child_content.contains("- parent-task"));
    }

    #[test]
    fn test_gate_type_content() {
        let (temp_dir, ctx) = create_temp_context();

        let content = r#"---
id: test-gate
title: Test gate
type: gate
---
Gate description.
"#;

        let args = commands::task_cmd::TaskArgs {
            ids: vec![],
            task_type: None,
            resume: false,
            resume_path: None,
            content: Some(content.to_string()),
            patch: None,
            append: None,
            editor: None,
        };

        let result = commands::task(&ctx, args);
        assert!(result.is_ok());

        let file_content = std::fs::read_to_string(temp_dir.path().join("test-gate.md")).unwrap();
        assert!(file_content.contains("title: Test gate"));
        assert!(file_content.contains("type: gate"));
    }
}
