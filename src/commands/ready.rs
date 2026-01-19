//! Ready command - shows tasks ready to work on.

use owo_colors::OwoColorize;

use crate::context::graph::available_tasks;
use crate::render;
use crate::{MontContext, Task, TaskType};

/// Max title length for ready output.
const READY_MAX_TITLE_LEN: usize = 120;

/// Show tasks that are ready to work on (all dependencies complete).
pub fn ready(ctx: &MontContext) {
    let graph = ctx.graph();

    if graph.is_empty() {
        println!("No ready tasks");
        return;
    }

    let ready: Vec<_> = available_tasks(&graph);

    if ready.is_empty() {
        println!("No ready tasks");
        return;
    }

    // Split into regular tasks and standalone jots
    let (regular, jots): (Vec<_>, Vec<_>) = ready
        .into_iter()
        .partition(|t| !t.is_jot());

    // Sort each group by id
    let mut regular = regular;
    let mut jots = jots;
    regular.sort_by(|a, b| a.id.cmp(&b.id));
    jots.sort_by(|a, b| a.id.cmp(&b.id));

    // Print regular tasks first, then jots
    let all_tasks: Vec<_> = regular.into_iter().chain(jots).collect();

    let max_id_len = all_tasks.iter().map(|t| t.id.len()).max().unwrap_or(0);

    for task in all_tasks {
        print_ready_task(task, max_id_len);
    }
}

fn print_ready_task(task: &Task, max_id_len: usize) {
    let title = render::truncate_to(task.title.as_deref().unwrap_or(""), READY_MAX_TITLE_LEN);

    // Format: [type]  id  title (same as fzf picker)
    let (type_tag, id_display) = match task.task_type {
        TaskType::Task => (
            "[task]".bright_green().bold().to_string(),
            format!("{:max_id_len$}", task.id).bright_green().bold().to_string(),
        ),
        TaskType::Jot => (
            "[jot] ".yellow().bold().to_string(),
            format!("{:max_id_len$}", task.id).yellow().bold().to_string(),
        ),
        TaskType::Gate => (
            "[gate]".purple().bold().to_string(),
            format!("{:max_id_len$}", task.id).purple().bold().to_string(),
        ),
    };

    println!("{}  {}  {}", type_tag, id_display, title);
}
