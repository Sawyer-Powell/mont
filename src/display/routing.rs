use std::collections::HashMap;

use super::layout::{Cell, Grid, Layout};

/// Route edges through the grid, updating Connection cells.
///
/// This function takes a grid with Task cells placed and adds Connection cells
/// to represent the edges between tasks.
///
/// The grid is expanded to insert connection rows between task rows.
/// After expansion:
/// - Even rows (0, 2, 4, ...) contain task cells
/// - Odd rows (1, 3, 5, ...) contain connection cells
pub fn route_edges(grid: &Grid, layout: &Layout) -> Grid {
    if layout.edges.is_empty() || grid.rows.is_empty() {
        return grid.clone();
    }

    // Build position lookup: task_id -> (row, col) in the original grid
    let mut task_positions: HashMap<&str, (usize, usize)> = HashMap::new();
    for (row_idx, row) in grid.rows.iter().enumerate() {
        for (col_idx, cell) in row.iter().enumerate() {
            if let Cell::Task(id) = cell {
                task_positions.insert(id.as_str(), (row_idx, col_idx));
            }
        }
    }

    // Create expanded grid: double the rows minus 1, add connection rows between task rows
    let original_rows = grid.rows.len();
    let original_cols = grid.width();
    let expanded_rows = original_rows * 2 - 1;

    let mut expanded = Grid::new();
    expanded.ensure_size(expanded_rows, original_cols);

    // Copy task rows to even positions
    for (orig_row, row) in grid.rows.iter().enumerate() {
        let expanded_row = orig_row * 2;
        for (col, cell) in row.iter().enumerate() {
            expanded.rows[expanded_row][col] = cell.clone();
        }
    }

    // Route each edge
    for (from_id, to_id) in &layout.edges {
        let Some(&(from_row, from_col)) = task_positions.get(from_id.as_str()) else {
            continue;
        };
        let Some(&(to_row, to_col)) = task_positions.get(to_id.as_str()) else {
            continue;
        };

        // Convert to expanded grid coordinates
        let from_expanded_row = from_row * 2;
        let to_expanded_row = to_row * 2;

        route_single_edge(
            &mut expanded,
            from_expanded_row,
            from_col,
            to_expanded_row,
            to_col,
        );
    }

    expanded
}

/// Route a single edge from (from_row, from_col) to (to_row, to_col).
///
/// Strategy:
/// 1. Go down from the source task
/// 2. If horizontal movement needed, go right/left in a connection row
/// 3. Go down to the target task
fn route_single_edge(
    grid: &mut Grid,
    from_row: usize,
    from_col: usize,
    to_row: usize,
    to_col: usize,
) {
    // Handle same-level edges (shouldn't happen with proper DAG, but handle gracefully)
    if from_row >= to_row {
        return;
    }

    // Mark the source cell as having a downward connection
    set_connection_flag(grid, from_row, from_col, Direction::Down);

    // Determine the connection row (one below the source task row)
    let conn_row = from_row + 1;

    if from_col == to_col {
        // Straight vertical: just go down
        // Mark connection row
        set_connection_flag(grid, conn_row, from_col, Direction::Up);
        set_connection_flag(grid, conn_row, from_col, Direction::Down);

        // Continue down through any intermediate rows
        for row in (conn_row + 1)..to_row {
            set_connection_flag(grid, row, from_col, Direction::Up);
            set_connection_flag(grid, row, from_col, Direction::Down);
        }
    } else {
        // Need horizontal routing
        // Go down from source
        set_connection_flag(grid, conn_row, from_col, Direction::Up);

        // Horizontal segment
        let (left_col, right_col) = if from_col < to_col {
            set_connection_flag(grid, conn_row, from_col, Direction::Right);
            (from_col, to_col)
        } else {
            set_connection_flag(grid, conn_row, from_col, Direction::Left);
            (to_col, from_col)
        };

        // Draw horizontal line
        for col in (left_col + 1)..right_col {
            set_connection_flag(grid, conn_row, col, Direction::Left);
            set_connection_flag(grid, conn_row, col, Direction::Right);
        }

        // Turn down at destination column
        if from_col < to_col {
            set_connection_flag(grid, conn_row, to_col, Direction::Left);
        } else {
            set_connection_flag(grid, conn_row, to_col, Direction::Right);
        }
        set_connection_flag(grid, conn_row, to_col, Direction::Down);

        // Continue down to target
        for row in (conn_row + 1)..to_row {
            set_connection_flag(grid, row, to_col, Direction::Up);
            set_connection_flag(grid, row, to_col, Direction::Down);
        }
    }

    // Mark the target cell as having an upward connection
    set_connection_flag(grid, to_row, to_col, Direction::Up);
}

#[derive(Debug, Clone, Copy)]
enum Direction {
    Up,
    Down,
    Left,
    Right,
}

/// Set a direction flag on a cell, converting Empty cells to Connection cells.
fn set_connection_flag(grid: &mut Grid, row: usize, col: usize, dir: Direction) {
    grid.ensure_size(row + 1, col + 1);

    let cell = &mut grid.rows[row][col];

    match cell {
        Cell::Task(_) => {
            // Don't modify task cells - they'll handle their own connections in rendering
        }
        Cell::Empty => {
            // Convert to connection cell
            let mut conn = Cell::connection();
            if let Cell::Connection {
                ref mut up,
                ref mut down,
                ref mut left,
                ref mut right,
            } = conn
            {
                match dir {
                    Direction::Up => *up = true,
                    Direction::Down => *down = true,
                    Direction::Left => *left = true,
                    Direction::Right => *right = true,
                }
            }
            *cell = conn;
        }
        Cell::Connection {
            up,
            down,
            left,
            right,
        } => {
            match dir {
                Direction::Up => *up = true,
                Direction::Down => *down = true,
                Direction::Left => *left = true,
                Direction::Right => *right = true,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::display::layout::{build_grid, compute_layout, debug_render_grid, prune_rows, skew_grid};
    use crate::graph::TaskGraph;
    use crate::task::{Task, TaskType};

    fn make_task(id: &str) -> Task {
        Task {
            id: id.to_string(),
            parent: None,
            preconditions: vec![],
            validations: vec![],
            title: None,
            validator: false,
            complete: false,
            in_progress: None,
            task_type: TaskType::Feature,
            description: String::new(),
        }
    }

    fn make_task_with_parent(id: &str, parent: &str) -> Task {
        Task {
            id: id.to_string(),
            parent: Some(parent.to_string()),
            preconditions: vec![],
            validations: vec![],
            title: None,
            validator: false,
            complete: false,
            in_progress: None,
            task_type: TaskType::Feature,
            description: String::new(),
        }
    }

    fn build_graph(tasks: Vec<Task>) -> TaskGraph {
        tasks.into_iter().map(|t| (t.id.clone(), t)).collect()
    }

    #[test]
    fn test_chain_routing() {
        // A -> B -> C (vertical chain)
        let c = make_task("C");
        let b = make_task_with_parent("B", "C");
        let a = make_task_with_parent("A", "B");

        let graph = build_graph(vec![a, b, c]);
        let layout = compute_layout(&graph);
        let grid = build_grid(&layout);
        let routed = route_edges(&grid, &layout);

        println!("\n=== Chain Routing ===");
        println!("Before:\n{}", debug_render_grid(&grid));
        println!("After:\n{}", debug_render_grid(&routed));

        // Verify structure: A, connection, B, connection, C
        assert_eq!(routed.rows.len(), 5); // 3 task rows + 2 connection rows
    }

    #[test]
    fn test_diamond_routing() {
        // Diamond: P forks to A and B, both point to R
        let r = make_task("R");
        let p = make_task_with_parent("P", "R");
        let mut a = make_task_with_parent("A", "R");
        a.preconditions = vec!["P".to_string()];
        let mut b = make_task_with_parent("B", "R");
        b.preconditions = vec!["P".to_string()];

        let graph = build_graph(vec![r, p, a, b]);
        let layout = compute_layout(&graph);
        let grid = build_grid(&layout);
        let routed = route_edges(&grid, &layout);

        println!("\n=== Diamond Routing ===");
        println!("Before:\n{}", debug_render_grid(&grid));
        println!("After:\n{}", debug_render_grid(&routed));

        // Verify expanded structure
        // build_grid creates 4 task rows (P, A, B, R), routing adds connection rows
        assert_eq!(routed.rows.len(), 7);
    }

    #[test]
    fn test_wide_merge_routing() {
        // Four tasks A, B, C, D all merge to R
        let r = make_task("R");
        let a = make_task_with_parent("A", "R");
        let b = make_task_with_parent("B", "R");
        let c = make_task_with_parent("C", "R");
        let d = make_task_with_parent("D", "R");

        let graph = build_graph(vec![r, a, b, c, d]);
        let layout = compute_layout(&graph);
        let grid = build_grid(&layout);
        let routed = route_edges(&grid, &layout);

        println!("\n=== Wide Merge Routing ===");
        println!("Before:\n{}", debug_render_grid(&grid));
        println!("After:\n{}", debug_render_grid(&routed));

        // Verify expanded structure
        // build_grid creates 5 task rows (A, B, C, D, R), routing adds connection rows
        assert_eq!(routed.rows.len(), 9);
    }

    /// Criss-cross: edges that conceptually cross
    ///
    ///   A   B        <- level 0
    ///    \ /
    ///     X          <- level 1 (A→C, B→D but C/D positions swap)
    ///    / \
    ///   C   D        <- level 2
    ///    \ /
    ///     Z          <- level 3
    ///
    /// A is at col 0, B at col 1
    /// C depends on B (wants col 1), D depends on A (wants col 0)
    /// This tests whether edges cross cleanly
    #[test]
    fn test_criss_cross_routing() {
        let z = make_task("Z");
        let mut c = make_task_with_parent("C", "Z");
        c.preconditions = vec!["B".to_string()];
        let mut d = make_task_with_parent("D", "Z");
        d.preconditions = vec!["A".to_string()];
        let mut a = make_task_with_parent("A", "Z");
        a.preconditions = vec![];
        let mut b = make_task_with_parent("B", "Z");
        b.preconditions = vec![];

        let graph = build_graph(vec![a, b, c, d, z]);
        let layout = compute_layout(&graph);

        println!("\n=== Criss-Cross ===");
        println!("Levels: {:?}", layout.levels);
        println!("Edges: {:?}", layout.edges);

        let grid = build_grid(&layout);
        let routed = route_edges(&grid, &layout);

        println!("Before:\n{}", debug_render_grid(&grid));
        println!("After:\n{}", debug_render_grid(&routed));
    }

    /// Wide fork to narrow merge: 4 children merge to one parent
    /// All edges route to column 0
    ///
    ///   A            <- level 0, col 0
    ///  /|\\
    /// B C D E        <- level 1, cols 0,1,2,3
    ///  \|/|/
    ///   Z            <- level 2, col 0
    #[test]
    fn test_wide_fork_narrow_merge() {
        let z = make_task("Z");
        let a = make_task_with_parent("A", "Z");
        let mut b = make_task_with_parent("B", "Z");
        b.preconditions = vec!["A".to_string()];
        let mut c = make_task_with_parent("C", "Z");
        c.preconditions = vec!["A".to_string()];
        let mut d = make_task_with_parent("D", "Z");
        d.preconditions = vec!["A".to_string()];
        let mut e = make_task_with_parent("E", "Z");
        e.preconditions = vec!["A".to_string()];

        let graph = build_graph(vec![a, b, c, d, e, z]);
        let layout = compute_layout(&graph);

        println!("\n=== Wide Fork Narrow Merge ===");
        println!("Levels: {:?}", layout.levels);
        println!("Edges: {:?}", layout.edges);

        let grid = build_grid(&layout);
        let routed = route_edges(&grid, &layout);

        println!("Before:\n{}", debug_render_grid(&grid));
        println!("After:\n{}", debug_render_grid(&routed));
    }

    /// Long horizontal route: task with predecessors spread far apart
    ///
    /// A B C D E      <- level 0: 5 independent sources
    /// │ │ │ │ │
    /// F G H I J      <- level 1: each under its predecessor
    /// └─┴─┴─┴─┘
    ///     X          <- level 2: X depends on F and J (cols 0 and 4)
    ///     │
    ///     Z          <- level 3
    ///
    /// The J→X edge needs to route from col 4 to col 0
    #[test]
    fn test_long_horizontal_route() {
        let z = make_task("Z");

        // X depends on F and J (far apart)
        let mut x = make_task_with_parent("X", "Z");
        x.preconditions = vec!["F".to_string(), "J".to_string()];

        // Level 1: F, G, H, I, J - each depends on corresponding source
        let mut f = make_task_with_parent("F", "Z");
        f.preconditions = vec!["A".to_string()];
        let mut g = make_task_with_parent("G", "Z");
        g.preconditions = vec!["B".to_string()];
        let mut h = make_task_with_parent("H", "Z");
        h.preconditions = vec!["C".to_string()];
        let mut i = make_task_with_parent("I", "Z");
        i.preconditions = vec!["D".to_string()];
        let mut j = make_task_with_parent("J", "Z");
        j.preconditions = vec!["E".to_string()];

        // Level 0: A, B, C, D, E - independent sources
        let a = make_task_with_parent("A", "Z");
        let b = make_task_with_parent("B", "Z");
        let c = make_task_with_parent("C", "Z");
        let d = make_task_with_parent("D", "Z");
        let e = make_task_with_parent("E", "Z");

        let graph = build_graph(vec![a, b, c, d, e, f, g, h, i, j, x, z]);
        let layout = compute_layout(&graph);

        println!("\n=== Long Horizontal Route ===");
        println!("Levels: {:?}", layout.levels);
        println!("Edges: {:?}", layout.edges);

        let grid = build_grid(&layout);
        let routed = route_edges(&grid, &layout);

        println!("Before:\n{}", debug_render_grid(&grid));
        println!("After:\n{}", debug_render_grid(&routed));
    }

    /// Two independent components side by side
    /// Component 1: A → B → C
    /// Component 2: X → Y → Z
    #[test]
    fn test_independent_components() {
        // Component 1
        let c = make_task("C");
        let b = make_task_with_parent("B", "C");
        let a = make_task_with_parent("A", "B");

        // Component 2
        let z = make_task("Z");
        let y = make_task_with_parent("Y", "Z");
        let x = make_task_with_parent("X", "Y");

        let graph = build_graph(vec![a, b, c, x, y, z]);
        let layout = compute_layout(&graph);

        println!("\n=== Independent Components ===");
        println!("Levels: {:?}", layout.levels);
        println!("Edges: {:?}", layout.edges);

        let grid = build_grid(&layout);
        let routed = route_edges(&grid, &layout);

        println!("Before:\n{}", debug_render_grid(&grid));
        println!("After:\n{}", debug_render_grid(&routed));
    }

    /// Long span with parallel paths and diamond - SKEWED version
    ///
    /// Same structure as test_parallel_diamond_routing but with skewing applied first
    #[test]
    fn test_parallel_diamond_skewed_routing() {
        // Z is the root
        let z = make_task("Z");

        // X depends on C and D (diamond bottom)
        let mut x = make_task_with_parent("X", "Z");
        x.preconditions = vec!["C".to_string(), "D".to_string()];

        // C and D depend on P (diamond middle)
        let mut c = make_task_with_parent("C", "Z");
        c.preconditions = vec!["P".to_string()];
        let mut d = make_task_with_parent("D", "Z");
        d.preconditions = vec!["P".to_string()];

        // P depends on A (diamond tip)
        let mut p = make_task_with_parent("P", "Z");
        p.preconditions = vec!["A".to_string()];

        // Parallel path: A → B → E → Z
        let mut b = make_task_with_parent("B", "Z");
        b.preconditions = vec!["A".to_string()];
        let mut e = make_task_with_parent("E", "Z");
        e.preconditions = vec!["B".to_string()];

        // A is the source
        let a = make_task_with_parent("A", "Z");

        let graph = build_graph(vec![a, b, c, d, e, p, x, z]);
        let layout = compute_layout(&graph);
        let grid = build_grid(&layout);
        let skewed = skew_grid(&grid);
        let routed = route_edges(&skewed, &layout);
        let pruned = prune_rows(&routed);

        println!("\n=== Parallel Diamond SKEWED Routing ===");
        println!("Original grid:\n{}", debug_render_grid(&grid));
        println!("Skewed grid:\n{}", debug_render_grid(&skewed));
        println!("Routed:\n{}", debug_render_grid(&routed));
        println!("Pruned:\n{}", debug_render_grid(&pruned));
    }

    /// Long span with parallel paths and diamond
    ///
    ///       A              <- level 0
    ///      / \
    ///     P   B            <- level 1
    ///    /|\   \
    ///   C D     E          <- level 2
    ///    \|     |
    ///     X     |          <- level 3
    ///      \    |
    ///       Z<--+          <- level 4 (E→Z spans 2 levels)
    ///
    /// Edges: A→P, A→B, P→C, P→D, C→X, D→X, B→E, E→Z, X→Z
    #[test]
    fn test_parallel_diamond_routing() {
        // Z is the root
        let z = make_task("Z");

        // X depends on C and D (diamond bottom)
        let mut x = make_task_with_parent("X", "Z");
        x.preconditions = vec!["C".to_string(), "D".to_string()];

        // C and D depend on P (diamond middle)
        let mut c = make_task_with_parent("C", "Z");
        c.preconditions = vec!["P".to_string()];
        let mut d = make_task_with_parent("D", "Z");
        d.preconditions = vec!["P".to_string()];

        // P depends on A (diamond tip)
        let mut p = make_task_with_parent("P", "Z");
        p.preconditions = vec!["A".to_string()];

        // Parallel path: A → B → E → Z
        let mut b = make_task_with_parent("B", "Z");
        b.preconditions = vec!["A".to_string()];
        let mut e = make_task_with_parent("E", "Z");
        e.preconditions = vec!["B".to_string()];

        // A is the source
        let a = make_task_with_parent("A", "Z");

        let graph = build_graph(vec![a, b, c, d, e, p, x, z]);
        let layout = compute_layout(&graph);

        println!("\n=== Parallel Diamond ===");
        println!("Levels: {:?}", layout.levels);
        println!("Edges: {:?}", layout.edges);

        let grid = build_grid(&layout);
        let routed = route_edges(&grid, &layout);

        println!("Before:\n{}", debug_render_grid(&grid));
        println!("After:\n{}", debug_render_grid(&routed));

        // build_grid creates 8 task rows (one per task), routing adds connection rows
        // Level 0: A (1)
        // Level 1: P, B (2)
        // Level 2: C, D, E (3)
        // Level 3: X (1)
        // Level 4: Z (1)
        // 8 task rows + 7 connection rows = 15 rows
        assert_eq!(routed.rows.len(), 15);
    }
}
