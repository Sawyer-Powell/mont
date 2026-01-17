use clap::{Parser, Subcommand};
use owo_colors::OwoColorize;
use std::path::PathBuf;

use mont::error_fmt::{AppError, IoResultExt, ParseResultExt, ValidationResultExt};
use mont::task::TaskType;
use mont::{graph, render, task, validations};

#[derive(Parser)]
#[command(name = "mont")]
#[command(about = "Task management and agent coordination")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
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
        /// Title for the task
        #[arg(long, short)]
        title: Option<String>,
        /// Description (markdown body)
        #[arg(long, short)]
        description: Option<String>,
        /// Parent task ID
        #[arg(long, short)]
        parent: Option<String>,
        /// Precondition task IDs (comma-separated or repeat flag)
        #[arg(long, value_delimiter = ',')]
        precondition: Vec<String>,
        /// Validation task IDs (comma-separated or repeat flag)
        #[arg(long, value_delimiter = ',')]
        validation: Vec<String>,
        /// Task type (feature, bug, epic)
        #[arg(long, short = 'T', value_parser = parse_task_type)]
        r#type: Option<TaskType>,
        /// Open in editor after creation (optionally specify editor name)
        #[arg(long, short)]
        editor: Option<Option<String>>,
        /// Resume editing a temp file from a previous failed validation
        #[arg(long, conflicts_with_all = ["id", "title", "description", "parent", "precondition", "validation", "type"])]
        resume: Option<PathBuf>,
    },
}

const TASKS_DIR: &str = ".tasks";

fn parse_task_type(s: &str) -> Result<TaskType, String> {
    match s.to_lowercase().as_str() {
        "feature" => Ok(TaskType::Feature),
        "bug" => Ok(TaskType::Bug),
        "epic" => Ok(TaskType::Epic),
        _ => Err(format!(
            "invalid task type '{}', must be one of: feature, bug, epic",
            s
        )),
    }
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::List { show_completed } => {
            if let Err(e) = list_tasks(show_completed) {
                eprint!("{}", e);
                std::process::exit(1);
            }
        }
        Commands::Ready => {
            if let Err(e) = ready_tasks() {
                eprint!("{}", e);
                std::process::exit(1);
            }
        }
        Commands::Check { id } => {
            if let Err(e) = check_tasks(id.as_deref()) {
                eprint!("{}", e);
                std::process::exit(1);
            }
        }
        Commands::New {
            id,
            title,
            description,
            parent,
            precondition,
            validation,
            r#type,
            editor,
            resume,
        } => {
            if let Err(e) = new_task(
                id,
                title,
                description,
                parent,
                precondition,
                validation,
                r#type,
                editor,
                resume,
            ) {
                eprint!("{}", e);
                std::process::exit(1);
            }
        }
    }
}

fn list_tasks(show_completed: bool) -> Result<(), AppError> {
    let dir = PathBuf::from(TASKS_DIR);

    if !dir.exists() {
        return Err(AppError::DirNotFound(TASKS_DIR.to_string()));
    }

    let mut paths: Vec<_> = std::fs::read_dir(&dir)
        .with_context(&format!("failed to read directory {}", TASKS_DIR))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "md"))
        .collect();
    paths.sort();

    let mut tasks = Vec::new();
    for path in &paths {
        let path_str = path.display().to_string();
        let content = std::fs::read_to_string(path).with_context(&format!("failed to read {}", path_str))?;
        let parsed = task::parse(&content).with_path(&path_str)?;
        tasks.push(parsed);
    }

    if tasks.is_empty() {
        println!("No tasks found in {}", TASKS_DIR);
        return Ok(());
    }

    let validated = graph::form_graph(tasks).with_tasks_dir(TASKS_DIR)?;

    let output = render::render_task_graph(&validated, show_completed);
    print!("{}", output);

    Ok(())
}

fn ready_tasks() -> Result<(), AppError> {
    let dir = PathBuf::from(TASKS_DIR);

    if !dir.exists() {
        return Err(AppError::DirNotFound(TASKS_DIR.to_string()));
    }

    let mut paths: Vec<_> = std::fs::read_dir(&dir)
        .with_context(&format!("failed to read directory {}", TASKS_DIR))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "md"))
        .collect();
    paths.sort();

    let mut tasks = Vec::new();
    for path in &paths {
        let path_str = path.display().to_string();
        let content = std::fs::read_to_string(path).with_context(&format!("failed to read {}", path_str))?;
        let parsed = task::parse(&content).with_path(&path_str)?;
        tasks.push(parsed);
    }

    if tasks.is_empty() {
        println!("No ready tasks");
        return Ok(());
    }

    let validated = graph::form_graph(tasks).with_tasks_dir(TASKS_DIR)?;
    let mut ready: Vec<_> = graph::available_tasks(&validated);
    ready.sort_by(|a, b| a.id.cmp(&b.id));

    if ready.is_empty() {
        println!("No ready tasks");
        return Ok(());
    }

    let max_id_len = ready.iter().map(|t| t.id.len()).max().unwrap_or(0);
    let max_title_len = ready
        .iter()
        .map(|t| render::truncate_title(t.title.as_deref().unwrap_or("")).len())
        .max()
        .unwrap_or(0);

    for task in ready {
        let title = render::truncate_title(task.title.as_deref().unwrap_or(""));
        let type_tag = match task.task_type {
            TaskType::Bug => format!("{}", "[bug]".red().bold()),
            TaskType::Epic => format!("{}", "[epic]".cyan().bold()),
            TaskType::Feature => format!("{}", "[feature]".bright_green().bold()),
        };
        println!(
            "{}  {:max_title_len$}  {}",
            format!("{:max_id_len$}", task.id).bright_green().bold(),
            title,
            type_tag
        );
    }

    Ok(())
}

fn check_tasks(id: Option<&str>) -> Result<(), AppError> {
    let dir = PathBuf::from(TASKS_DIR);

    if !dir.exists() {
        return Err(AppError::DirNotFound(TASKS_DIR.to_string()));
    }

    let mut paths: Vec<_> = std::fs::read_dir(&dir)
        .with_context(&format!("failed to read directory {}", TASKS_DIR))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "md"))
        .collect();
    paths.sort();

    let mut tasks = Vec::new();
    for path in &paths {
        let path_str = path.display().to_string();
        let content = std::fs::read_to_string(path).with_context(&format!("failed to read {}", path_str))?;
        let parsed = task::parse(&content).with_path(&path_str)?;
        tasks.push(parsed);
    }

    if tasks.is_empty() {
        println!("No tasks found in {}", TASKS_DIR);
        return Ok(());
    }

    match id {
        Some(task_id) => check_single_task(&tasks, task_id),
        None => check_full_graph(tasks),
    }
}

fn check_single_task(tasks: &[task::Task], task_id: &str) -> Result<(), AppError> {
    let graph: validations::TaskGraph = tasks.iter().map(|t| (t.id.clone(), t.clone())).collect();

    let Some(task) = graph.get(task_id) else {
        return Err(AppError::TaskNotFound {
            task_id: task_id.to_string(),
            tasks_dir: TASKS_DIR.to_string(),
        });
    };

    validations::validate_task(task, &graph).with_tasks_dir(TASKS_DIR)?;

    println!("ok: task '{}' is valid", task_id);
    Ok(())
}

fn check_full_graph(tasks: Vec<task::Task>) -> Result<(), AppError> {
    let count = tasks.len();
    validations::validate_graph(tasks).with_tasks_dir(TASKS_DIR)?;
    println!("ok: {} tasks validated", count);
    Ok(())
}

fn new_task(
    id: Option<String>,
    title: Option<String>,
    description: Option<String>,
    parent: Option<String>,
    preconditions: Vec<String>,
    validations_list: Vec<String>,
    task_type: Option<TaskType>,
    editor: Option<Option<String>>,
    resume: Option<PathBuf>,
) -> Result<(), AppError> {
    let dir = PathBuf::from(TASKS_DIR);

    if !dir.exists() {
        std::fs::create_dir_all(&dir)
            .with_context(&format!("failed to create directory {}", TASKS_DIR))?;
    }

    // Resume mode: re-open a temp file from a previous failed validation
    if let Some(temp_path) = resume {
        let editor_name = editor.flatten();
        return resume_from_temp(&dir, &temp_path, editor_name.as_deref());
    }

    let existing_tasks = load_existing_tasks(&dir)?;
    let existing_ids: std::collections::HashSet<_> =
        existing_tasks.iter().map(|t| t.id.as_str()).collect();

    let task_id = resolve_task_id(id, title.as_deref(), &existing_ids)?;

    let content = build_task_content(&task_id, &title, &description, &parent, &preconditions, &validations_list, &task_type);

    // Editor mode: create temp file, open editor, validate on save
    if let Some(editor_opt) = editor {
        let editor_name = editor_opt.as_deref();
        return create_with_editor(&dir, &task_id, &content, editor_name);
    }

    // Non-editor mode: validate in memory, then write
    let new_task = task::parse(&content).with_path(&format!("{}/{}.md", TASKS_DIR, task_id))?;

    let mut all_tasks = existing_tasks;
    all_tasks.push(new_task);
    validations::validate_graph(all_tasks).with_tasks_dir(TASKS_DIR)?;

    let file_path = dir.join(format!("{}.md", task_id));
    std::fs::write(&file_path, &content)
        .with_context(&format!("failed to write {}", file_path.display()))?;

    println!("created: {}", file_path.display().to_string().bright_green());

    Ok(())
}

fn create_with_editor(
    tasks_dir: &PathBuf,
    task_id: &str,
    initial_content: &str,
    editor_name: Option<&str>,
) -> Result<(), AppError> {
    let temp_dir = std::env::temp_dir();
    let temp_path = temp_dir.join(format!("mont-task-{}.md", task_id));

    std::fs::write(&temp_path, initial_content)
        .with_context(&format!("failed to write temp file {}", temp_path.display()))?;

    let mut cmd = mont::resolve_editor(editor_name, &temp_path)?;
    cmd.status()
        .with_context("failed to run editor")?;

    validate_and_copy_temp(tasks_dir, &temp_path, editor_name)
}

fn resume_from_temp(
    tasks_dir: &PathBuf,
    temp_path: &PathBuf,
    editor_name: Option<&str>,
) -> Result<(), AppError> {
    if !temp_path.exists() {
        return Err(AppError::TempFileNotFound(temp_path.display().to_string()));
    }

    let mut cmd = mont::resolve_editor(editor_name, temp_path)?;
    cmd.status()
        .with_context("failed to run editor")?;

    validate_and_copy_temp(tasks_dir, temp_path, editor_name)
}

fn validate_and_copy_temp(
    tasks_dir: &PathBuf,
    temp_path: &PathBuf,
    editor_name: Option<&str>,
) -> Result<(), AppError> {
    let content = std::fs::read_to_string(temp_path)
        .with_context(&format!("failed to read temp file {}", temp_path.display()))?;

    let temp_path_str = temp_path.display().to_string();
    let parsed = match task::parse(&content).with_path(&temp_path_str) {
        Ok(t) => t,
        Err(e) => {
            return Err(AppError::TempValidationFailed {
                error: Box::new(e),
                temp_path: temp_path_str,
                editor_name: editor_name.map(String::from),
            });
        }
    };

    let existing_tasks = load_existing_tasks(tasks_dir)?;
    let existing_ids: std::collections::HashSet<_> =
        existing_tasks.iter().map(|t| t.id.as_str()).collect();

    if existing_ids.contains(parsed.id.as_str()) {
        return Err(AppError::TempValidationFailed {
            error: Box::new(AppError::IdAlreadyExists(parsed.id.clone())),
            temp_path: temp_path_str,
            editor_name: editor_name.map(String::from),
        });
    }

    let mut all_tasks = existing_tasks;
    all_tasks.push(parsed.clone());

    if let Err(e) = validations::validate_graph(all_tasks).with_tasks_dir(TASKS_DIR) {
        return Err(AppError::TempValidationFailed {
            error: Box::new(e),
            temp_path: temp_path_str,
            editor_name: editor_name.map(String::from),
        });
    }

    let final_path = tasks_dir.join(format!("{}.md", parsed.id));
    std::fs::write(&final_path, &content)
        .with_context(&format!("failed to write {}", final_path.display()))?;

    std::fs::remove_file(temp_path)
        .with_context(&format!("failed to remove temp file {}", temp_path.display()))?;

    println!("created: {}", final_path.display().to_string().bright_green());

    Ok(())
}

fn load_existing_tasks(dir: &PathBuf) -> Result<Vec<task::Task>, AppError> {
    let mut paths: Vec<_> = std::fs::read_dir(dir)
        .with_context(&format!("failed to read directory {}", TASKS_DIR))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "md"))
        .collect();
    paths.sort();

    let mut tasks = Vec::new();
    for path in &paths {
        let path_str = path.display().to_string();
        let content =
            std::fs::read_to_string(path).with_context(&format!("failed to read {}", path_str))?;
        let parsed = task::parse(&content).with_path(&path_str)?;
        tasks.push(parsed);
    }

    Ok(tasks)
}

fn resolve_task_id(
    id: Option<String>,
    title: Option<&str>,
    existing_ids: &std::collections::HashSet<&str>,
) -> Result<String, AppError> {
    if let Some(id) = id {
        return Ok(id);
    }

    if title.is_none() {
        return Err(AppError::IdOrTitleRequired);
    }

    const MAX_ATTEMPTS: u32 = 100;

    for _ in 0..MAX_ATTEMPTS {
        let Some(candidate) = petname::petname(2, "-") else {
            continue;
        };
        if !existing_ids.contains(candidate.as_str()) {
            return Ok(candidate);
        }
    }

    Err(AppError::IdGenerationFailed {
        attempts: MAX_ATTEMPTS,
    })
}

fn build_task_content(
    id: &str,
    title: &Option<String>,
    description: &Option<String>,
    parent: &Option<String>,
    preconditions: &[String],
    validations_list: &[String],
    task_type: &Option<TaskType>,
) -> String {
    let mut content = String::new();
    content.push_str("---\n");
    content.push_str(&format!("id: {}\n", id));

    if let Some(t) = title {
        content.push_str(&format!("title: {}\n", t));
    }

    if let Some(p) = parent {
        content.push_str(&format!("parent: {}\n", p));
    }

    if let Some(tt) = task_type {
        let type_str = match tt {
            TaskType::Feature => "feature",
            TaskType::Bug => "bug",
            TaskType::Epic => "epic",
        };
        content.push_str(&format!("type: {}\n", type_str));
    }

    if !preconditions.is_empty() {
        content.push_str("preconditions:\n");
        for pre in preconditions {
            content.push_str(&format!("  - {}\n", pre));
        }
    }

    if !validations_list.is_empty() {
        content.push_str("validations:\n");
        for val in validations_list {
            content.push_str(&format!("  - {}\n", val));
        }
    }

    content.push_str("---\n\n");

    if let Some(d) = description {
        content.push_str(d);
        content.push('\n');
    }

    content
}
