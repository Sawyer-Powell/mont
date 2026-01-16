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
/// Within each level, tasks are ordered by priority:
/// 1. In-progress tasks (highest)
/// 2. Bug tasks
/// 3. Regular tasks
/// 4. Alphabetical by ID (tiebreaker)
pub fn compute_layout(graph: &TaskGraph) -> Layout {
    if graph.is_empty() {
        return Layout {
            positions: HashMap::new(),
            levels: Vec::new(),
            edges: Vec::new(),
        };
    }

    // Get transitive reduction edges: from -> [to]
    // These are the edges we'll actually display
    let effective_successors = graph::transitive_reduction(graph);

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

    // Initialize sources at level 0
    for &id in &queue {
        levels_map.insert(id, 0);
    }

    // Track remaining in-degrees for BFS
    let mut remaining_in_degree = in_degree.clone();

    while let Some(task_id) = queue.pop_front() {
        let task_level = levels_map[task_id];

        if let Some(successors) = effective_successors.get(task_id) {
            for &succ in successors {
                // Update level to be at least task_level + 1
                let succ_level = levels_map.entry(succ).or_insert(0);
                *succ_level = (*succ_level).max(task_level + 1);

                // Decrement in-degree and add to queue when ready
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

    // Sort tasks within each level by priority
    for level in &mut level_tasks {
        level.sort_by(|a, b| {
            let task_a = graph.get(*a);
            let task_b = graph.get(*b);

            // Priority: in-progress (2) > bug (1) > other (0)
            let priority_a = match task_a {
                Some(t) if t.in_progress.is_some() => 2,
                Some(t) if t.task_type == TaskType::Bug => 1,
                _ => 0,
            };
            let priority_b = match task_b {
                Some(t) if t.in_progress.is_some() => 2,
                Some(t) if t.task_type == TaskType::Bug => 1,
                _ => 0,
            };

            // Higher priority first, then alphabetical
            priority_b.cmp(&priority_a).then_with(|| a.cmp(b))
        });
    }

    // Build positions map
    let mut positions = HashMap::new();
    for (level, tasks_at_level) in level_tasks.iter().enumerate() {
        for (index, &task_id) in tasks_at_level.iter().enumerate() {
            positions.insert(
                task_id.to_string(),
                Position { level, index },
            );
        }
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

/// Build the initial grid with tasks placed.
///
/// Tasks at the same level are placed on the same row, in different columns.
/// Row = level, Column = index within level.
/// This will be skewed later during rendering (each task needs its own output line).
pub fn build_grid(layout: &Layout) -> Grid {
    let mut grid = Grid::new();

    if layout.levels.is_empty() {
        return grid;
    }

    // Rows = number of levels, Columns = max tasks at any level
    let num_rows = layout.levels.len();
    let max_width = layout.levels.iter().map(|l| l.len()).max().unwrap_or(0);

    grid.ensure_size(num_rows, max_width);

    // Place tasks: row = level, col = index within level
    for (level_idx, level) in layout.levels.iter().enumerate() {
        for (col, task_id) in level.iter().enumerate() {
            grid.rows[level_idx][col] = Cell::Task(task_id.clone());
        }
    }

    grid
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
}
