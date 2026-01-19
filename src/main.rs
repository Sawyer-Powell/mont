use clap::{Parser, Subcommand};
use std::path::PathBuf;

use mont::commands;
use mont::commands::shared::{pick_task, TaskFilter};
use mont::error_fmt::AppError;
use mont::{MontContext, TaskType};

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
    /// Create a new task
    Task {
        /// Title for the task (required unless using --resume)
        #[arg(required_unless_present = "resume")]
        title: Option<String>,
        /// Description (markdown body)
        #[arg(long, short)]
        description: Option<String>,
        /// Before target task IDs (comma-separated)
        #[arg(long, short, value_delimiter = ',')]
        before: Vec<String>,
        /// After dependency task IDs (comma-separated)
        #[arg(long, value_delimiter = ',')]
        after: Vec<String>,
        /// Validation task IDs (comma-separated)
        #[arg(long, value_delimiter = ',')]
        gate: Vec<String>,
        /// Open in editor after creation
        #[arg(long, short)]
        editor: Option<Option<String>>,
        /// Resume editing a temp file from a previous failed gate
        #[arg(long, conflicts_with_all = ["title", "description", "before", "after", "gate"])]
        resume: Option<PathBuf>,
    },
    /// Create a new gate
    Gate {
        /// Title for the gate (required unless using --resume)
        #[arg(required_unless_present = "resume")]
        title: Option<String>,
        /// Description (markdown body)
        #[arg(long, short)]
        description: Option<String>,
        /// Before target task IDs (comma-separated)
        #[arg(long, short, value_delimiter = ',')]
        before: Vec<String>,
        /// After dependency task IDs (comma-separated)
        #[arg(long, value_delimiter = ',')]
        after: Vec<String>,
        /// Validation task IDs (comma-separated)
        #[arg(long, value_delimiter = ',')]
        gate: Vec<String>,
        /// Open in editor after creation
        #[arg(long, short)]
        editor: Option<Option<String>>,
        /// Resume editing a temp file from a previous failed gate
        #[arg(long, conflicts_with_all = ["title", "description", "before", "after", "gate"])]
        resume: Option<PathBuf>,
    },
    /// Edit an existing task
    Edit {
        /// Task ID to edit (original ID when using --resume). If not provided, opens interactive picker.
        id: Option<String>,
        /// New task ID (renames the task and updates references)
        #[arg(long)]
        new_id: Option<String>,
        /// Title for the task
        #[arg(long, short)]
        title: Option<String>,
        /// Description (markdown body)
        #[arg(long, short)]
        description: Option<String>,
        /// Before target task IDs (comma-separated)
        #[arg(long, short, value_delimiter = ',')]
        before: Vec<String>,
        /// After dependency task IDs (comma-separated)
        #[arg(long, value_delimiter = ',')]
        after: Vec<String>,
        /// Gate task IDs (comma-separated)
        #[arg(long, value_delimiter = ',')]
        gate: Vec<String>,
        /// Task type (task, jot, gate)
        #[arg(long, short = 'T', value_parser = parse_task_type)]
        r#type: Option<TaskType>,
        /// Open in editor (optionally specify editor name)
        #[arg(long, short)]
        editor: Option<Option<String>>,
        /// Resume editing a temp file from a previous failed gate
        #[arg(long, conflicts_with_all = ["new_id", "title", "description", "before", "after", "gate", "type"])]
        resume: Option<PathBuf>,
    },
    /// Delete a task and remove all references to it
    Delete {
        /// Task ID to delete. If not provided, opens interactive picker.
        id: Option<String>,
        /// Force deletion without confirmation prompt
        #[arg(long, short)]
        force: bool,
    },
    /// Quickly jot down an idea (creates a new task with type 'jot')
    Jot {
        /// Title for the jot (required unless using --resume)
        #[arg(required_unless_present = "resume")]
        title: Option<String>,
        /// Description (markdown body)
        #[arg(long, short)]
        description: Option<String>,
        /// Before target task IDs (comma-separated)
        #[arg(long, short, value_delimiter = ',')]
        before: Vec<String>,
        /// After dependency task IDs (comma-separated)
        #[arg(long, value_delimiter = ',')]
        after: Vec<String>,
        /// Validation task IDs (comma-separated)
        #[arg(long, value_delimiter = ',')]
        gate: Vec<String>,
        /// Open in editor after creation
        #[arg(long, short)]
        editor: Option<Option<String>>,
        /// Resume editing a temp file from a previous failed gate
        #[arg(long, conflicts_with_all = ["title", "description", "before", "after", "gate"])]
        resume: Option<PathBuf>,
    },
    /// Distill a jot into one or more proper tasks
    Distill {
        /// Jot task ID to distill. If not provided, opens interactive picker.
        id: Option<String>,
    },
    /// Show details for a single task
    Show {
        /// Task ID to show. If not provided, opens interactive picker.
        id: Option<String>,
        /// Show shortened version (omit description)
        #[arg(long, short)]
        short: bool,
        /// Open in editor (optionally specify editor command)
        #[arg(long, short)]
        editor: Option<Option<String>>,
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
    let ctx = MontContext::load(PathBuf::from(".tasks"))?;

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
            title,
            description,
            before,
            after,
            gate,
            editor,
            resume,
        } => commands::task(
            &ctx,
            commands::new::CreateArgs {
                title,
                description,
                before,
                after,
                gates: gate,
                editor,
                resume,
            },
        ),
        Commands::Gate {
            title,
            description,
            before,
            after,
            gate,
            editor,
            resume,
        } => commands::gate(
            &ctx,
            commands::new::CreateArgs {
                title,
                description,
                before,
                after,
                gates: gate,
                editor,
                resume,
            },
        ),
        Commands::Edit {
            id,
            new_id,
            title,
            description,
            before,
            after,
            gate,
            r#type,
            editor,
            resume,
        } => {
            let resolved_id = match id {
                Some(id) => id,
                None => pick_task(&ctx.graph(), TaskFilter::Active)?,
            };
            commands::edit(
                &ctx,
                commands::edit::EditArgs {
                    id: resolved_id,
                    new_id,
                    title,
                    description,
                    before,
                    after,
                    gates: gate,
                    task_type: r#type,
                    editor,
                    resume,
                },
            )
        }
        Commands::Delete { id, force } => {
            let resolved_id = match id {
                Some(id) => id,
                None => pick_task(&ctx.graph(), TaskFilter::Active)?,
            };
            commands::delete(&ctx, &resolved_id, force)
        }
        Commands::Jot {
            title,
            description,
            before,
            after,
            gate,
            editor,
            resume,
        } => commands::jot(
            &ctx,
            commands::jot::JotArgs {
                title,
                description,
                before,
                after,
                gates: gate,
                editor,
                resume,
            },
        ),
        Commands::Distill { id } => {
            let resolved_id = match id {
                Some(id) => id,
                None => pick_task(&ctx.graph(), TaskFilter::Active)?,
            };
            commands::distill(&ctx, &resolved_id)
        }
        Commands::Show { id, short, editor } => {
            let resolved_id = match id {
                Some(id) => id,
                None => pick_task(&ctx.graph(), TaskFilter::All)?,
            };
            commands::show(&ctx, &resolved_id, short, editor)
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
                        pick_task(&ctx.graph(), TaskFilter::Active)?
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

    // Tests for mont task
    #[test]
    fn test_task_creates_file() {
        let (temp_dir, ctx) = create_temp_context();

        let args = commands::new::CreateArgs {
            title: Some("Test task".to_string()),
            description: None,
            before: vec![],
            after: vec![],
            gates: vec![],
            editor: None,
            resume: None,
        };

        let result = commands::task(&ctx, args);
        assert!(result.is_ok());

        // Find the created file (ID is auto-generated)
        let files: Vec<_> = std::fs::read_dir(temp_dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "md"))
            .collect();
        assert_eq!(files.len(), 1);

        let content = std::fs::read_to_string(files[0].path()).unwrap();
        assert!(content.contains("title: Test task"));
        // type: task is not written since it's the default
        assert!(!content.contains("type:"));
    }

    #[test]
    fn test_gate_creates_file_with_gate_type() {
        let (temp_dir, ctx) = create_temp_context();

        let args = commands::new::CreateArgs {
            title: Some("Test gate".to_string()),
            description: None,
            before: vec![],
            after: vec![],
            gates: vec![],
            editor: None,
            resume: None,
        };

        let result = commands::gate(&ctx, args);
        assert!(result.is_ok());

        // Find the created file (ID is auto-generated)
        let files: Vec<_> = std::fs::read_dir(temp_dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "md"))
            .collect();
        assert_eq!(files.len(), 1);

        let content = std::fs::read_to_string(files[0].path()).unwrap();
        assert!(content.contains("title: Test gate"));
        assert!(content.contains("type: gate"));
    }

    // Tests for mont edit
    #[test]
    fn test_edit_task_updates_title() {
        let (temp_dir, ctx) = create_temp_context();

        // Create a task directly with a known ID
        let task = Task {
            id: "edit-test".to_string(),
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

        // Edit the task
        let edit_args = commands::edit::EditArgs {
            id: "edit-test".to_string(),
            new_id: None,
            title: Some("Updated title".to_string()),
            description: None,
            before: vec![],
            after: vec![],
            gates: vec![],
            task_type: None,
            editor: None,
            resume: None,
        };
        let result = commands::edit(&ctx, edit_args);
        assert!(result.is_ok());

        let content = std::fs::read_to_string(temp_dir.path().join("edit-test.md")).unwrap();
        assert!(content.contains("title: Updated title"));
    }

    #[test]
    fn test_edit_task_rename_propagates_references() {
        let (temp_dir, ctx) = create_temp_context();

        // Create parent task directly with a known ID
        let parent = Task {
            id: "parent-task".to_string(),
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

        // Rename parent
        let edit_args = commands::edit::EditArgs {
            id: "parent-task".to_string(),
            new_id: Some("renamed-parent".to_string()),
            title: None,
            description: None,
            before: vec![],
            after: vec![],
            gates: vec![],
            task_type: None,
            editor: None,
            resume: None,
        };
        let result = commands::edit(&ctx, edit_args);
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
}
