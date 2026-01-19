use std::collections::HashMap;

use owo_colors::OwoColorize;
use renderdag::{Ancestor, GraphRowRenderer, Renderer};

use crate::context::graph;
use crate::{Task, TaskGraph, TaskType};

type BoxRenderer = renderdag::BoxDrawingRenderer<String, GraphRowRenderer<String>>;

pub const MAX_TITLE_LEN: usize = 60;

pub fn render_task_graph(graph: &TaskGraph, show_completed: bool) -> String {
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
        output.push_str(&render_section(&active));
    }

    if !jots.is_empty() {
        if !output.is_empty() {
            output.push('\n');
        }
        output.push_str(&render_section(&jots));
    }

    if !gates.is_empty() {
        if !output.is_empty() {
            output.push('\n');
        }
        output.push_str(&render_section(&gates));
    }

    if show_completed && !complete.is_empty() {
        if !output.is_empty() {
            output.push('\n');
        }
        output.push_str(&render_section(&complete));
    }

    output
}

fn render_section(graph: &TaskGraph) -> String {
    let components = graph.connected_components();
    let mut output = String::new();

    for component_ids in components {
        // Build sub-graph for this component
        let component: TaskGraph = component_ids
            .iter()
            .filter_map(|&id| graph.get(id).cloned())
            .collect();

        output.push_str(&render_component(&component));
    }

    output
}

fn render_component(graph: &TaskGraph) -> String {
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
        let task_line = format_task_line(task, graph);

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

pub fn format_task_line(task: &Task, graph: &TaskGraph) -> String {
    format_task_line_impl(task, graph, true)
}

pub fn format_task_line_short(task: &Task, graph: &TaskGraph) -> String {
    format_task_line_impl(task, graph, false)
}

fn format_task_line_impl(task: &Task, graph: &TaskGraph, show_gate_suffix: bool) -> String {
    let is_available = !task.is_complete() && !task.is_gate() && graph::is_available(task, graph);
    let is_in_progress = task.is_in_progress();
    let is_jot = task.is_jot();

    let id_display = if task.is_complete() {
        task.id.bright_black().bold().to_string()
    } else if task.is_gate() {
        task.id.purple().bold().to_string()
    } else if is_jot || is_in_progress {
        task.id.yellow().bold().to_string()
    } else if is_available {
        task.id.bright_green().bold().to_string()
    } else {
        task.id.bright_black().bold().to_string()
    };

    let title_raw = task.title.as_deref().unwrap_or("");
    let title_truncated = truncate_title(title_raw);

    let type_suffix = match task.task_type {
        TaskType::Jot => format!(" {}", "[jot]".yellow()),
        TaskType::Task => String::new(),
        TaskType::Gate => String::new(),
    };

    let title_formatted = if task.is_complete() {
        format!("{}{}", title_truncated.bright_black(), type_suffix)
    } else if task.is_gate() {
        if show_gate_suffix {
            format!("{} {}{}", title_truncated, "[gate]".purple(), type_suffix)
        } else {
            format!("{}{}", title_truncated.purple(), type_suffix)
        }
    } else if is_jot || is_in_progress {
        format!("{}{}", title_truncated.yellow(), type_suffix)
    } else if is_available {
        format!("{}{}", title_truncated.bright_green(), type_suffix)
    } else {
        format!("{}{}", title_truncated.bright_black(), type_suffix)
    };

    format!("{} {}", id_display, title_formatted)
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
    use crate::Task;

    fn make_task(id: &str) -> Task {
        Task {
            id: id.to_string(),
            before: vec![],
            after: vec![],
            validations: vec![],
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
            before: vec![before_id.to_string()],
            after: vec![],
            validations: vec![],
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
        let output = render_component(&graph);
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
        let output = render_component(&graph);
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
        let output = render_component(&graph);
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
        let output = render_component(&graph);
        let stripped = strip_ansi(&output);

        println!("\n=== Task Types and States ===\n{}", stripped);

        assert!(stripped.contains("[jot]"));
    }
}
