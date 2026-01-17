use clap::{Parser, Subcommand};
use owo_colors::OwoColorize;
use std::path::PathBuf;

use mont::error_fmt::{AppError, IoResultExt, ParseResultExt, ValidationResultExt};
use mont::task::TaskType;
use mont::{display, graph, task, validations};

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
}

const TASKS_DIR: &str = ".tasks";

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

    let output = display::render_task_graph(&validated, show_completed);
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
        .map(|t| display::truncate_title(t.title.as_deref().unwrap_or("")).len())
        .max()
        .unwrap_or(0);

    for task in ready {
        let title = display::truncate_title(task.title.as_deref().unwrap_or(""));
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
