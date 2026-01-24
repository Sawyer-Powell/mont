//! Show command - displays details for a single task.

use owo_colors::OwoColorize;

use crate::error_fmt::AppError;
use crate::render::{print_gates_section, TaskDisplayView};
use crate::{MontContext, Task, TaskType};

/// Show details for a single task.
pub fn show(ctx: &MontContext, id: &str, short: bool) -> Result<(), AppError> {
    let task = ctx
        .graph()
        .get(id)
        .ok_or_else(|| AppError::TaskNotFound {
            task_id: id.to_string(),
            tasks_dir: ctx.tasks_dir().display().to_string(),
        })?
        .clone();

    // Print task details using shared helpers
    print_task_details(ctx, &task, short);

    Ok(())
}

fn print_task_details(ctx: &MontContext, task: &Task, short: bool) {
    let graph = ctx.graph();
    let config = ctx.config();
    let view = TaskDisplayView::from_task(task, &graph, &config.default_gates);

    const LABEL_WIDTH: usize = 14;

    // Task ID
    println!("{:LABEL_WIDTH$} {}", "Id".bold(), view.id_colored());

    // Title
    if task.title.is_some() {
        println!(
            "{:LABEL_WIDTH$} {}",
            "Title".bold(),
            view.title_colored(usize::MAX)
        );
    }

    // Status
    println!("{:LABEL_WIDTH$} {}", "Status".bold(), view.status_colored());

    // Type
    let type_value = match task.task_type {
        TaskType::Task => "[task]".bright_green().to_string(),
        TaskType::Jot => "[jot]".yellow().to_string(),
        TaskType::Gate => "[gate]".purple().to_string(),
    };
    println!("{:LABEL_WIDTH$} {}", "Type".bold(), type_value);

    // Before
    if !task.before.is_empty() {
        println!(
            "{:LABEL_WIDTH$} {}",
            "Before".bold(),
            task.before.join(", ").cyan()
        );
    }

    // After
    if !task.after.is_empty() {
        println!(
            "{:LABEL_WIDTH$} {}",
            "After".bold(),
            task.after.join(", ").cyan()
        );
    }

    // Gates section using shared helper
    let all_gate_ids = ctx.all_gate_ids(task);
    print_gates_section(task, &all_gate_ids, "", LABEL_WIDTH);

    // Description (unless short mode)
    if !short && !task.description.is_empty() {
        println!();
        let mut skin = termimad::MadSkin::default();
        skin.headers[0].align = termimad::Alignment::Left;
        skin.headers[1].align = termimad::Alignment::Left;
        skin.headers[2].align = termimad::Alignment::Left;
        skin.headers[3].align = termimad::Alignment::Left;
        skin.headers[4].align = termimad::Alignment::Left;
        skin.headers[5].align = termimad::Alignment::Left;
        skin.print_text(&task.description);
    }
}
