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
    use crate::display::layout::{build_grid, compute_layout, debug_render_grid};
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
        assert_eq!(routed.rows.len(), 5); // 3 task rows + 2 connection rows
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
        assert_eq!(routed.rows.len(), 3); // 2 task rows + 1 connection row
    }
}
