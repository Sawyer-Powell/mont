use clap::{Parser, Subcommand};
use std::path::PathBuf;

use mont::{display, graph, task};

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
