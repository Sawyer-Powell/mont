use clap::{Parser, Subcommand};
use std::path::PathBuf;

use mont::error_fmt::{AppError, IoResultExt, ParseResultExt, ValidationResultExt};
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
        /// Directory containing task files (default: .tasks)
        #[arg(short, long, default_value = ".tasks")]
        dir: PathBuf,

        /// Show completed tasks (hidden by default)
        #[arg(long)]
        show_completed: bool,
    },
    /// Validate the task graph
    Check {
        /// Directory containing task files (default: .tasks)
        #[arg(short, long, default_value = ".tasks")]
        dir: PathBuf,

        /// Specific task ID to validate (validates entire graph if not provided)
        id: Option<String>,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::List { dir, show_completed } => {
            if let Err(e) = list_tasks(&dir, show_completed) {
                eprint!("{}", e);
                std::process::exit(1);
            }
        }
        Commands::Check { dir, id } => {
            if let Err(e) = check_tasks(&dir, id.as_deref()) {
                eprint!("{}", e);
                std::process::exit(1);
            }
        }
    }
}

fn list_tasks(dir: &PathBuf, show_completed: bool) -> Result<(), AppError> {
    let dir_str = dir.display().to_string();

    if !dir.exists() {
        return Err(AppError::DirNotFound(dir_str));
    }

    let mut paths: Vec<_> = std::fs::read_dir(dir)
        .with_context(&format!("failed to read directory {}", dir_str))?
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
        println!("No tasks found in {}", dir.display());
        return Ok(());
    }

    let validated = graph::form_graph(tasks).with_tasks_dir(&dir_str)?;

    let mut task_vec: Vec<_> = validated.into_values().collect();
    task_vec.sort_by(|a, b| a.id.cmp(&b.id));

    if !show_completed {
        task_vec.retain(|t| !t.complete);
    }

    let output = display::render_task_graph(&task_vec);
    print!("{}", output);

    Ok(())
}

fn check_tasks(dir: &PathBuf, id: Option<&str>) -> Result<(), AppError> {
    let dir_str = dir.display().to_string();

    if !dir.exists() {
        return Err(AppError::DirNotFound(dir_str));
    }

    let mut paths: Vec<_> = std::fs::read_dir(dir)
        .with_context(&format!("failed to read directory {}", dir_str))?
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
        println!("No tasks found in {}", dir.display());
        return Ok(());
    }

    match id {
        Some(task_id) => check_single_task(&tasks, task_id, &dir_str),
        None => check_full_graph(tasks, &dir_str),
    }
}

fn check_single_task(tasks: &[task::Task], task_id: &str, dir_str: &str) -> Result<(), AppError> {
    let graph: validations::TaskGraph = tasks.iter().map(|t| (t.id.clone(), t.clone())).collect();

    let Some(task) = graph.get(task_id) else {
        return Err(AppError::TaskNotFound {
            task_id: task_id.to_string(),
            tasks_dir: dir_str.to_string(),
        });
    };

    validations::validate_task(task, &graph).with_tasks_dir(dir_str)?;

    println!("ok: task '{}' is valid", task_id);
    Ok(())
}

fn check_full_graph(tasks: Vec<task::Task>, dir_str: &str) -> Result<(), AppError> {
    let count = tasks.len();
    validations::validate_graph(tasks).with_tasks_dir(dir_str)?;
    println!("ok: {} tasks validated", count);
    Ok(())
}
