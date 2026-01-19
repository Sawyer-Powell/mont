//! Status command - shows current work status with task details and info.

use std::collections::HashSet;

use owo_colors::OwoColorize;

use crate::render::{format_task_line, print_gates_section, task_marker_for_state, TaskDisplayView};
use crate::{MontContext, Task, TaskGraph, TaskType};

/// Show status of in-progress tasks with full details, up-next tasks, and info.
pub fn status(ctx: &MontContext) {
    let graph = ctx.graph();
    let config = ctx.config();

    if graph.is_empty() {
        println!("No tasks found");
        return;
    }

    // Find all in-progress tasks
    let in_progress: Vec<_> = graph
        .values()
        .filter(|t| t.is_in_progress())
        .collect();

    // Tasks in Progress section
    println!("{}", "Tasks in Progress".bold());
    if in_progress.is_empty() {
        println!("  {}", "None".bright_black());
    } else {
        for (i, task) in in_progress.iter().enumerate() {
            if i > 0 {
                println!();
            }
            print_task_details(ctx, task);
        }
    }

    // Up Next section
    let up_next = find_up_next(&in_progress, &graph);
    println!();
    println!("{}", "Up Next".bold());
    if up_next.is_empty() {
        println!("  {}", "None".bright_black());
    } else {
        for task in up_next {
            let view = TaskDisplayView::from_task(task, &graph, &config.default_gates);
            let marker = task_marker_for_state(view.state);
            let line = format_task_line(task, &graph, &config.default_gates);
            println!("  {} {}", marker.bright_black(), line.bright_black());
        }
    }

    // Info section
    println!();
    println!("{}", "Info".bold());
    let ready_count = count_ready_tasks(&graph);
    let jot_count = graph.values().filter(|t| t.is_jot() && !t.is_complete()).count();
    let gate_count = graph.values().filter(|t| t.is_gate()).count();

    let completed_count = graph.values().filter(|t| t.is_complete()).count();

    // Left-align numbers in a 4-char field
    println!("  {:<4} tasks ready for work", ready_count.to_string().cyan());
    println!("  {:<4} jots needing distillation", jot_count.to_string().yellow());
    println!("  {:<4} gates", gate_count.to_string().purple());
    println!("  {:<4} completed", completed_count.to_string().bright_black());
}

fn print_task_details(ctx: &MontContext, task: &Task) {
    let graph = ctx.graph();
    let config = ctx.config();
    let view = TaskDisplayView::from_task(task, &graph, &config.default_gates);

    const LABEL_WIDTH: usize = 14;

    // Task ID
    println!("  {:LABEL_WIDTH$} {}", "Id".bold(), view.id_colored());

    // Title
    if task.title.is_some() {
        println!(
            "  {:LABEL_WIDTH$} {}",
            "Title".bold(),
            view.title_colored(usize::MAX)
        );
    }

    // Status
    println!("  {:LABEL_WIDTH$} {}", "Status".bold(), view.status_colored());

    // Type
    let type_value = match task.task_type {
        TaskType::Task => "[task]".bright_green().to_string(),
        TaskType::Jot => "[jot]".yellow().to_string(),
        TaskType::Gate => "[gate]".purple().to_string(),
    };
    println!("  {:LABEL_WIDTH$} {}", "Type".bold(), type_value);

    // Before
    if !task.before.is_empty() {
        println!(
            "  {:LABEL_WIDTH$} {}",
            "Before".bold(),
            task.before.join(", ").cyan()
        );
    }

    // After
    if !task.after.is_empty() {
        println!(
            "  {:LABEL_WIDTH$} {}",
            "After".bold(),
            task.after.join(", ").cyan()
        );
    }

    // Gates section using shared helper
    let all_gate_ids = ctx.all_gate_ids(task);
    print_gates_section(task, &all_gate_ids, "  ", LABEL_WIDTH);
}

/// Find tasks that are waiting on the in-progress tasks to complete.
fn find_up_next<'a>(in_progress: &[&Task], graph: &'a TaskGraph) -> Vec<&'a Task> {
    let in_progress_ids: HashSet<&str> = in_progress.iter().map(|t| t.id.as_str()).collect();

    let mut up_next: Vec<&Task> = graph
        .values()
        .filter(|task| {
            // Skip if already in progress or complete
            if task.is_in_progress() || task.is_complete() || task.is_gate() {
                return false;
            }

            // Check if this task depends on any in-progress task
            // via `after` relationship
            for after_id in &task.after {
                if in_progress_ids.contains(after_id.as_str()) {
                    return true;
                }
            }

            // Check if any in-progress task has this task in its `before` list
            for ip_task in in_progress {
                if ip_task.before.contains(&task.id) {
                    return true;
                }
            }

            false
        })
        .collect();

    up_next.sort_by(|a, b| a.id.cmp(&b.id));
    up_next
}

/// Count tasks that are ready for work (all preconditions met, not in progress/complete).
fn count_ready_tasks(graph: &TaskGraph) -> usize {
    graph
        .values()
        .filter(|task| {
            // Skip if already in progress, complete, or is a gate/jot
            if task.is_in_progress() || task.is_complete() || task.is_gate() || task.is_jot() {
                return false;
            }

            // Check all after dependencies are complete
            for after_id in &task.after {
                if graph.get(after_id).is_some_and(|t| !t.is_complete()) {
                    return false;
                }
            }

            true
        })
        .count()
}
