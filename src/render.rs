use std::collections::{HashMap, HashSet};

use owo_colors::OwoColorize;
use renderdag::{Ancestor, GraphRowRenderer, Renderer};

use crate::context::graph;
use crate::{Task, TaskGraph, TaskType, GateStatus};

type BoxRenderer = renderdag::BoxDrawingRenderer<String, GraphRowRenderer<String>>;

pub const MAX_TITLE_LEN: usize = 60;

/// Display state of a task for rendering purposes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DisplayState {
    Complete,
    Gate,
    Jot,
    InProgress,
    Available,
    Waiting,
}

/// Gate progress for a task: (passed_or_skipped, total).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GateProgress {
    pub passed: usize,
    pub total: usize,
}

impl GateProgress {
    pub fn is_complete(&self) -> bool {
        self.total > 0 && self.passed == self.total
    }
}

/// A view of a task optimized for display purposes.
/// Encapsulates all computed display properties.
#[derive(Debug, Clone)]
pub struct TaskDisplayView {
    pub id: String,
    pub title: String,
    pub task_type: TaskType,
    pub state: DisplayState,
    pub gate_progress: Option<GateProgress>,
}

impl TaskDisplayView {
    /// Create a TaskDisplayView from a Task.
    ///
    /// - `graph` is used to determine if the task is available (dependencies complete)
    /// - `default_gates` is used to calculate gate progress for in-progress tasks
    pub fn from_task(task: &Task, graph: &TaskGraph, default_gates: &[String]) -> Self {
        let is_available = !task.is_complete() && !task.is_gate() && graph::is_available(task, graph);

        let state = if task.is_complete() {
            DisplayState::Complete
        } else if task.is_gate() {
            DisplayState::Gate
        } else if task.is_jot() {
            DisplayState::Jot
        } else if task.is_in_progress() {
            DisplayState::InProgress
        } else if is_available {
            DisplayState::Available
        } else {
            DisplayState::Waiting
        };

        let gate_progress = if state == DisplayState::InProgress {
            Some(calculate_gate_progress(task, default_gates))
        } else {
            None
        };

        Self {
            id: task.id.clone(),
            title: task.title.clone().unwrap_or_default(),
            task_type: task.task_type,
            state,
            gate_progress,
        }
    }

    /// Get the type tag for display (e.g., "[task]", "[jot]", "[gate]").
    pub fn type_tag(&self) -> &'static str {
        match self.state {
            DisplayState::Complete => "[done]",
            DisplayState::Gate => "[gate]",
            DisplayState::Jot => "[jot] ",
            DisplayState::InProgress => "[work]",
            DisplayState::Available => "[task]",
            DisplayState::Waiting => "[wait]",
        }
    }

    /// Get the colored type tag for display.
    pub fn type_tag_colored(&self) -> String {
        match self.state {
            DisplayState::Complete => "[done]".bright_black().to_string(),
            DisplayState::Gate => "[gate]".purple().to_string(),
            DisplayState::Jot => "[jot] ".yellow().to_string(),
            DisplayState::InProgress => "[work]".yellow().to_string(),
            DisplayState::Available => "[task]".bright_green().to_string(),
            DisplayState::Waiting => "[wait]".bright_black().to_string(),
        }
    }

    /// Get the colored ID for display.
    pub fn id_colored(&self) -> String {
        match self.state {
            DisplayState::Complete => self.id.bright_black().bold().to_string(),
            DisplayState::Gate => self.id.purple().bold().to_string(),
            DisplayState::Jot | DisplayState::InProgress => self.id.yellow().bold().to_string(),
            DisplayState::Available => self.id.bright_green().bold().to_string(),
            DisplayState::Waiting => self.id.bright_black().bold().to_string(),
        }
    }

    /// Get the colored ID for display, padded to a specific width.
    pub fn id_colored_padded(&self, width: usize) -> String {
        let padded = format!("{:width$}", self.id);
        match self.state {
            DisplayState::Complete => padded.bright_black().bold().to_string(),
            DisplayState::Gate => padded.purple().bold().to_string(),
            DisplayState::Jot | DisplayState::InProgress => padded.yellow().bold().to_string(),
            DisplayState::Available => padded.bright_green().bold().to_string(),
            DisplayState::Waiting => padded.bright_black().bold().to_string(),
        }
    }

    /// Get the colored title for display (truncated to max_len).
    pub fn title_colored(&self, max_len: usize) -> String {
        let truncated = truncate_to(&self.title, max_len);
        match self.state {
            DisplayState::Complete => truncated.bright_black().to_string(),
            DisplayState::Gate => truncated.purple().to_string(),
            DisplayState::Jot | DisplayState::InProgress => truncated.yellow().to_string(),
            DisplayState::Available => truncated.bright_green().to_string(),
            DisplayState::Waiting => truncated.bright_black().to_string(),
        }
    }

    /// Get the gate progress indicator if applicable (e.g., "(1/3)").
    pub fn gate_progress_colored(&self) -> Option<String> {
        let progress = self.gate_progress?;
        if progress.total == 0 {
            return None;
        }

        let progress_str = format!("({}/{})", progress.passed, progress.total);
        Some(if progress.is_complete() {
            progress_str.bright_green().to_string()
        } else {
            progress_str.red().to_string()
        })
    }

    /// Format a single-line display: [type] id title (x/N)
    pub fn format_line(&self, max_title_len: usize) -> String {
        let base = format!(
            "{} {} {}",
            self.type_tag_colored(),
            self.id_colored(),
            self.title_colored(max_title_len)
        );

        match self.gate_progress_colored() {
            Some(progress) => format!("{} {}", base, progress),
            None => base,
        }
    }

    /// Format a single-line display with padded ID: [type]  id  title (x/N)
    pub fn format_line_padded(&self, id_width: usize, max_title_len: usize) -> String {
        let base = format!(
            "{}  {}  {}",
            self.type_tag_colored(),
            self.id_colored_padded(id_width),
            self.title_colored(max_title_len)
        );

        match self.gate_progress_colored() {
            Some(progress) => format!("{} {}", base, progress),
            None => base,
        }
    }

    /// Get the colored status string for display.
    pub fn status_colored(&self) -> String {
        match self.state {
            DisplayState::Complete => "complete".bright_black().to_string(),
            DisplayState::InProgress => "in progress".yellow().to_string(),
            _ => "incomplete".white().to_string(),
        }
    }
}

/// Format a gate status as (icon, colored_gate_id).
pub fn format_gate_status(gate_id: &str, status: GateStatus) -> (String, String) {
    match status {
        GateStatus::Passed => ("✓".bright_green().to_string(), gate_id.bright_black().to_string()),
        GateStatus::Skipped => ("○".bright_black().to_string(), gate_id.bright_black().to_string()),
        GateStatus::Pending => ("•".red().to_string(), gate_id.white().to_string()),
        GateStatus::Failed => ("✗".red().to_string(), gate_id.red().to_string()),
    }
}

/// Print a gates section for a task.
/// Shows all gates (task gates + default gates) with their status.
/// Does nothing for gate-type or jot-type tasks.
pub fn print_gates_section(task: &Task, all_gate_ids: &[String], indent: &str, label_width: usize) {
    if task.is_gate() || task.is_jot() {
        return;
    }

    if all_gate_ids.is_empty() {
        return;
    }

    // Build status map from task's gates
    let gate_status_map: HashMap<&str, GateStatus> = task
        .gates
        .iter()
        .map(|g| (g.id.as_str(), g.status))
        .collect();

    // Gates are already in correct order from all_gate_ids():
    // default gates first (config.yml order), then task-specific gates

    for (i, gate_id) in all_gate_ids.iter().enumerate() {
        let label = if i == 0 { "Gates" } else { "" };
        let status = gate_status_map
            .get(gate_id.as_str())
            .copied()
            .unwrap_or(GateStatus::Pending);
        let (icon, gate_display) = format_gate_status(gate_id, status);
        println!("{}{:label_width$} {} {}", indent, label.bold(), icon, gate_display);
    }
}

/// Get the colored marker icon for a display state.
pub fn task_marker_for_state(state: DisplayState) -> String {
    match state {
        DisplayState::Complete => "●".bright_black().to_string(),
        DisplayState::Gate => "◈".purple().to_string(),
        DisplayState::Jot => "◇".yellow().to_string(),
        DisplayState::InProgress => "◐".yellow().to_string(),
        DisplayState::Available => "◉".bright_green().to_string(),
        DisplayState::Waiting => "○".bright_black().to_string(),
    }
}

/// Calculate gate progress for a task.
/// Returns GateProgress with (passed_or_skipped_count, total_gates_count).
fn calculate_gate_progress(task: &Task, default_gates: &[String]) -> GateProgress {
    // Collect all gates: task gates + default gates
    let task_gate_ids: HashSet<&str> = task.gate_ids().collect();
    let default_gate_ids: HashSet<&str> = default_gates.iter().map(|s| s.as_str()).collect();
    let all_gates: HashSet<&str> = task_gate_ids.union(&default_gate_ids).copied().collect();

    let total = all_gates.len();
    if total == 0 {
        return GateProgress { passed: 0, total: 0 };
    }

    // Count passed or skipped from task's validations
    let passed = task
        .gates
        .iter()
        .filter(|v| {
            all_gates.contains(v.id.as_str())
                && matches!(v.status, GateStatus::Passed | GateStatus::Skipped)
        })
        .count();

    GateProgress { passed, total }
}

pub fn render_task_graph(graph: &TaskGraph, default_gates: &[String], show_completed: bool) -> String {
    if graph.is_empty() {
        return String::new();
    }

    // Active tasks (not jots, not gates, not complete)
    let mut active: TaskGraph = graph
        .iter()
        .filter(|(_, t)| !t.is_gate() && !t.is_jot() && !t.is_complete())
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    // Standalone jots (jots not connected to other tasks)
    let jots: TaskGraph = graph
        .iter()
        .filter(|(_, t)| t.is_jot() && !t.is_complete())
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    let gates: TaskGraph = graph
        .iter()
        .filter(|(_, t)| t.is_gate())
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    let complete: TaskGraph = graph
        .iter()
        .filter(|(_, t)| !t.is_gate() && t.is_complete())
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    active.retain(|_, task| !graph::is_group_complete(task, graph));

    let mut output = String::new();

    if !active.is_empty() {
        output.push_str(&render_section(&active, default_gates));
    }

    if !jots.is_empty() {
        if !output.is_empty() {
            output.push('\n');
        }
        output.push_str(&render_section(&jots, default_gates));
    }

    if !gates.is_empty() {
        if !output.is_empty() {
            output.push('\n');
        }
        output.push_str(&render_section(&gates, default_gates));
    }

    if show_completed && !complete.is_empty() {
        if !output.is_empty() {
            output.push('\n');
        }
        output.push_str(&render_section(&complete, default_gates));
    }

    output
}

fn render_section(graph: &TaskGraph, default_gates: &[String]) -> String {
    let components = graph.connected_components();
    let mut output = String::new();
    let mut prev_was_multi = false;

    for component_ids in components {
        // Build sub-graph for this component
        let component: TaskGraph = component_ids
            .iter()
            .filter_map(|&id| graph.get(id).cloned())
            .collect();

        let is_multi = component.len() > 1;

        // Add blank line before multi-task groups, or after previous multi-task group
        if !output.is_empty() && (is_multi || prev_was_multi) {
            output.push('\n');
        }

        output.push_str(&render_component(&component, default_gates));
        prev_was_multi = is_multi;
    }

    output
}

fn render_component(graph: &TaskGraph, default_gates: &[String]) -> String {
    if graph.is_empty() {
        return String::new();
    }

    let effective_successors = graph.transitive_reduction();
    let topo_order = graph.topological_order();

    let mut renderer: BoxRenderer = GraphRowRenderer::<String>::new()
        .output()
        .with_min_row_height(0)
        .build_box_drawing();

    let mut output = String::new();

    for task_id in topo_order {
        let Some(task) = graph.get(task_id) else {
            continue;
        };

        let ancestors = build_ancestors(task_id, &effective_successors);
        let marker = task_marker(task, graph);
        let task_line = format_task_line(task, graph, default_gates);

        let row = renderer.next_row(task_id.to_string(), ancestors, marker, task_line);
        output.push_str(&row);
    }

    output
}

fn build_ancestors(task_id: &str, effective_successors: &HashMap<&str, Vec<&str>>) -> Vec<Ancestor<String>> {
    effective_successors
        .get(task_id)
        .map(|succs| {
            succs
                .iter()
                .map(|&s| Ancestor::Parent(s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

pub fn task_marker(task: &Task, graph: &TaskGraph) -> String {
    let is_available = !task.is_complete() && !task.is_gate() && graph::is_available(task, graph);
    let is_in_progress = task.is_in_progress();
    let is_jot = task.is_jot();

    if task.is_gate() {
        "◈".purple().to_string()
    } else if task.is_complete() {
        "●".bright_black().to_string()
    } else if is_in_progress {
        "◐".yellow().to_string()
    } else if is_jot {
        "◇".yellow().to_string()
    } else if is_available {
        "◉".bright_green().to_string()
    } else {
        "○".bright_black().to_string()
    }
}

pub fn format_task_line(task: &Task, graph: &TaskGraph, default_gates: &[String]) -> String {
    let view = TaskDisplayView::from_task(task, graph, default_gates);
    view.format_line(MAX_TITLE_LEN)
}

pub fn format_task_line_short(task: &Task, graph: &TaskGraph) -> String {
    let view = TaskDisplayView::from_task(task, graph, &[]);
    view.format_line(MAX_TITLE_LEN)
}

/// Truncate a title to fit within MAX_TITLE_LEN.
pub fn truncate_title(title: &str) -> String {
    truncate_to(title, MAX_TITLE_LEN)
}

/// Truncate a title to fit within the specified length.
/// Trims trailing whitespace before adding ellipsis.
pub fn truncate_to(title: &str, max_len: usize) -> String {
    if title.len() <= max_len {
        title.to_string()
    } else {
        let truncated = &title[..max_len - 1];
        let trimmed = truncated.trim_end();
        format!("{}…", trimmed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Task, TaskType};

    fn make_task(id: &str) -> Task {
        Task {
            id: id.to_string(),
            new_id: None,
            before: vec![],
            after: vec![],
            gates: vec![],
            title: Some(format!("{} title", id)),
            status: None,
            task_type: TaskType::Task,
            description: String::new(),
            deleted: false,
        }
    }

    fn make_task_with_before(id: &str, before_id: &str) -> Task {
        Task {
            id: id.to_string(),
            new_id: None,
            before: vec![before_id.to_string()],
            after: vec![],
            gates: vec![],
            title: Some(format!("{} title", id)),
            status: None,
            task_type: TaskType::Task,
            description: String::new(),
            deleted: false,
        }
    }

    fn build_graph(tasks: Vec<Task>) -> TaskGraph {
        tasks.into_iter().map(|t| (t.id.clone(), t)).collect()
    }

    fn strip_ansi(s: &str) -> String {
        let re = regex::Regex::new(r"\x1b\[[0-9;]*m").unwrap();
        re.replace_all(s, "").to_string()
    }

    #[test]
    fn test_render_chain() {
        let c = make_task("C");
        let b = make_task_with_before("B", "C");
        let a = make_task_with_before("A", "B");

        let graph = build_graph(vec![a, b, c]);
        let output = render_component(&graph, &Vec::<String>::new());
        let stripped = strip_ansi(&output);

        println!("\n=== Chain ===\n{}", stripped);

        assert!(stripped.contains("A"));
        assert!(stripped.contains("B"));
        assert!(stripped.contains("C"));
    }

    #[test]
    fn test_render_diamond() {
        let r = make_task("R");
        let p = make_task_with_before("P", "R");
        let mut a = make_task_with_before("A", "R");
        a.after = vec!["P".to_string()];
        let mut b = make_task_with_before("B", "R");
        b.after = vec!["P".to_string()];

        let graph = build_graph(vec![r, p, a, b]);
        let output = render_component(&graph, &Vec::<String>::new());
        let stripped = strip_ansi(&output);

        println!("\n=== Diamond ===\n{}", stripped);

        assert!(stripped.contains("P"));
        assert!(stripped.contains("A"));
        assert!(stripped.contains("B"));
        assert!(stripped.contains("R"));
    }

    #[test]
    fn test_render_parallel_diamond() {
        let z = make_task("Z");

        let mut x = make_task_with_before("X", "Z");
        x.after = vec!["C".to_string(), "D".to_string()];

        let mut c = make_task_with_before("C", "Z");
        c.after = vec!["P".to_string()];
        let mut d = make_task_with_before("D", "Z");
        d.after = vec!["P".to_string()];

        let mut p = make_task_with_before("P", "Z");
        p.after = vec!["A".to_string()];

        let mut b = make_task_with_before("B", "Z");
        b.after = vec!["A".to_string()];
        let mut e = make_task_with_before("E", "Z");
        e.after = vec!["B".to_string()];

        let a = make_task_with_before("A", "Z");

        let graph = build_graph(vec![a, b, c, d, e, p, x, z]);
        let output = render_component(&graph, &Vec::<String>::new());
        let stripped = strip_ansi(&output);

        println!("\n=== Parallel Diamond ===\n{}", stripped);

        assert!(stripped.contains("A"));
        assert!(stripped.contains("B"));
        assert!(stripped.contains("C"));
        assert!(stripped.contains("D"));
        assert!(stripped.contains("E"));
        assert!(stripped.contains("P"));
        assert!(stripped.contains("X"));
        assert!(stripped.contains("Z"));
    }

    #[test]
    fn test_render_with_task_types_and_states() {
        use crate::Status;

        let root = make_task("root");

        let mut jot_task = make_task_with_before("jot-task", "root");
        jot_task.task_type = TaskType::Jot;

        let mut in_progress = make_task_with_before("in-progress", "root");
        in_progress.status = Some(Status::InProgress);

        let mut completed = make_task_with_before("completed", "root");
        completed.status = Some(Status::Complete);

        let mut gate = make_task("gate-task");
        gate.task_type = TaskType::Gate;

        let graph = build_graph(vec![root, jot_task, in_progress, completed, gate]);
        let output = render_component(&graph, &Vec::<String>::new());
        let stripped = strip_ansi(&output);

        println!("\n=== Task Types and States ===\n{}", stripped);

        assert!(stripped.contains("[jot]"));
    }
}
