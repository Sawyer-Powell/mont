use std::fmt;
use std::io;

use owo_colors::OwoColorize;

use crate::context::{GraphReadError, LoadError, SettingsError};
use crate::{ParseError, TransactionError, ValidationError};
use crate::EditorError;

/// Application error with context for actionable error messages.
#[derive(Debug)]
pub enum AppError {
    /// Directory not found
    DirNotFound(String),
    /// IO error with context
    Io { context: String, source: io::Error },
    /// Parse error with file path context
    Parse { file_path: String, source: ParseError },
    /// Validation error with tasks directory context
    Validation { tasks_dir: String, source: ValidationError },
    /// Task not found
    TaskNotFound { task_id: String, tasks_dir: String },
    /// Editor resolution error
    Editor(EditorError),
    /// ID or title required
    IdOrTitleRequired,
    /// Failed to generate unique ID
    IdGenerationFailed { attempts: u32 },
    /// Temp file not found (for --resume)
    TempFileNotFound(String),
    /// Task ID already exists
    IdAlreadyExists(String),
    /// Temp file validation failed (shows resume command)
    TempValidationFailed {
        error: Box<AppError>,
        temp_path: String,
        editor_name: Option<String>,
    },
    /// No changes provided to edit command
    NoChangesProvided,
    /// Edit temp file validation failed (shows resume command)
    EditTempValidationFailed {
        error: Box<AppError>,
        original_id: String,
        temp_path: String,
        editor_name: Option<String>,
    },
    /// Task is not a jot (for distill command)
    NotAJot(String),
    /// Load error (loading tasks or config)
    Load(LoadError),
    /// fzf not found
    FzfNotFound,
    /// User cancelled picker
    PickerCancelled,
    /// No active tasks to pick from
    NoActiveTasks,
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::DirNotFound(dir) => write!(f, "{}", format_dir_not_found(dir)),
            AppError::Io { context, source } => {
                write!(f, "{}", format_cli_error(&format!("{}: {}", context, source)))
            }
            AppError::Parse { file_path, source } => {
                write!(f, "{}", format_parse_error(source, file_path))
            }
            AppError::Validation { tasks_dir, source } => {
                write!(f, "{}", format_validation_error(source, tasks_dir))
            }
            AppError::TaskNotFound { task_id, tasks_dir } => {
                write!(f, "{}", format_task_not_found(task_id, tasks_dir))
            }
            AppError::Editor(source) => {
                write!(f, "{}", format_editor_error(source))
            }
            AppError::IdOrTitleRequired => {
                write!(f, "{}", format_id_or_title_required())
            }
            AppError::IdGenerationFailed { attempts } => {
                write!(f, "{}", format_id_generation_failed(*attempts))
            }
            AppError::TempFileNotFound(path) => {
                write!(f, "{}", format_temp_file_not_found(path))
            }
            AppError::IdAlreadyExists(id) => {
                write!(f, "{}", format_id_already_exists(id))
            }
            AppError::TempValidationFailed { error, temp_path, editor_name } => {
                write!(f, "{}", format_temp_validation_failed(error, temp_path, editor_name.as_deref()))
            }
            AppError::NoChangesProvided => {
                write!(f, "{}", format_no_changes_provided())
            }
            AppError::EditTempValidationFailed { error, original_id, temp_path, editor_name } => {
                write!(f, "{}", format_edit_temp_validation_failed(error, original_id, temp_path, editor_name.as_deref()))
            }
            AppError::NotAJot(id) => {
                write!(f, "{}", format_not_a_jot(id))
            }
            AppError::Load(e) => {
                write!(f, "{}", format_load_error(e))
            }
            AppError::FzfNotFound => {
                write!(f, "{}", format_fzf_not_found())
            }
            AppError::PickerCancelled => {
                write!(f, "{}", format_picker_cancelled())
            }
            AppError::NoActiveTasks => {
                write!(f, "{}", format_no_active_tasks())
            }
        }
    }
}

impl std::error::Error for AppError {}

/// Extension trait to add file path context to parse results.
pub trait ParseResultExt<T> {
    fn with_path(self, path: &str) -> Result<T, AppError>;
}

impl<T> ParseResultExt<T> for Result<T, ParseError> {
    fn with_path(self, path: &str) -> Result<T, AppError> {
        self.map_err(|e| AppError::Parse {
            file_path: path.to_string(),
            source: e,
        })
    }
}

/// Extension trait to add tasks_dir context to validation results.
pub trait ValidationResultExt<T> {
    fn with_tasks_dir(self, tasks_dir: &str) -> Result<T, AppError>;
}

impl<T> ValidationResultExt<T> for Result<T, ValidationError> {
    fn with_tasks_dir(self, tasks_dir: &str) -> Result<T, AppError> {
        self.map_err(|e| AppError::Validation {
            tasks_dir: tasks_dir.to_string(),
            source: e,
        })
    }
}

/// Extension trait to add context to IO results.
pub trait IoResultExt<T> {
    fn with_context(self, context: &str) -> Result<T, AppError>;
}

impl<T> IoResultExt<T> for Result<T, io::Error> {
    fn with_context(self, context: &str) -> Result<T, AppError> {
        self.map_err(|e| AppError::Io {
            context: context.to_string(),
            source: e,
        })
    }
}

// ============================================================================
// Formatting functions (internal implementation)
// ============================================================================

fn format_parse_error(error: &ParseError, file_path: &str) -> String {
    let mut out = String::new();

    out.push_str(&format!("{}: ", "error".red().bold()));

    match error {
        ParseError::MissingFrontmatter => {
            out.push_str(&format!(
                "missing frontmatter delimiters in {}\n",
                file_path.cyan()
            ));
            out.push('\n');
            out.push_str(&format!("  {}\n", "Task files require YAML frontmatter between --- delimiters.".dimmed()));
            out.push('\n');
            out.push_str(&format!("  {}:\n", "To fix this".bold()));
            out.push_str(&format!("    Add frontmatter to the top of {}:\n", file_path.cyan()));
            out.push('\n');
            out.push_str(&format!("      {}\n", "---".dimmed()));
            out.push_str(&format!("      {}\n", "id: your-task-id".dimmed()));
            out.push_str(&format!("      {}\n", "title: Your task title".dimmed()));
            out.push_str(&format!("      {}\n", "---".dimmed()));
        }
        ParseError::InvalidYaml(yaml_err) => {
            out.push_str(&format!("invalid YAML in {}\n", file_path.cyan()));
            out.push('\n');
            out.push_str(&format!("  {}\n", yaml_err.to_string().dimmed()));
        }
        ParseError::GateWithAfter(task_id) => {
            out.push_str(&format!(
                "gate '{}' has after dependencies\n",
                task_id.yellow()
            ));
            out.push('\n');
            out.push_str(&format!("  {}\n", "Gates cannot have after dependencies because they are".dimmed()));
            out.push_str(&format!("  {}\n", "reusable validation criteria, not work items in the task graph.".dimmed()));
            out.push('\n');
            out.push_str(&format!("  {}:\n", "To fix this".bold()));
            out.push_str(&format!(
                "    1. Remove the {} field from {}\n",
                "after".cyan(),
                file_path.cyan()
            ));
            out.push_str(&format!(
                "    2. Or change {} to make this a regular task\n",
                "type: gate".cyan()
            ));
        }
        ParseError::GateMarkedComplete(task_id) => {
            out.push_str(&format!(
                "gate '{}' is marked complete\n",
                task_id.yellow()
            ));
            out.push('\n');
            out.push_str(&format!("  {}\n", "Gates are reusable and cannot be completed.".dimmed()));
            out.push_str(&format!("  {}\n", "They define validation criteria that can be run multiple times.".dimmed()));
            out.push('\n');
            out.push_str(&format!("  {}:\n", "To fix this".bold()));
            out.push_str(&format!(
                "    1. Remove {} from {}\n",
                "complete: true".cyan(),
                file_path.cyan()
            ));
            out.push_str(&format!(
                "    2. Or change {} to make this a regular task\n",
                "type: gate".cyan()
            ));
        }
    }

    out
}

fn format_validation_error(error: &ValidationError, tasks_dir: &str) -> String {
    let mut out = String::new();

    out.push_str(&format!("{}: ", "error".red().bold()));

    match error {
        ValidationError::InvalidBefore { task_id, before_id } => {
            out.push_str(&format!(
                "task '{}' references invalid before target '{}'\n",
                task_id.yellow(),
                before_id.yellow()
            ));
            out.push('\n');
            out.push_str(&format!(
                "  {}\n",
                format!("The before target '{}' does not exist in {}/", before_id, tasks_dir).dimmed()
            ));
            out.push('\n');
            out.push_str(&format!("  {}:\n", "To fix this".bold()));
            out.push_str(&format!(
                "    1. Create the missing task: {}/{}.md\n",
                tasks_dir.cyan(),
                before_id.cyan()
            ));
            out.push_str(&format!(
                "    2. Remove the {} field from {}/{}.md\n",
                "before".cyan(),
                tasks_dir.cyan(),
                task_id.cyan()
            ));
            out.push_str("    3. Change the before target to an existing task\n");
        }
        ValidationError::InvalidAfter {
            task_id,
            after_id,
        } => {
            out.push_str(&format!(
                "task '{}' references invalid after dependency '{}'\n",
                task_id.yellow(),
                after_id.yellow()
            ));
            out.push('\n');
            out.push_str(&format!(
                "  {}\n",
                format!("The after dependency '{}' does not exist in {}/", after_id, tasks_dir).dimmed()
            ));
            out.push('\n');
            out.push_str(&format!("  {}:\n", "To fix this".bold()));
            out.push_str(&format!(
                "    1. Create the missing task: {}/{}.md\n",
                tasks_dir.cyan(),
                after_id.cyan()
            ));
            out.push_str(&format!(
                "    2. Remove '{}' from after in {}/{}.md\n",
                after_id.cyan(),
                tasks_dir.cyan(),
                task_id.cyan()
            ));
            out.push_str("    3. Change the after dependency to an existing task\n");
        }
        ValidationError::AfterIsGate {
            task_id,
            after_id,
        } => {
            out.push_str(&format!(
                "task '{}' has gate '{}' as an after dependency\n",
                task_id.yellow(),
                after_id.yellow()
            ));
            out.push('\n');
            out.push_str(&format!("  {}\n", "Gates define validation criteria, not work dependencies.".dimmed()));
            out.push_str(&format!("  {}\n", "Use the 'validations' field instead of 'after'.".dimmed()));
            out.push('\n');
            out.push_str(&format!("  {}:\n", "To fix this".bold()));
            out.push_str(&format!(
                "    In {}/{}.md, move '{}' from after to validations:\n",
                tasks_dir.cyan(),
                task_id.cyan(),
                after_id.cyan()
            ));
            out.push('\n');
            out.push_str(&format!("      {}\n", "# Before:".dimmed()));
            out.push_str(&format!("      {}:\n", "after".dimmed()));
            out.push_str(&format!("      {}  - {}\n", "".dimmed(), after_id.dimmed()));
            out.push('\n');
            out.push_str(&format!("      {}\n", "# After:".dimmed()));
            out.push_str(&format!("      {}:\n", "validations".dimmed()));
            out.push_str(&format!("      {}  - {}\n", "".dimmed(), after_id.dimmed()));
        }
        ValidationError::ValidationNotFound {
            task_id,
            validation_id,
        } => {
            out.push_str(&format!(
                "task '{}' references non-existent validation '{}'\n",
                task_id.yellow(),
                validation_id.yellow()
            ));
            out.push('\n');
            out.push_str(&format!(
                "  {}\n",
                format!("The validation task '{}' does not exist in {}/", validation_id, tasks_dir).dimmed()
            ));
            out.push('\n');
            out.push_str(&format!("  {}:\n", "To fix this".bold()));
            out.push_str(&format!(
                "    1. Create the gate: {}/{}.md with {}\n",
                tasks_dir.cyan(),
                validation_id.cyan(),
                "type: gate".cyan()
            ));
            out.push_str(&format!(
                "    2. Remove '{}' from validations in {}/{}.md\n",
                validation_id.cyan(),
                tasks_dir.cyan(),
                task_id.cyan()
            ));
            out.push_str("    3. Change the validation to reference an existing gate\n");
        }
        ValidationError::InvalidValidation {
            task_id,
            validation_id,
        } => {
            out.push_str(&format!(
                "task '{}' references validation '{}' which is not a gate\n",
                task_id.yellow(),
                validation_id.yellow()
            ));
            out.push('\n');
            out.push_str(&format!(
                "  {}\n",
                format!("The task '{}' exists but is not a gate.", validation_id).dimmed()
            ));
            out.push('\n');
            out.push_str(&format!("  {}:\n", "To fix this".bold()));
            out.push_str(&format!(
                "    1. Change to {} in {}/{}.md\n",
                "type: gate".cyan(),
                tasks_dir.cyan(),
                validation_id.cyan()
            ));
            out.push_str(&format!(
                "    2. Remove '{}' from validations in {}/{}.md\n",
                validation_id.cyan(),
                tasks_dir.cyan(),
                task_id.cyan()
            ));
            out.push_str("    3. Change the validation to reference an existing gate\n");
        }
        ValidationError::ValidationNotRootGate {
            task_id,
            validation_id,
        } => {
            out.push_str(&format!(
                "task '{}' references gate '{}' which has a before target\n",
                task_id.yellow(),
                validation_id.yellow()
            ));
            out.push('\n');
            out.push_str(&format!("  {}\n", "Gates used in the 'validations' field must be root gates".dimmed()));
            out.push_str(&format!("  {}\n", "(they cannot have a before target).".dimmed()));
            out.push('\n');
            out.push_str(&format!("  {}:\n", "To fix this".bold()));
            out.push_str(&format!(
                "    1. Remove the {} field from {}/{}.md\n",
                "before".cyan(),
                tasks_dir.cyan(),
                validation_id.cyan()
            ));
            out.push_str(&format!(
                "    2. Use a different root gate in {}/{}.md\n",
                tasks_dir.cyan(),
                task_id.cyan()
            ));
        }
        ValidationError::CycleDetected => {
            out.push_str("cycle detected in task graph\n");
            out.push('\n');
            out.push_str(&format!("  {}\n", "The task graph contains a circular dependency.".dimmed()));
            out.push_str(&format!("  {}\n", "Tasks cannot depend on themselves directly or indirectly.".dimmed()));
            out.push('\n');
            out.push_str(&format!("  {}:\n", "To fix this".bold()));
            out.push_str("    1. Review before and after relationships\n");
            out.push_str("    2. Look for chains like: A → B → C → A\n");
            out.push_str(&format!(
                "    3. Run {} to see task relationships\n",
                "mont list".cyan()
            ));
        }
        ValidationError::DuplicateTaskId(task_id) => {
            out.push_str(&format!("duplicate task id '{}'\n", task_id.yellow()));
            out.push('\n');
            out.push_str(&format!(
                "  {}\n",
                format!("Multiple task files define id: '{}'", task_id).dimmed()
            ));
            out.push('\n');
            out.push_str(&format!("  {}:\n", "To fix this".bold()));
            out.push_str(&format!(
                "    1. Search for duplicates: grep -r 'id: {}' {}/\n",
                task_id.cyan(),
                tasks_dir.cyan()
            ));
            out.push_str("    2. Rename one of the tasks to have a unique id\n");
            out.push_str("    3. Delete the duplicate file if unintended\n");
        }
    }

    out
}

fn format_cli_error(message: &str) -> String {
    format!("{}: {}\n", "error".red().bold(), message)
}

fn format_task_not_found(task_id: &str, tasks_dir: &str) -> String {
    let mut out = String::new();

    out.push_str(&format!("{}: ", "error".red().bold()));
    out.push_str(&format!("task '{}' not found\n", task_id.yellow()));
    out.push('\n');
    out.push_str(&format!(
        "  {}\n",
        format!("No task with id '{}' exists in {}/", task_id, tasks_dir).dimmed()
    ));
    out.push('\n');
    out.push_str(&format!("  {}:\n", "To fix this".bold()));
    out.push_str("    1. Check the spelling of the task id\n");
    out.push_str(&format!(
        "    2. List available tasks: {}\n",
        "mont list".cyan()
    ));
    out.push_str(&format!(
        "    3. Create the task: {}/{}.md\n",
        tasks_dir.cyan(),
        task_id.cyan()
    ));

    out
}

fn format_dir_not_found(dir: &str) -> String {
    let mut out = String::new();

    out.push_str(&format!("{}: ", "error".red().bold()));
    out.push_str(&format!("tasks directory not found: {}\n", dir.yellow()));
    out.push('\n');
    out.push_str(&format!("  {}\n", "The specified tasks directory does not exist.".dimmed()));
    out.push('\n');
    out.push_str(&format!("  {}:\n", "To fix this".bold()));
    out.push_str(&format!(
        "    1. Create the directory: {}\n",
        format!("mkdir -p {}", dir).cyan()
    ));
    out.push_str(&format!(
        "    2. Use a different directory: {}\n",
        "mont list -d /path/to/tasks".cyan()
    ));
    out.push_str("    3. Run from a directory that contains a .tasks folder\n");

    out
}

fn format_editor_error(error: &EditorError) -> String {
    let mut out = String::new();

    out.push_str(&format!("{}: ", "error".red().bold()));

    match error {
        EditorError::NotFound(msg) => {
            out.push_str(&format!("{}\n", msg));
            out.push('\n');
            out.push_str(&format!("  {}\n", "No text editor could be resolved.".dimmed()));
            out.push('\n');
            out.push_str(&format!("  {}:\n", "To fix this".bold()));
            out.push_str(&format!(
                "    1. Set the {} environment variable: {}\n",
                "$EDITOR".cyan(),
                "export EDITOR=vim".cyan()
            ));
            out.push_str("    2. Pass an editor explicitly via command-line argument\n");
        }
    }

    out
}

fn format_id_or_title_required() -> String {
    let mut out = String::new();

    out.push_str(&format!("{}: ", "error".red().bold()));
    out.push_str("either --id or --title is required\n");
    out.push('\n');
    out.push_str(&format!("  {}\n", "A task needs an identifier to be created.".dimmed()));
    out.push('\n');
    out.push_str(&format!("  {}:\n", "To fix this".bold()));
    out.push_str(&format!(
        "    1. Provide an id: {}\n",
        "mont new --id my-task".cyan()
    ));
    out.push_str(&format!(
        "    2. Provide a title: {}\n",
        "mont new --title \"My task title\"".cyan()
    ));

    out
}

fn format_id_generation_failed(attempts: u32) -> String {
    let mut out = String::new();

    out.push_str(&format!("{}: ", "error".red().bold()));
    out.push_str(&format!("failed to generate unique id after {} attempts\n", attempts));
    out.push('\n');
    out.push_str(&format!("  {}\n", "All generated IDs collided with existing tasks.".dimmed()));
    out.push('\n');
    out.push_str(&format!("  {}:\n", "To fix this".bold()));
    out.push_str(&format!(
        "    Provide an explicit id: {}\n",
        "mont new --id my-unique-id".cyan()
    ));

    out
}

fn format_temp_file_not_found(path: &str) -> String {
    let mut out = String::new();

    out.push_str(&format!("{}: ", "error".red().bold()));
    out.push_str(&format!("temp file not found: {}\n", path.yellow()));
    out.push('\n');
    out.push_str(&format!("  {}\n", "The specified temp file does not exist or was already cleaned up.".dimmed()));
    out.push('\n');
    out.push_str(&format!("  {}:\n", "To fix this".bold()));
    out.push_str(&format!(
        "    Create a new task instead: {}\n",
        "mont new --editor".cyan()
    ));

    out
}

fn format_id_already_exists(id: &str) -> String {
    let mut out = String::new();

    out.push_str(&format!("{}: ", "error".red().bold()));
    out.push_str(&format!("task id '{}' already exists\n", id.yellow()));
    out.push('\n');
    out.push_str(&format!("  {}\n", "A task with this ID is already in the task graph.".dimmed()));
    out.push('\n');
    out.push_str(&format!("  {}:\n", "To fix this".bold()));
    out.push_str(&format!(
        "    Change the {} field in your task file to a unique value.\n",
        "id".cyan()
    ));

    out
}

fn format_temp_validation_failed(error: &AppError, temp_path: &str, editor_name: Option<&str>) -> String {
    let mut out = String::new();

    // First, display the underlying error
    out.push_str(&error.to_string());
    out.push('\n');

    // Then show how to resume
    out.push_str(&format!("  {}\n", "Your task file has been saved to:".dimmed()));
    out.push_str(&format!("    {}\n", temp_path.cyan()));
    out.push('\n');
    out.push_str(&format!("  {}:\n", "To fix and retry".bold()));

    let resume_cmd = match editor_name {
        Some(name) => format!("mont new --resume {} --editor {}", temp_path, name),
        None => format!("mont new --resume {}", temp_path),
    };
    out.push_str(&format!("    {}\n", resume_cmd.cyan()));

    out
}

fn format_no_changes_provided() -> String {
    let mut out = String::new();

    out.push_str(&format!("{}: ", "error".red().bold()));
    out.push_str("no changes provided\n");
    out.push('\n');
    out.push_str(&format!("  {}\n", "The edit command requires at least one change.".dimmed()));
    out.push('\n');
    out.push_str(&format!("  {}:\n", "To fix this".bold()));
    out.push_str(&format!(
        "    1. Provide field flags: {}\n",
        "mont edit <task-id> --title \"New title\"".cyan()
    ));
    out.push_str(&format!(
        "    2. Use editor mode: {}\n",
        "mont edit <task-id> --editor".cyan()
    ));
    out.push_str(&format!(
        "    3. Rename the task: {}\n",
        "mont edit <task-id> --new-id new-id".cyan()
    ));

    out
}

fn format_edit_temp_validation_failed(error: &AppError, original_id: &str, temp_path: &str, editor_name: Option<&str>) -> String {
    let mut out = String::new();

    // First, display the underlying error
    out.push_str(&error.to_string());
    out.push('\n');

    // Then show how to resume editing
    out.push_str(&format!("  {}\n", "Your task file has been saved to:".dimmed()));
    out.push_str(&format!("    {}\n", temp_path.cyan()));
    out.push('\n');
    out.push_str(&format!("  {}:\n", "To fix and retry".bold()));

    let resume_cmd = match editor_name {
        Some(name) => format!("mont edit {} --resume {} --editor {}", original_id, temp_path, name),
        None => format!("mont edit {} --resume {}", original_id, temp_path),
    };
    out.push_str(&format!("    {}\n", resume_cmd.cyan()));

    out
}

fn format_not_a_jot(id: &str) -> String {
    let mut out = String::new();

    out.push_str(&format!("{}: ", "error".red().bold()));
    out.push_str(&format!("task '{}' is not a jot\n", id.yellow()));
    out.push('\n');
    out.push_str(&format!("  {}\n", "The distill command can only be used on jot-type tasks.".dimmed()));
    out.push('\n');
    out.push_str(&format!("  {}:\n", "To fix this".bold()));
    out.push_str(&format!(
        "    1. Use {} to create a jot first\n",
        "mont jot".cyan()
    ));
    out.push_str("    2. Check that the task has 'type: jot' in its frontmatter\n");

    out
}

fn format_load_error(error: &LoadError) -> String {
    match error {
        LoadError::Graph(e) => format_graph_read_error(e),
        LoadError::Settings(e) => format_settings_error(e),
    }
}

fn format_graph_read_error(error: &GraphReadError) -> String {
    let mut out = String::new();

    out.push_str(&format!("{}: ", "error".red().bold()));
    out.push_str("failed to load task graph\n");

    for (path, io_err) in &error.io_errors {
        out.push_str(&format!(
            "  {} {}: {}\n",
            "•".red(),
            path.display().to_string().cyan(),
            io_err
        ));
    }

    for (path, parse_err) in &error.parse_errors {
        out.push_str(&format!(
            "  {} {}: {:?}\n",
            "•".red(),
            path.display().to_string().cyan(),
            parse_err
        ));
    }

    for val_err in &error.validation_errors {
        out.push_str(&format!("  {} {:?}\n", "•".red(), val_err));
    }

    out
}

fn format_settings_error(error: &SettingsError) -> String {
    let mut out = String::new();

    out.push_str(&format!("{}: ", "error".red().bold()));
    out.push_str("failed to load config.yml\n");
    out.push_str(&format!("  {} {}\n", "•".red(), error));

    out
}

fn format_fzf_not_found() -> String {
    let mut out = String::new();

    out.push_str(&format!("{}: ", "error".red().bold()));
    out.push_str("fzf not found\n");
    out.push('\n');
    out.push_str(&format!("  {}\n", "The interactive picker requires fzf to be installed.".dimmed()));
    out.push('\n');
    out.push_str(&format!("  {}:\n", "To fix this".bold()));
    out.push_str(&format!(
        "    1. Install fzf: {}\n",
        "brew install fzf".cyan()
    ));
    out.push_str("    2. Or provide a task ID directly as an argument\n");

    out
}

fn format_picker_cancelled() -> String {
    "Cancelled\n".to_string()
}

fn format_no_active_tasks() -> String {
    let mut out = String::new();

    out.push_str(&format!("{}: ", "error".red().bold()));
    out.push_str("no active tasks\n");
    out.push('\n');
    out.push_str(&format!("  {}\n", "There are no non-completed tasks to pick from.".dimmed()));
    out.push('\n');
    out.push_str(&format!("  {}:\n", "To fix this".bold()));
    out.push_str(&format!(
        "    Create a new task: {}\n",
        "mont new --title \"My task\"".cyan()
    ));

    out
}

impl From<EditorError> for AppError {
    fn from(e: EditorError) -> Self {
        AppError::Editor(e)
    }
}

impl From<LoadError> for AppError {
    fn from(e: LoadError) -> Self {
        AppError::Load(e)
    }
}

impl From<TransactionError> for AppError {
    fn from(e: TransactionError) -> Self {
        match e {
            TransactionError::Validation(v) => AppError::Validation {
                tasks_dir: ".tasks".to_string(),
                source: v,
            },
            TransactionError::Io(io) => AppError::Io {
                context: "transaction failed".to_string(),
                source: io,
            },
            TransactionError::TaskNotFound(id) => AppError::TaskNotFound {
                task_id: id,
                tasks_dir: ".tasks".to_string(),
            },
            TransactionError::TaskAlreadyExists(id) => AppError::IdAlreadyExists(id),
            TransactionError::IdGenerationFailed(attempts) => {
                AppError::IdGenerationFailed { attempts }
            }
            TransactionError::Conflict { expected, actual } => AppError::Io {
                context: format!(
                    "concurrent modification: expected version {}, found {}",
                    expected, actual
                ),
                source: io::Error::other("version conflict"),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strip_ansi(s: &str) -> String {
        let re = regex::Regex::new(r"\x1b\[[0-9;]*m").unwrap();
        re.replace_all(s, "").to_string()
    }

    #[test]
    fn test_format_missing_frontmatter() {
        let err = AppError::Parse {
            file_path: ".tasks/my-task.md".to_string(),
            source: ParseError::MissingFrontmatter,
        };
        let output = err.to_string();
        let stripped = strip_ansi(&output);

        assert!(stripped.contains("error:"));
        assert!(stripped.contains("missing frontmatter"));
        assert!(stripped.contains(".tasks/my-task.md"));
        assert!(stripped.contains("To fix this"));
        assert!(stripped.contains("---"));
    }

    #[test]
    fn test_format_invalid_before() {
        let err = AppError::Validation {
            tasks_dir: ".tasks".to_string(),
            source: ValidationError::InvalidBefore {
                task_id: "setup-db".to_string(),
                before_id: "database".to_string(),
            },
        };
        let output = err.to_string();
        let stripped = strip_ansi(&output);

        assert!(stripped.contains("error:"));
        assert!(stripped.contains("setup-db"));
        assert!(stripped.contains("database"));
        assert!(stripped.contains("does not exist"));
        assert!(stripped.contains("To fix this"));
        assert!(stripped.contains("Create the missing task"));
    }

    #[test]
    fn test_format_cycle_detected() {
        let err = AppError::Validation {
            tasks_dir: ".tasks".to_string(),
            source: ValidationError::CycleDetected,
        };
        let output = err.to_string();
        let stripped = strip_ansi(&output);

        assert!(stripped.contains("error:"));
        assert!(stripped.contains("cycle detected"));
        assert!(stripped.contains("circular dependency"));
        assert!(stripped.contains("To fix this"));
    }

    #[test]
    fn test_format_duplicate_task_id() {
        let err = AppError::Validation {
            tasks_dir: ".tasks".to_string(),
            source: ValidationError::DuplicateTaskId("my-task".to_string()),
        };
        let output = err.to_string();
        let stripped = strip_ansi(&output);

        assert!(stripped.contains("error:"));
        assert!(stripped.contains("duplicate task id"));
        assert!(stripped.contains("my-task"));
        assert!(stripped.contains("grep"));
        assert!(stripped.contains("To fix this"));
    }

    #[test]
    fn test_format_task_not_found() {
        let err = AppError::TaskNotFound {
            task_id: "missing-task".to_string(),
            tasks_dir: ".tasks".to_string(),
        };
        let output = err.to_string();
        let stripped = strip_ansi(&output);

        assert!(stripped.contains("error:"));
        assert!(stripped.contains("missing-task"));
        assert!(stripped.contains("not found"));
        assert!(stripped.contains("mont list"));
        assert!(stripped.contains("To fix this"));
    }

    #[test]
    fn test_format_dir_not_found() {
        let err = AppError::DirNotFound("/nonexistent/path".to_string());
        let output = err.to_string();
        let stripped = strip_ansi(&output);

        assert!(stripped.contains("error:"));
        assert!(stripped.contains("/nonexistent/path"));
        assert!(stripped.contains("does not exist"));
        assert!(stripped.contains("mkdir"));
        assert!(stripped.contains("To fix this"));
    }

    #[test]
    fn test_extension_trait_parse() {
        let result: Result<(), ParseError> = Err(ParseError::MissingFrontmatter);
        let app_result = result.with_path("test.md");
        assert!(app_result.is_err());

        let err = app_result.unwrap_err();
        assert!(matches!(err, AppError::Parse { file_path, .. } if file_path == "test.md"));
    }

    #[test]
    fn test_extension_trait_validation() {
        let result: Result<(), ValidationError> = Err(ValidationError::CycleDetected);
        let app_result = result.with_tasks_dir(".tasks");
        assert!(app_result.is_err());

        let err = app_result.unwrap_err();
        assert!(matches!(err, AppError::Validation { tasks_dir, .. } if tasks_dir == ".tasks"));
    }
}
