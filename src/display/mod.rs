pub mod layout;
pub mod routing;

// Re-export types for convenience
pub use layout::{Cell, Grid, Layout, Position, build_grid, compute_layout, debug_render_grid};
pub use routing::route_edges;
