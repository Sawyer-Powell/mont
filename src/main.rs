use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

use mont::commands;
use mont::commands::shared::{pick_in_progress_task, pick_task};
use mont::error_fmt::AppError;
use mont::{MontContext, TaskType};

#[derive(Parser)]
#[command(name = "mont")]
#[command(about = "Task management and agent coordination")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

/// Shared task field arguments used by both `new` and `edit` commands
#[derive(Args, Clone)]
struct TaskFields {
    /// Title for the task
    #[arg(long, short)]
    title: Option<String>,
    /// Description (markdown body)
    #[arg(long, short)]
    description: Option<String>,
    /// Before target task IDs (comma-separated or repeat flag)
    #[arg(long, short, value_delimiter = ',')]
    before: Vec<String>,
    /// After dependency task IDs (comma-separated or repeat flag)
    #[arg(long, value_delimiter = ',')]
    after: Vec<String>,
    /// Gate task IDs (comma-separated or repeat flag)
    #[arg(long, value_delimiter = ',')]
    gate: Vec<String>,
    /// Task type (feature, bug)
    #[arg(long, short = 'T', value_parser = parse_task_type)]
    r#type: Option<TaskType>,
    /// Open in editor after creation/editing (optionally specify editor name)
    #[arg(long, short)]
    editor: Option<Option<String>>,
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
    New {
        /// Unique task ID (generated if not provided)
        #[arg(long)]
        id: Option<String>,
        #[command(flatten)]
        fields: TaskFields,
        /// Resume editing a temp file from a previous failed gate
        #[arg(long, conflicts_with_all = ["id", "title", "description", "before", "after", "gate", "type"])]
        resume: Option<PathBuf>,
    },
    /// Edit an existing task
    Edit {
        /// Task ID to edit (original ID when using --resume). If not provided, opens interactive picker.
        id: Option<String>,
        /// New task ID (renames the task and updates references)
        #[arg(long)]
        new_id: Option<String>,
        #[command(flatten)]
        fields: TaskFields,
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
        Commands::New { id, fields, resume } => commands::new(
            &ctx,
            commands::new::NewArgs {
                id,
                title: fields.title,
                description: fields.description,
                before: fields.before,
                after: fields.after,
                gates: fields.gate,
                task_type: fields.r#type,
                editor: fields.editor,
                resume,
            },
        ),
        Commands::Edit {
            id,
            new_id,
            fields,
            resume,
        } => {
            let resolved_id = match id {
                Some(id) => id,
                None => pick_task(&ctx.graph())?,
            };
            commands::edit(
                &ctx,
                commands::edit::EditArgs {
                    id: resolved_id,
                    new_id,
                    title: fields.title,
                    description: fields.description,
                    before: fields.before,
                    after: fields.after,
                    gates: fields.gate,
                    task_type: fields.r#type,
                    editor: fields.editor,
                    resume,
                },
            )
        }
        Commands::Delete { id, force } => {
            let resolved_id = match id {
                Some(id) => id,
                None => pick_task(&ctx.graph())?,
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
                None => pick_task(&ctx.graph())?,
            };
            commands::distill(&ctx, &resolved_id)
        }
        Commands::Show { id, short, editor } => {
            let resolved_id = match id {
                Some(id) => id,
                None => pick_task(&ctx.graph())?,
            };
            commands::show(&ctx, &resolved_id, short, editor)
        }
        Commands::Unlock { id, passed, skipped } => {
            let resolved_id = match id {
                Some(id) => id,
                None => pick_in_progress_task(&ctx.graph())?,
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
                None => pick_in_progress_task(&ctx.graph())?,
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
                None => pick_task(&ctx.graph())?,
            };
            commands::start(&ctx, &resolved_id)
        }
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use mont::commands;
    use mont::MontContext;

    fn create_temp_context() -> (TempDir, MontContext) {
        let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
        let ctx = MontContext::load(temp_dir.path().to_path_buf()).expect("failed to load context");
        (temp_dir, ctx)
    }

    // Tests for mont new
    #[test]
    fn test_new_task_creates_file() {
        let (temp_dir, ctx) = create_temp_context();

        let args = commands::new::NewArgs {
            id: Some("test-task".to_string()),
            title: Some("Test task".to_string()),
            description: None,
            before: vec![],
            after: vec![],
            gates: vec![],
            task_type: None,
            editor: None,
            resume: None,
        };

        let result = commands::new(&ctx, args);
        assert!(result.is_ok());

        let file_path = temp_dir.path().join("test-task.md");
        assert!(file_path.exists());

        let content = std::fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("id: test-task"));
        assert!(content.contains("title: Test task"));
    }

    #[test]
    fn test_new_task_duplicate_id_fails() {
        let (_temp_dir, ctx) = create_temp_context();

        // Create first task
        let args1 = commands::new::NewArgs {
            id: Some("my-task".to_string()),
            title: Some("First task".to_string()),
            description: None,
            before: vec![],
            after: vec![],
            gates: vec![],
            task_type: None,
            editor: None,
            resume: None,
        };
        commands::new(&ctx, args1).unwrap();

        // Try to create second task with same ID
        let args2 = commands::new::NewArgs {
            id: Some("my-task".to_string()),
            title: Some("Second task".to_string()),
            description: None,
            before: vec![],
            after: vec![],
            gates: vec![],
            task_type: None,
            editor: None,
            resume: None,
        };
        let result = commands::new(&ctx, args2);
        assert!(result.is_err());
    }

    // Tests for mont edit
    #[test]
    fn test_edit_task_updates_title() {
        let (temp_dir, ctx) = create_temp_context();

        // Create a task
        let new_args = commands::new::NewArgs {
            id: Some("edit-test".to_string()),
            title: Some("Original title".to_string()),
            description: None,
            before: vec![],
            after: vec![],
            gates: vec![],
            task_type: None,
            editor: None,
            resume: None,
        };
        commands::new(&ctx, new_args).unwrap();

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

        // Create parent task
        let parent_args = commands::new::NewArgs {
            id: Some("parent-task".to_string()),
            title: Some("Parent".to_string()),
            description: None,
            before: vec![],
            after: vec![],
            gates: vec![],
            task_type: None,
            editor: None,
            resume: None,
        };
        commands::new(&ctx, parent_args).unwrap();

        // Create child task with before reference
        let child_args = commands::new::NewArgs {
            id: Some("child-task".to_string()),
            title: Some("Child".to_string()),
            description: None,
            before: vec!["parent-task".to_string()],
            after: vec![],
            gates: vec![],
            task_type: None,
            editor: None,
            resume: None,
        };
        commands::new(&ctx, child_args).unwrap();

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
