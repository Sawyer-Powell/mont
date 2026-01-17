use std::collections::{HashMap, VecDeque};

use crate::graph::{self, TaskGraph};
use crate::task::TaskType;

/// A cell in the display grid.
#[derive(Debug, Clone, PartialEq)]
pub enum Cell {
    /// A task node
    Task(String),
    /// Empty cell
    Empty,
    /// Connection lines with directional flags
    Connection {
        up: bool,
        down: bool,
        left: bool,
        right: bool,
    },
}

impl Cell {
    /// Create an empty connection cell
    pub fn connection() -> Self {
        Cell::Connection {
            up: false,
            down: false,
            left: false,
            right: false,
        }
    }

    /// Check if this is an empty connection (no directions set)
    pub fn is_empty_connection(&self) -> bool {
        matches!(
            self,
            Cell::Connection {
                up: false,
                down: false,
                left: false,
                right: false
            }
        )
    }
}

/// The display grid containing rows of cells.
#[derive(Debug, Clone)]
pub struct Grid {
    pub rows: Vec<Vec<Cell>>,
}

impl Grid {
    /// Create a new empty grid
    pub fn new() -> Self {
        Grid { rows: Vec::new() }
    }

    /// Get the width (number of columns) of the grid
    pub fn width(&self) -> usize {
        self.rows.iter().map(|r| r.len()).max().unwrap_or(0)
    }

    /// Get the height (number of rows) of the grid
    pub fn height(&self) -> usize {
        self.rows.len()
    }

    /// Ensure the grid has at least the specified dimensions
    pub fn ensure_size(&mut self, rows: usize, cols: usize) {
        while self.rows.len() < rows {
            self.rows.push(Vec::new());
        }
        for row in &mut self.rows {
            while row.len() < cols {
                row.push(Cell::Empty);
            }
        }
    }

    /// Get a mutable reference to a cell, expanding the grid if needed
    pub fn get_mut(&mut self, row: usize, col: usize) -> &mut Cell {
        self.ensure_size(row + 1, col + 1);
        &mut self.rows[row][col]
    }

    /// Get a reference to a cell, or None if out of bounds
    pub fn get(&self, row: usize, col: usize) -> Option<&Cell> {
        self.rows.get(row).and_then(|r| r.get(col))
    }
}

impl Default for Grid {
    fn default() -> Self {
        Self::new()
    }
}

/// Position of a task in the logical layout
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub level: usize,
    pub index: usize, // Index within the level (0 = first task at this level)
}

/// Layout information computed from tasks
#[derive(Debug)]
pub struct Layout {
    /// Map from task ID to its position
    pub positions: HashMap<String, Position>,
    /// Tasks organized by level (level 0 = roots/first to display)
    pub levels: Vec<Vec<String>>,
    /// Edges to display (from transitive reduction): (from_id, to_id)
    pub edges: Vec<(String, String)>,
}

/// Compute layout for a TaskGraph.
///
/// Uses transitive reduction to determine display edges.
/// Level assignment uses BFS from sources (tasks with no predecessors).
/// Level = longest path from any source to this task.
///
/// Column assignment aligns tasks with their predecessors:
/// - Single predecessor: use predecessor's column (vertical alignment)
/// - Multiple predecessors: use leftmost predecessor's column
/// - Conflicts resolved by priority (in-progress > bugs > alphabetical)
pub fn compute_layout(graph: &TaskGraph) -> Layout {
    if graph.is_empty() {
        return Layout {
            positions: HashMap::new(),
            levels: Vec::new(),
            edges: Vec::new(),
        };
    }

    // Get transitive reduction edges: from -> [to]
    let effective_successors = graph::transitive_reduction(graph);

    // Build predecessor map (reverse of successors)
    let mut predecessors: HashMap<&str, Vec<&str>> = HashMap::new();
    for id in graph.keys() {
        predecessors.insert(id.as_str(), Vec::new());
    }
    for (from, successors) in &effective_successors {
        for &to in successors {
            predecessors.entry(to).or_default().push(from);
        }
    }

    // Build in-degree map from the effective edges
    let mut in_degree: HashMap<&str, usize> = HashMap::new();
    for id in graph.keys() {
        in_degree.insert(id.as_str(), 0);
    }
    for successors in effective_successors.values() {
        for &succ in successors {
            *in_degree.entry(succ).or_insert(0) += 1;
        }
    }

    // Compute levels using BFS (longest path from sources)
    let mut levels_map: HashMap<&str, usize> = HashMap::new();
    let mut queue: VecDeque<&str> = in_degree
        .iter()
        .filter(|&(_, deg)| *deg == 0)
        .map(|(&id, _)| id)
        .collect();

    for &id in &queue {
        levels_map.insert(id, 0);
    }

    let mut remaining_in_degree = in_degree.clone();

    while let Some(task_id) = queue.pop_front() {
        let task_level = levels_map[task_id];

        if let Some(successors) = effective_successors.get(task_id) {
            for &succ in successors {
                let succ_level = levels_map.entry(succ).or_insert(0);
                *succ_level = (*succ_level).max(task_level + 1);

                if let Some(deg) = remaining_in_degree.get_mut(succ) {
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(succ);
                    }
                }
            }
        }
    }

    // Group tasks by level
    let max_level = levels_map.values().copied().max().unwrap_or(0);
    let mut level_tasks: Vec<Vec<&str>> = vec![Vec::new(); max_level + 1];

    for (&id, &level) in &levels_map {
        level_tasks[level].push(id);
    }

    // Assign columns based on successor alignment (work backwards from last level)
    let mut columns: HashMap<&str, usize> = HashMap::new();

    // Helper to get task priority for sorting (lower = higher priority)
    let task_priority = |id: &str| -> i32 {
        match graph.get(id) {
            Some(t) if t.in_progress.is_some() => -2,
            Some(t) if t.task_type == TaskType::Bug => -1,
            _ => 0,
        }
    };

    // Last level (roots/sinks): assign columns by priority order
    let last_level = max_level;
    level_tasks[last_level].sort_by(|a, b| {
        task_priority(a).cmp(&task_priority(b)).then_with(|| a.cmp(b))
    });
    for (col, &task_id) in level_tasks[last_level].iter().enumerate() {
        columns.insert(task_id, col);
    }

    // Work backwards from second-to-last level to level 0
    // Each task gets the column of its successor (if it has only one)
    for level_idx in (0..last_level).rev() {
        let tasks_at_level = &mut level_tasks[level_idx];

        // Compute preferred column for each task based on successors
        let mut preferred: Vec<(&str, usize)> = tasks_at_level
            .iter()
            .map(|&task_id| {
                let succs = effective_successors
                    .get(task_id)
                    .map(|v| v.as_slice())
                    .unwrap_or(&[]);
                let preferred_col = if succs.is_empty() {
                    // No successors - independent task, use column 0
                    0
                } else {
                    // Use leftmost successor's column
                    succs
                        .iter()
                        .filter_map(|s| columns.get(s))
                        .min()
                        .copied()
                        .unwrap_or(0)
                };
                (task_id, preferred_col)
            })
            .collect();

        // Sort by preferred column, then by priority, then alphabetically
        preferred.sort_by(|a, b| {
            a.1.cmp(&b.1)
                .then_with(|| task_priority(a.0).cmp(&task_priority(b.0)))
                .then_with(|| a.0.cmp(b.0))
        });

        // Assign columns, shifting to resolve conflicts within this level
        let mut used_cols: std::collections::HashSet<usize> = std::collections::HashSet::new();
        let mut assigned: Vec<(&str, usize)> = Vec::new();

        for (task_id, preferred_col) in preferred {
            let mut col = preferred_col;
            while used_cols.contains(&col) {
                col += 1;
            }
            used_cols.insert(col);
            columns.insert(task_id, col);
            assigned.push((task_id, col));
        }

        // Reorder level_tasks to match final column order
        assigned.sort_by_key(|(_, col)| *col);
        *tasks_at_level = assigned.into_iter().map(|(id, _)| id).collect();
    }

    // Build positions map using computed columns
    let mut positions = HashMap::new();
    for (&task_id, &col) in &columns {
        let level = levels_map[task_id];
        positions.insert(task_id.to_string(), Position { level, index: col });
    }

    // Collect edges as (from, to) pairs
    let mut edges: Vec<(String, String)> = Vec::new();
    for (from, successors) in &effective_successors {
        for &to in successors {
            edges.push((from.to_string(), to.to_string()));
        }
    }

    // Convert level_tasks from &str to String
    let levels: Vec<Vec<String>> = level_tasks
        .into_iter()
        .map(|level| level.into_iter().map(String::from).collect())
        .collect();

    Layout { positions, levels, edges }
}

/// Build a skewed grid with one task per row.
///
/// Tasks are placed in level order (from layout.levels), with each task
/// getting its own row. Column comes from the position's index.
/// This produces a grid ready for routing without needing a separate skew step.
pub fn build_grid(layout: &Layout) -> Grid {
    let mut grid = Grid::new();

    if layout.levels.is_empty() {
        return grid;
    }

    // Find max column for grid width
    let max_col = layout.positions.values().map(|p| p.index).max().unwrap_or(0) + 1;

    // Create one row per task, in level order
    for level in &layout.levels {
        for task_id in level {
            if let Some(pos) = layout.positions.get(task_id) {
                let mut row = vec![Cell::Empty; max_col];
                row[pos.index] = Cell::Task(task_id.clone());
                grid.rows.push(row);
            }
        }
    }

    grid
}

/// Skew a grid so each task gets its own row.
///
/// Takes a grid where multiple tasks may share a row (same level)
/// and expands it so each task is on a separate row.
/// Column positions are preserved.
///
/// Example:
/// ```text
/// Before:          After:
/// A . .            A . .
/// P B .            P . .
/// C D E    ->      . B .
/// X . .            C . .
/// Z . .            . D .
///                  . . E
///                  X . .
///                  Z . .
/// ```
pub fn skew_grid(grid: &Grid) -> Grid {
    let mut skewed = Grid::new();
    let width = grid.width();

    for row in &grid.rows {
        // Collect tasks from this row with their column positions
        let tasks: Vec<(usize, &Cell)> = row
            .iter()
            .enumerate()
            .filter(|(_, cell)| matches!(cell, Cell::Task(_)))
            .collect();

        if tasks.is_empty() {
            // Empty row - preserve it
            skewed.rows.push(vec![Cell::Empty; width]);
        } else {
            // Create a separate row for each task
            for (col, cell) in tasks {
                let mut new_row = vec![Cell::Empty; width];
                new_row[col] = cell.clone();
                skewed.rows.push(new_row);
            }
        }
    }

    skewed
}

/// Prune rows that only contain pure vertical connections.
///
/// A row is pruneable if:
/// - It has no Task cells
/// - All Connection cells are pure vertical (up && down && !left && !right)
/// - Empty cells are allowed
///
/// This reduces visual noise without losing information.
pub fn prune_rows(grid: &Grid) -> Grid {
    let mut pruned = Grid::new();

    for row in &grid.rows {
        if is_pruneable_row(row) {
            continue;
        }
        pruned.rows.push(row.clone());
    }

    pruned
}

/// Check if a row contains only pure vertical connections (and empty cells).
fn is_pruneable_row(row: &[Cell]) -> bool {
    let mut has_any_connection = false;

    for cell in row {
        match cell {
            Cell::Task(_) => return false,
            Cell::Empty => {}
            Cell::Connection { up, down, left, right } => {
                // Pure vertical: up and down only, no horizontal
                if *left || *right {
                    return false;
                }
                if *up && *down {
                    has_any_connection = true;
                } else if *up || *down {
                    // Partial vertical (endpoint) - not pruneable
                    return false;
                }
            }
        }
    }

    // Only prune if there's at least one vertical connection
    has_any_connection
}

/// Debug render a grid to ASCII for visual inspection.
/// Each cell is rendered as a single character.
pub fn debug_render_grid(grid: &Grid) -> String {
    let mut output = String::new();

    for row in &grid.rows {
        for cell in row {
            let ch = match cell {
                Cell::Task(id) => id.chars().next().unwrap_or('?'),
                Cell::Empty => '.',
                Cell::Connection { up, down, left, right } => {
                    match (*up, *down, *left, *right) {
                        (true, true, false, false) => '│',
                        (false, false, true, true) => '─',
                        (true, true, false, true) => '├',
                        (true, true, true, false) => '┤',
                        (false, true, false, true) => '┌',
                        (false, true, true, false) => '┐',
                        (true, false, false, true) => '└',
                        (true, false, true, false) => '┘',
                        (true, true, true, true) => '┼',
                        (false, true, true, true) => '┬',  // down+left+right (T-junction)
                        (true, false, true, true) => '┴',  // up+left+right (inverted T)
                        (true, false, false, false) => '╵',
                        (false, true, false, false) => '╷',
                        (false, false, true, false) => '╴',
                        (false, false, false, true) => '╶',
                        _ => ' ',
                    }
                }
            };
            output.push(ch);
        }
        output.push('\n');
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
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

    /// Helper to build a TaskGraph from a list of tasks
    fn build_graph(tasks: Vec<Task>) -> TaskGraph {
        tasks.into_iter().map(|t| (t.id.clone(), t)).collect()
    }

    /// Diamond pattern: P forks to A and B, both merge to R (root)
    ///
    ///   P
    ///  / \
    /// A   B
    ///  \ /
    ///   R
    #[test]
    fn test_diamond() {
        let r = make_task("R");
        let p = make_task_with_parent("P", "R");
        let mut a = make_task_with_parent("A", "R");
        a.preconditions = vec!["P".to_string()];
        let mut b = make_task_with_parent("B", "R");
        b.preconditions = vec!["P".to_string()];

        let graph = build_graph(vec![r, p, a, b]);
        let layout = compute_layout(&graph);
        let grid = build_grid(&layout);

        println!("\n=== Diamond ===");
        println!("{}", debug_render_grid(&grid));

        assert_eq!(layout.levels, vec![vec!["P"], vec!["A", "B"], vec!["R"]]);
    }

    /// Wide merge: 4 children merge to one parent
    ///
    /// A B C D
    ///  \|/|/
    ///   R
    #[test]
    fn test_wide_merge() {
        let r = make_task("R");
        let a = make_task_with_parent("A", "R");
        let b = make_task_with_parent("B", "R");
        let c = make_task_with_parent("C", "R");
        let d = make_task_with_parent("D", "R");

        let graph = build_graph(vec![r, a, b, c, d]);
        let layout = compute_layout(&graph);
        let grid = build_grid(&layout);

        println!("\n=== Wide Merge ===");
        println!("{}", debug_render_grid(&grid));

        assert_eq!(layout.levels, vec![vec!["A", "B", "C", "D"], vec!["R"]]);
    }

    /// Chain: A -> B -> C
    #[test]
    fn test_chain() {
        let c = make_task("C");
        let b = make_task_with_parent("B", "C");
        let a = make_task_with_parent("A", "B");

        let graph = build_graph(vec![a, b, c]);
        let layout = compute_layout(&graph);
        let grid = build_grid(&layout);

        println!("\n=== Chain ===");
        println!("{}", debug_render_grid(&grid));

        assert_eq!(layout.levels, vec![vec!["A"], vec!["B"], vec!["C"]]);
    }

    /// Test build_grid creates skewed output (one task per row) with parallel diamond
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
    #[test]
    fn test_skew_parallel_diamond() {
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

        println!("\n=== Parallel Diamond (build_grid creates skewed) ===");
        println!("{}", debug_render_grid(&grid));

        // build_grid now creates skewed grid directly: 8 rows (one per task)
        assert_eq!(grid.rows.len(), 8);

        // Each row should have exactly one task
        for row in &grid.rows {
            let task_count = row.iter().filter(|c| matches!(c, Cell::Task(_))).count();
            assert_eq!(task_count, 1);
        }
    }
}
