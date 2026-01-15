use clap::{Parser, Subcommand};
use std::path::PathBuf;

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
                eprintln!("error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Check { dir, id } => {
            if let Err(e) = check_tasks(&dir, id.as_deref()) {
                eprintln!("error: {}", e);
                std::process::exit(1);
            }
        }
    }
}

fn list_tasks(dir: &PathBuf, show_completed: bool) -> Result<(), Box<dyn std::error::Error>> {
    // Check if directory exists
    if !dir.exists() {
        return Err(format!("tasks directory not found: {}", dir.display()).into());
    }

    // Find all .md files in the directory (sorted for determinism)
    let mut paths: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map_or(false, |ext| ext == "md"))
        .collect();
    paths.sort();

    let mut tasks = Vec::new();
    for path in paths {
        let content = std::fs::read_to_string(&path)?;
        match task::parse(&content) {
            Ok(task) => tasks.push(task),
            Err(e) => {
                eprintln!("warning: failed to parse {}: {}", path.display(), e);
            }
        }
    }

    if tasks.is_empty() {
        println!("No tasks found in {}", dir.display());
        return Ok(());
    }

    // Validate the task graph
    let validated = graph::form_graph(tasks)?;

    // Convert to sorted Vec for deterministic rendering
    let mut task_vec: Vec<_> = validated.into_values().collect();
    task_vec.sort_by(|a, b| a.id.cmp(&b.id));

    // Filter out completed tasks unless --show-completed is passed
    if !show_completed {
        task_vec.retain(|t| !t.complete);
    }

    // Render and print
    let output = display::render_task_graph(&task_vec);
    print!("{}", output);

    Ok(())
}

fn check_tasks(dir: &PathBuf, id: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    if !dir.exists() {
        return Err(format!("tasks directory not found: {}", dir.display()).into());
    }

    let mut paths: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map_or(false, |ext| ext == "md"))
        .collect();
    paths.sort();

    let mut tasks = Vec::new();
    for path in paths {
        let content = std::fs::read_to_string(&path)?;
        match task::parse(&content) {
            Ok(t) => tasks.push(t),
            Err(e) => {
                return Err(format!("failed to parse {}: {}", path.display(), e).into());
            }
        }
    }

    if tasks.is_empty() {
        println!("No tasks found in {}", dir.display());
        return Ok(());
    }

    match id {
        Some(task_id) => check_single_task(&tasks, task_id),
        None => check_full_graph(tasks),
    }
}

fn check_single_task(
    tasks: &[task::Task],
    task_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let graph: validations::TaskGraph = tasks
        .iter()
        .map(|t| (t.id.clone(), t.clone()))
        .collect();

    let Some(task) = graph.get(task_id) else {
        return Err(format!("task '{}' not found", task_id).into());
    };

    validations::validate_task(task, &graph)?;

    println!("ok: task '{}' is valid", task_id);
    Ok(())
}

fn check_full_graph(tasks: Vec<task::Task>) -> Result<(), Box<dyn std::error::Error>> {
    let count = tasks.len();
    validations::validate_graph(tasks)?;
    println!("ok: {} tasks validated", count);
    Ok(())
}
