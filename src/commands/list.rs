//! List command - displays all tasks in the task graph.

use crate::render;
use crate::MontContext;

/// List all tasks in the task graph.
pub fn list(ctx: &MontContext, show_completed: bool) {
    let graph = ctx.graph();
    let config = ctx.config();

    if graph.is_empty() {
        println!("No tasks found");
        return;
    }

    let output = render::render_task_graph(&graph, &config.default_gates, show_completed);
    print!("{}", output);
}
