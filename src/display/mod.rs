pub mod layout;
pub mod render;
pub mod routing;

// Re-export types for convenience
pub use layout::{Cell, Grid, Layout, Position, build_grid, compute_layout, debug_render_grid};
pub use render::connection_symbol;
pub use routing::route_edges;
