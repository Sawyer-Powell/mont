//! Ready command - shows tasks ready to work on.

use owo_colors::OwoColorize;

use crate::context::graph::available_tasks;
use crate::render;
use crate::{MontContext, TaskType};

/// Show tasks that are ready to work on (all dependencies complete).
pub fn ready(ctx: &MontContext) {
    let graph = ctx.graph();

    if graph.is_empty() {
        println!("No ready tasks");
        return;
    }

    let mut ready: Vec<_> = available_tasks(&graph);
    ready.sort_by(|a, b| a.id.cmp(&b.id));

    if ready.is_empty() {
        println!("No ready tasks");
        return;
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
            TaskType::Task => String::new(),
            TaskType::Jot => format!("{}", "[jot]".yellow().bold()),
            TaskType::Gate => format!("{}", "[gate]".purple().bold()),
        };
        println!(
            "{}  {:max_title_len$}  {}",
            format!("{:max_id_len$}", task.id).bright_green().bold(),
            title,
            type_tag
        );
    }
}
