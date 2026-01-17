use std::collections::HashMap;

use owo_colors::OwoColorize;

use super::layout::{build_grid, compute_layout, prune_rows, Cell, Grid};
use super::routing::route_edges;
use crate::graph::{self, TaskGraph};
use crate::task::{Task, TaskType};

/// Maximum title length before truncation.
pub const MAX_TITLE_LEN: usize = 60;

/// Render a task graph to a displayable string with sections.
///
/// Sections are rendered in order:
/// 1. Active tasks (incomplete, non-validators)
/// 2. Validator tasks
/// 3. Complete tasks (if show_completed is true)
///
/// Each section is separated by a blank line.
/// Within each section, connected components are rendered separately.
pub fn render_task_graph(graph: &TaskGraph, show_completed: bool) -> String {
    if graph.is_empty() {
        return String::new();
    }

    // Separate into sections
    let mut active: TaskGraph = graph
        .iter()
        .filter(|(_, t)| !t.validator && !t.complete)
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    let validators: TaskGraph = graph
        .iter()
        .filter(|(_, t)| t.validator)
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    let complete: TaskGraph = graph
        .iter()
        .filter(|(_, t)| !t.validator && t.complete)
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    // Filter out tasks whose entire subtree is complete (group complete)
    active.retain(|_, task| !graph::is_group_complete(task, graph));

    let mut output = String::new();

    // Render active section (by components)
    if !active.is_empty() {
        output.push_str(&render_section_by_components(&active));
    }

    // Render validators section (by components)
    if !validators.is_empty() {
        if !output.is_empty() {
            output.push('\n');
        }
        output.push_str(&render_section_by_components(&validators));
    }

    // Render complete section (by components)
    if show_completed && !complete.is_empty() {
        if !output.is_empty() {
            output.push('\n');
        }
        output.push_str(&render_section_by_components(&complete));
    }

    output
}

/// Render a section by splitting into connected components and rendering each separately.
fn render_section_by_components(graph: &TaskGraph) -> String {
    let components = find_connected_components(graph);
    let mut output = String::new();

    for component in components {
        if !output.is_empty() {
            // No blank line between components in same section
        }
        output.push_str(&render_grid(&component));
    }

    output
}

/// Find connected components in a task graph using union-find.
fn find_connected_components(graph: &TaskGraph) -> Vec<TaskGraph> {
    if graph.is_empty() {
        return Vec::new();
    }

    let task_ids: Vec<&str> = graph.keys().map(|s| s.as_str()).collect();
    let id_to_idx: HashMap<&str, usize> = task_ids
        .iter()
        .enumerate()
        .map(|(i, &id)| (id, i))
        .collect();

    let mut parent: Vec<usize> = (0..task_ids.len()).collect();

    fn find(parent: &mut [usize], i: usize) -> usize {
        if parent[i] != i {
            parent[i] = find(parent, parent[i]);
        }
        parent[i]
    }

    fn union(parent: &mut [usize], i: usize, j: usize) {
        let pi = find(parent, i);
        let pj = find(parent, j);
        if pi != pj {
            parent[pi] = pj;
        }
    }

    // Union tasks that are connected via parent or preconditions
    for task in graph.values() {
        let task_idx = id_to_idx[task.id.as_str()];

        if let Some(p) = &task.parent {
            if let Some(&parent_idx) = id_to_idx.get(p.as_str()) {
                union(&mut parent, task_idx, parent_idx);
            }
        }

        for precond in &task.preconditions {
            if let Some(&precond_idx) = id_to_idx.get(precond.as_str()) {
                union(&mut parent, task_idx, precond_idx);
            }
        }

        for validation in &task.validations {
            if let Some(&val_idx) = id_to_idx.get(validation.id.as_str()) {
                union(&mut parent, task_idx, val_idx);
            }
        }
    }

    // Group tasks by their root
    let mut by_root: HashMap<usize, Vec<&str>> = HashMap::new();
    for (i, &id) in task_ids.iter().enumerate() {
        let root = find(&mut parent, i);
        by_root.entry(root).or_default().push(id);
    }

    // Sort components for deterministic output (by first task ID alphabetically)
    let mut components: Vec<Vec<&str>> = by_root.into_values().collect();
    components.sort_by(|a, b| {
        let a_min = a.iter().min().unwrap_or(&"");
        let b_min = b.iter().min().unwrap_or(&"");
        a_min.cmp(b_min)
    });

    // Build TaskGraph for each component
    components
        .into_iter()
        .map(|ids| {
            ids.into_iter()
                .filter_map(|id| graph.get(id).map(|t| (id.to_string(), t.clone())))
                .collect()
        })
        .collect()
}

/// Render a task graph to a displayable string.
///
/// Pipeline:
/// 1. layout → build_grid (one task per row, columns based on successors)
/// 2. route_edges (add connections)
/// 3. prune_rows (remove pure vertical rows)
/// 4. render
pub fn render_grid(graph: &TaskGraph) -> String {
    if graph.is_empty() {
        return String::new();
    }

    let layout = compute_layout(graph);
    let grid = build_grid(&layout);
    let routed = route_edges(&grid, &layout);
    let pruned = prune_rows(&routed);

    render_grid_to_string(&pruned, graph)
}

/// Render a processed grid to a string.
fn render_grid_to_string(grid: &Grid, graph: &TaskGraph) -> String {
    let mut output = String::new();

    for row in &grid.rows {
        let line = render_row(row, graph);
        output.push_str(&line);
        output.push('\n');
    }

    output
}

/// Render a single row to a string.
///
/// Processes cells left-to-right. Tasks render their full line (marker + id + title).
/// Connections that would overlap with existing task output are skipped.
fn render_row(row: &[Cell], graph: &TaskGraph) -> String {
    let mut output = String::new();
    let mut display_width = 0usize; // Track display width, not byte length

    for (col, cell) in row.iter().enumerate() {
        let target_pos = col * 2;

        // Skip if we already have content at this position
        if display_width > target_pos {
            continue;
        }

        // Pad to target position if needed
        while display_width < target_pos {
            output.push(' ');
            display_width += 1;
        }

        match cell {
            Cell::Task(id) => {
                if let Some(task) = graph.get(id) {
                    let task_line = render_task_line(task, graph);
                    // Task lines are long - they extend past the grid
                    // Set display_width to a large value to skip remaining cells
                    display_width = usize::MAX;
                    output.push_str(&task_line);
                } else {
                    // Fallback if task not found
                    output.push_str(&format!("? {}", id));
                    display_width = usize::MAX;
                }
            }
            Cell::Connection { .. } => {
                output.push_str(connection_symbol(cell));
                display_width += 2; // Connection symbols are 2 display columns
            }
            Cell::Empty => {
                output.push_str("  ");
                display_width += 2;
            }
        }
    }

    // Trim trailing whitespace
    output.trim_end().to_string()
}

/// Render a task line with marker, colored ID, and formatted title.
fn render_task_line(task: &Task, graph: &TaskGraph) -> String {
    let is_available = !task.complete && !task.validator && graph::is_available(task, graph);
    let is_in_progress = task.in_progress.is_some();
    let is_bug = task.task_type == TaskType::Bug;
    let is_epic = task.task_type == TaskType::Epic;

    let marker = if task.validator {
        "◈".purple().to_string()
    } else if task.complete {
        "●".bright_black().to_string()
    } else if is_in_progress {
        "◐".yellow().to_string()
    } else if is_available && is_bug {
        "◉".red().to_string()
    } else if is_available && is_epic {
        "◉".cyan().to_string()
    } else if is_available {
        "◉".bright_green().to_string()
    } else {
        "○".bright_black().to_string()
    };

    let id_display = if task.complete {
        task.id.bright_black().bold().to_string()
    } else if task.validator {
        task.id.purple().bold().to_string()
    } else if is_in_progress {
        task.id.yellow().bold().to_string()
    } else if is_available && is_bug {
        task.id.red().bold().to_string()
    } else if is_available && is_epic {
        task.id.cyan().bold().to_string()
    } else if is_available {
        task.id.bright_green().bold().to_string()
    } else {
        task.id.bright_black().bold().to_string()
    };

    let title_raw = task.title.as_deref().unwrap_or("");
    let title_truncated = truncate_title(title_raw);

    let type_suffix = match task.task_type {
        TaskType::Bug => {
            if is_available {
                format!(" {}", "[bug]".red())
            } else {
                format!(" {}", "[bug]".bright_black())
            }
        }
        TaskType::Epic => {
            if is_available {
                format!(" {}", "[epic]".cyan())
            } else {
                format!(" {}", "[epic]".bright_black())
            }
        }
        TaskType::Feature => String::new(),
    };

    let title_formatted = if task.complete {
        format!("{}{}", title_truncated.bright_black(), type_suffix)
    } else if task.validator {
        format!("{} {}{}", title_truncated, "[validator]".purple(), type_suffix)
    } else if is_in_progress {
        format!("{}{}", title_truncated.yellow(), type_suffix)
    } else if is_available && is_bug {
        format!("{}{}", title_truncated.red(), type_suffix)
    } else if is_available && is_epic {
        format!("{}{}", title_truncated.cyan(), type_suffix)
    } else if is_available {
        format!("{}{}", title_truncated.bright_green(), type_suffix)
    } else {
        format!("{}{}", title_truncated.bright_black(), type_suffix)
    };

    format!("{} {} {}", marker, id_display, title_formatted)
}

/// Truncate a title to MAX_TITLE_LEN characters.
pub fn truncate_title(title: &str) -> String {
    if title.len() <= MAX_TITLE_LEN {
        title.to_string()
    } else {
        format!("{}…", &title[..MAX_TITLE_LEN - 1])
    }
}

/// Convert a Connection cell to its ASCII symbol representation.
///
/// Returns a 2-char string (symbol + space) to maintain column alignment.
/// Uses rounded corners for a softer appearance.
///
/// Symbol mapping:
/// ```text
/// {up, down}              -> │
/// {left, right}           -> ─
/// {up, down, right}       -> ├
/// {up, down, left}        -> ┤
/// {down, right}           -> ╭
/// {down, left}            -> ╮
/// {up, right}             -> ╰
/// {up, left}              -> ╯
/// {up, down, left, right} -> ┼
/// ```
pub fn connection_symbol(cell: &Cell) -> &'static str {
    match cell {
        Cell::Connection {
            up,
            down,
            left,
            right,
        } => match (*up, *down, *left, *right) {
            // Vertical
            (true, true, false, false) => "│ ",
            // Horizontal
            (false, false, true, true) => "──",
            // T-junctions
            (true, true, false, true) => "├─",
            (true, true, true, false) => "┤ ",
            (false, true, true, true) => "┬─",
            (true, false, true, true) => "┴─",
            // Corners (rounded)
            (false, true, false, true) => "╭─",
            (false, true, true, false) => "╮ ",
            (true, false, false, true) => "╰─",
            (true, false, true, false) => "╯ ",
            // Cross
            (true, true, true, true) => "┼─",
            // Half-lines (endpoints)
            (true, false, false, false) => "╵ ",
            (false, true, false, false) => "╷ ",
            (false, false, true, false) => "╴ ",
            (false, false, false, true) => "╶─",
            // Empty connection
            (false, false, false, false) => "  ",
        },
        Cell::Task(_) | Cell::Empty => "  ",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn conn(up: bool, down: bool, left: bool, right: bool) -> Cell {
        Cell::Connection {
            up,
            down,
            left,
            right,
        }
    }

    #[test]
    fn test_vertical_line() {
        assert_eq!(connection_symbol(&conn(true, true, false, false)), "│ ");
    }

    #[test]
    fn test_horizontal_line() {
        assert_eq!(connection_symbol(&conn(false, false, true, true)), "──");
    }

    #[test]
    fn test_t_junctions() {
        assert_eq!(connection_symbol(&conn(true, true, false, true)), "├─");
        assert_eq!(connection_symbol(&conn(true, true, true, false)), "┤ ");
        assert_eq!(connection_symbol(&conn(false, true, true, true)), "┬─");
        assert_eq!(connection_symbol(&conn(true, false, true, true)), "┴─");
    }

    #[test]
    fn test_rounded_corners() {
        assert_eq!(connection_symbol(&conn(false, true, false, true)), "╭─");
        assert_eq!(connection_symbol(&conn(false, true, true, false)), "╮ ");
        assert_eq!(connection_symbol(&conn(true, false, false, true)), "╰─");
        assert_eq!(connection_symbol(&conn(true, false, true, false)), "╯ ");
    }

    #[test]
    fn test_cross() {
        assert_eq!(connection_symbol(&conn(true, true, true, true)), "┼─");
    }

    #[test]
    fn test_endpoints() {
        assert_eq!(connection_symbol(&conn(true, false, false, false)), "╵ ");
        assert_eq!(connection_symbol(&conn(false, true, false, false)), "╷ ");
        assert_eq!(connection_symbol(&conn(false, false, true, false)), "╴ ");
        assert_eq!(connection_symbol(&conn(false, false, false, true)), "╶─");
    }

    #[test]
    fn test_empty_connection() {
        assert_eq!(connection_symbol(&conn(false, false, false, false)), "  ");
    }

    #[test]
    fn test_task_and_empty_cells() {
        assert_eq!(connection_symbol(&Cell::Task("test".to_string())), "  ");
        assert_eq!(connection_symbol(&Cell::Empty), "  ");
    }

    use crate::task::Task;

    fn make_task(id: &str) -> Task {
        Task {
            id: id.to_string(),
            parent: None,
            preconditions: vec![],
            validations: vec![],
            title: Some(format!("{} title", id)),
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
            title: Some(format!("{} title", id)),
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

    /// Strip ANSI escape codes for comparison
    fn strip_ansi(s: &str) -> String {
        let re = regex::Regex::new(r"\x1b\[[0-9;]*m").unwrap();
        re.replace_all(s, "").to_string()
    }

    #[test]
    fn test_render_parallel_diamond() {
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
        let output = render_grid(&graph);
        let stripped = strip_ansi(&output);

        println!("\n=== Render Parallel Diamond ===");
        println!("{}", stripped);

        // Verify all 8 tasks are present
        assert!(stripped.contains("A A title"));
        assert!(stripped.contains("B B title"));
        assert!(stripped.contains("C C title"));
        assert!(stripped.contains("D D title"));
        assert!(stripped.contains("E E title"));
        assert!(stripped.contains("P P title"));
        assert!(stripped.contains("X X title"));
        assert!(stripped.contains("Z Z title"));
    }

    #[test]
    fn test_render_with_task_types_and_states() {
        // Test various task states and types
        let mut root = make_task("root");
        root.parent = None;

        let mut bug_task = make_task_with_parent("bug-task", "root");
        bug_task.task_type = TaskType::Bug;

        let mut epic_task = make_task_with_parent("epic-task", "root");
        epic_task.task_type = TaskType::Epic;

        let mut in_progress = make_task_with_parent("in-progress", "root");
        in_progress.in_progress = Some(1);

        let mut completed = make_task_with_parent("completed", "root");
        completed.complete = true;

        let mut validator = make_task("validator-task");
        validator.validator = true;

        let graph = build_graph(vec![root, bug_task, epic_task, in_progress, completed, validator]);
        let output = render_grid(&graph);
        let stripped = strip_ansi(&output);

        println!("\n=== Task Types and States (stripped) ===");
        println!("{}", stripped);

        println!("\n=== Task Types and States (with colors) ===");
        println!("{}", output);

        // Verify type suffixes
        assert!(stripped.contains("[bug]"), "should have [bug] suffix");
        assert!(stripped.contains("[epic]"), "should have [epic] suffix");
        assert!(stripped.contains("[validator]"), "should have [validator] suffix");

        // Verify markers are present
        assert!(stripped.contains("◉"), "should have available marker");
        assert!(stripped.contains("○"), "should have blocked marker");
        assert!(stripped.contains("●"), "should have complete marker");
        assert!(stripped.contains("◐"), "should have in-progress marker");
        assert!(stripped.contains("◈"), "should have validator marker");
    }
}
