//! Delete command - deletes a task and removes all references to it.

use std::io::Write;

use owo_colors::OwoColorize;

use crate::error_fmt::{AppError, IoResultExt};
use crate::MontContext;

/// Delete a task and remove all references to it from other tasks.
pub fn delete(ctx: &MontContext, id: &str, force: bool) -> Result<(), AppError> {
    // Check if task exists
    if !ctx.graph().contains(id) {
        return Err(AppError::TaskNotFound {
            task_id: id.to_string(),
            tasks_dir: ctx.tasks_dir().display().to_string(),
        });
    }

    // Find all references to this task (for display purposes)
    let references = find_references(ctx, id);

    // Show summary and ask for confirmation
    if !force {
        println!("{} {}", "Deleting task:".bold(), id.bright_yellow());

        if references.is_empty() {
            println!("  No other tasks reference this task.");
        } else {
            println!("\n{}:", "References to remove".bold());
            for (task_id, ref_type) in &references {
                println!(
                    "  {} {} reference in {}",
                    "•".red(),
                    ref_type,
                    task_id.cyan()
                );
            }
        }

        println!();
        print!("Continue? [y/N] ");
        std::io::stdout().flush().with_context("failed to flush stdout")?;

        let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .with_context("failed to read input")?;

        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Aborted.");
            return Ok(());
        }
    }

    // Delete the task (MontContext handles reference cleanup and file removal)
    ctx.delete(id)?;

    // Show which tasks were updated (if any had references)
    for (task_id, _) in &references {
        println!("  updated: {}", task_id.cyan());
    }

    println!("{} {}", "deleted:".red(), id.bright_yellow());

    Ok(())
}

/// Find all tasks that reference the given task ID.
fn find_references(ctx: &MontContext, id: &str) -> Vec<(String, String)> {
    let graph = ctx.graph();
    let mut references = Vec::new();

    for task in graph.values() {
        if task.id == id {
            continue;
        }

        if task.before.contains(&id.to_string()) {
            references.push((task.id.clone(), "before".to_string()));
        }

        for dep in &task.after {
            if dep == id {
                references.push((task.id.clone(), "after".to_string()));
            }
        }

        for val in &task.validations {
            if val.id == id {
                references.push((task.id.clone(), "validation".to_string()));
            }
        }
    }

    references
}
