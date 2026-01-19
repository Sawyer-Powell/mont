//! Ready command - shows tasks ready to work on.

use crate::context::graph::available_tasks;
use crate::render::{task_marker_for_state, DisplayState, TaskDisplayView};
use crate::MontContext;

/// Max title length for ready output.
const READY_MAX_TITLE_LEN: usize = 120;

/// Show tasks that are ready to work on (all dependencies complete).
pub fn ready(ctx: &MontContext) {
    let graph = ctx.graph();
    let config = ctx.config();

    if graph.is_empty() {
        println!("No ready tasks");
        return;
    }

    let ready: Vec<_> = available_tasks(&graph);

    if ready.is_empty() {
        println!("No ready tasks");
        return;
    }

    // Build display views for all ready tasks
    let views: Vec<_> = ready
        .into_iter()
        .map(|t| TaskDisplayView::from_task(t, &graph, &config.default_gates))
        .collect();

    // Split into in-progress, regular tasks, and jots
    let (in_progress, rest): (Vec<_>, Vec<_>) = views
        .into_iter()
        .partition(|v| v.state == DisplayState::InProgress);

    let (mut regular, mut jots): (Vec<_>, Vec<_>) = rest
        .into_iter()
        .partition(|v| v.state != DisplayState::Jot);

    // Sort each group by id
    regular.sort_by(|a, b| a.id.cmp(&b.id));
    jots.sort_by(|a, b| a.id.cmp(&b.id));

    // Calculate max id length across all groups
    let all_views: Vec<&TaskDisplayView> = in_progress.iter()
        .chain(regular.iter())
        .chain(jots.iter())
        .collect();
    let max_id_len = all_views.iter().map(|v| v.id.len()).max().unwrap_or(0);

    // Print in-progress first (separated)
    for view in &in_progress {
        print_ready_line(view, max_id_len);
    }

    // Separator if we have in-progress tasks and other tasks
    if !in_progress.is_empty() && (!regular.is_empty() || !jots.is_empty()) {
        println!();
    }

    // Print regular tasks
    for view in &regular {
        print_ready_line(view, max_id_len);
    }

    // Print jots
    for view in &jots {
        print_ready_line(view, max_id_len);
    }
}

fn print_ready_line(view: &TaskDisplayView, max_id_len: usize) {
    let marker = task_marker_for_state(view.state);
    let line = view.format_line_padded(max_id_len, READY_MAX_TITLE_LEN);
    println!("{} {}", marker, line);
}
