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

    // Assign columns based on predecessor alignment
    let mut columns: HashMap<&str, usize> = HashMap::new();

    // Helper to get task priority for sorting (lower = higher priority)
    let task_priority = |id: &str| -> i32 {
        match graph.get(id) {
            Some(t) if t.in_progress.is_some() => -2,
            Some(t) if t.task_type == TaskType::Bug => -1,
            _ => 0,
        }
    };

    // Level 0: assign columns by priority order
    level_tasks[0].sort_by(|a, b| {
        task_priority(a).cmp(&task_priority(b)).then_with(|| a.cmp(b))
    });
    for (col, &task_id) in level_tasks[0].iter().enumerate() {
        columns.insert(task_id, col);
    }

    // For each subsequent level, assign columns based on predecessors
    for level_idx in 1..=max_level {
        let tasks_at_level = &mut level_tasks[level_idx];

        // Compute preferred column for each task
        let mut preferred: Vec<(&str, usize)> = tasks_at_level
            .iter()
            .map(|&task_id| {
                let preds = predecessors.get(task_id).map(|v| v.as_slice()).unwrap_or(&[]);
                let preferred_col = if preds.is_empty() {
                    0
                } else {
                    // Use leftmost predecessor's column
                    preds
                        .iter()
                        .filter_map(|p| columns.get(p))
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

        // Assign columns, shifting right on conflicts
        let mut used_columns: std::collections::HashSet<usize> = std::collections::HashSet::new();
        let mut assigned: Vec<(&str, usize)> = Vec::new();

        for (task_id, mut col) in preferred {
            while used_columns.contains(&col) {
                col += 1;
            }
            used_columns.insert(col);
            columns.insert(task_id, col);
            assigned.push((task_id, col));
        }

        // Reorder level_tasks to match column order
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

/// Build the initial grid with tasks placed.
///
/// Tasks are placed at their computed positions (level, column).
/// Row = level, Column = computed column from edge-based assignment.
pub fn build_grid(layout: &Layout) -> Grid {
    let mut grid = Grid::new();

    if layout.positions.is_empty() {
        return grid;
    }

    // Find grid dimensions from positions
    let num_rows = layout.positions.values().map(|p| p.level).max().unwrap_or(0) + 1;
    let max_col = layout.positions.values().map(|p| p.index).max().unwrap_or(0) + 1;

    grid.ensure_size(num_rows, max_col);

    // Place tasks at their computed positions
    for (task_id, pos) in &layout.positions {
        grid.rows[pos.level][pos.index] = Cell::Task(task_id.clone());
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
