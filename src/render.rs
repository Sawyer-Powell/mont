use std::collections::HashMap;

use owo_colors::OwoColorize;
use renderdag::{Ancestor, GraphRowRenderer, Renderer};

use crate::graph::{self, TaskGraph};
use crate::task::{Task, TaskType};

type BoxRenderer = renderdag::BoxDrawingRenderer<String, GraphRowRenderer<String>>;

pub const MAX_TITLE_LEN: usize = 60;

pub fn render_task_graph(graph: &TaskGraph, show_completed: bool) -> String {
    if graph.is_empty() {
        return String::new();
    }

    let mut active: TaskGraph = graph
        .iter()
        .filter(|(_, t)| !t.is_gate() && !t.complete)
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    let gates: TaskGraph = graph
        .iter()
        .filter(|(_, t)| t.is_gate())
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    let complete: TaskGraph = graph
        .iter()
        .filter(|(_, t)| !t.is_gate() && t.complete)
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    active.retain(|_, task| !graph::is_group_complete(task, graph));

    let mut output = String::new();

    if !active.is_empty() {
        output.push_str(&render_section(&active));
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
    let components = find_connected_components(graph);
    let mut output = String::new();

    for component in components {
        output.push_str(&render_component(&component));
    }

    output
}

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

    for task in graph.values() {
        let task_idx = id_to_idx[task.id.as_str()];

        for p in &task.before {
            if let Some(&parent_idx) = id_to_idx.get(p.as_str()) {
                union(&mut parent, task_idx, parent_idx);
            }
        }

        for precond in &task.after {
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

    let mut by_root: HashMap<usize, Vec<&str>> = HashMap::new();
    for (i, &id) in task_ids.iter().enumerate() {
        let root = find(&mut parent, i);
        by_root.entry(root).or_default().push(id);
    }

    let mut components: Vec<Vec<&str>> = by_root.into_values().collect();
    components.sort_by(|a, b| {
        let a_min = a.iter().min().unwrap_or(&"");
        let b_min = b.iter().min().unwrap_or(&"");
        a_min.cmp(b_min)
    });

    components
        .into_iter()
        .map(|ids| {
            ids.into_iter()
                .filter_map(|id| graph.get(id).map(|t| (id.to_string(), t.clone())))
                .collect()
        })
        .collect()
}

fn render_component(graph: &TaskGraph) -> String {
    if graph.is_empty() {
        return String::new();
    }

    let effective_successors = graph::transitive_reduction(graph);
    let topo_order = topological_order(graph, &effective_successors);

    let mut renderer: BoxRenderer = GraphRowRenderer::<String>::new()
        .output()
        .with_min_row_height(0)
        .build_box_drawing();

    let mut output = String::new();

    for task_id in topo_order {
        let Some(task) = graph.get(&task_id) else {
            continue;
        };

        let ancestors = build_ancestors(&task_id, &effective_successors);
        let marker = task_marker(task, graph);
        let task_line = format_task_line(task, graph);

        let row = renderer.next_row(task_id.clone(), ancestors, marker, task_line);
        output.push_str(&row);
    }

    output
}

fn topological_order(
    graph: &TaskGraph,
    effective_successors: &HashMap<&str, Vec<&str>>,
) -> Vec<String> {
    let mut in_degree: HashMap<&str, usize> = HashMap::new();
    for id in graph.keys() {
        in_degree.insert(id.as_str(), 0);
    }

    for successors in effective_successors.values() {
        for &succ in successors {
            if let Some(deg) = in_degree.get_mut(succ) {
                *deg += 1;
            }
        }
    }

    let mut queue: Vec<&str> = in_degree
        .iter()
        .filter(|(_, deg)| **deg == 0)
        .map(|(&id, _)| id)
        .collect();
    queue.sort();

    let mut result = Vec::new();
    let mut remaining = in_degree.clone();

    while let Some(task_id) = queue.pop() {
        result.push(task_id.to_string());

        if let Some(successors) = effective_successors.get(task_id) {
            for &succ in successors {
                if let Some(deg) = remaining.get_mut(succ) {
                    *deg -= 1;
                    if *deg == 0 {
                        let pos = queue.partition_point(|&x| x > succ);
                        queue.insert(pos, succ);
                    }
                }
            }
        }
    }

    result
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
    let is_available = !task.complete && !task.is_gate() && graph::is_available(task, graph);
    let is_in_progress = task.in_progress.is_some();
    let is_jot = task.is_jot();

    if task.is_gate() {
        "◈".purple().to_string()
    } else if task.complete {
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
    let is_available = !task.complete && !task.is_gate() && graph::is_available(task, graph);
    let is_in_progress = task.in_progress.is_some();
    let is_jot = task.is_jot();

    let id_display = if task.complete {
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

    let title_formatted = if task.complete {
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

pub fn truncate_title(title: &str) -> String {
    if title.len() <= MAX_TITLE_LEN {
        title.to_string()
    } else {
        format!("{}…", &title[..MAX_TITLE_LEN - 1])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::Task;

    fn make_task(id: &str) -> Task {
        Task {
            id: id.to_string(),
            before: vec![],
            after: vec![],
            validations: vec![],
            title: Some(format!("{} title", id)),
            complete: false,
            in_progress: None,
            task_type: TaskType::Task,
            description: String::new(),
        }
    }

    fn make_task_with_before(id: &str, before_id: &str) -> Task {
        Task {
            id: id.to_string(),
            before: vec![before_id.to_string()],
            after: vec![],
            validations: vec![],
            title: Some(format!("{} title", id)),
            complete: false,
            in_progress: None,
            task_type: TaskType::Task,
            description: String::new(),
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
        let root = make_task("root");

        let mut jot_task = make_task_with_before("jot-task", "root");
        jot_task.task_type = TaskType::Jot;

        let mut in_progress = make_task_with_before("in-progress", "root");
        in_progress.in_progress = Some(1);

        let mut completed = make_task_with_before("completed", "root");
        completed.complete = true;

        let mut gate = make_task("gate-task");
        gate.task_type = TaskType::Gate;

        let graph = build_graph(vec![root, jot_task, in_progress, completed, gate]);
        let output = render_component(&graph);
        let stripped = strip_ansi(&output);

        println!("\n=== Task Types and States ===\n{}", stripped);

        assert!(stripped.contains("[jot]"));
    }
}
