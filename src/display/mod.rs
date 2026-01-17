pub mod layout;
pub mod render;
pub mod routing;

// Re-export types for convenience
pub use layout::{Cell, Grid, Layout, Position, build_grid, compute_layout, debug_render_grid, prune_rows};
pub use render::{connection_symbol, render_grid, render_task_graph, truncate_title, MAX_TITLE_LEN};
pub use routing::route_edges;
