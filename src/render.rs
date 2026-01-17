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

    active.retain(|_, task| !graph::is_group_complete(task, graph));

    let mut output = String::new();

    if !active.is_empty() {
        output.push_str(&render_section(&active));
    }

    if !validators.is_empty() {
        if !output.is_empty() {
            output.push('\n');
        }
        output.push_str(&render_section(&validators));
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

fn task_marker(task: &Task, graph: &TaskGraph) -> String {
    let is_available = !task.complete && !task.validator && graph::is_available(task, graph);
    let is_in_progress = task.in_progress.is_some();
    let is_bug = task.task_type == TaskType::Bug;
    let is_epic = task.task_type == TaskType::Epic;

    if task.validator {
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
    }
}

fn format_task_line(task: &Task, graph: &TaskGraph) -> String {
    let is_available = !task.complete && !task.validator && graph::is_available(task, graph);
    let is_in_progress = task.in_progress.is_some();
    let is_bug = task.task_type == TaskType::Bug;
    let is_epic = task.task_type == TaskType::Epic;

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

    fn strip_ansi(s: &str) -> String {
        let re = regex::Regex::new(r"\x1b\[[0-9;]*m").unwrap();
        re.replace_all(s, "").to_string()
    }

    #[test]
    fn test_render_chain() {
        let c = make_task("C");
        let b = make_task_with_parent("B", "C");
        let a = make_task_with_parent("A", "B");

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
        let p = make_task_with_parent("P", "R");
        let mut a = make_task_with_parent("A", "R");
        a.preconditions = vec!["P".to_string()];
        let mut b = make_task_with_parent("B", "R");
        b.preconditions = vec!["P".to_string()];

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

        let mut x = make_task_with_parent("X", "Z");
        x.preconditions = vec!["C".to_string(), "D".to_string()];

        let mut c = make_task_with_parent("C", "Z");
        c.preconditions = vec!["P".to_string()];
        let mut d = make_task_with_parent("D", "Z");
        d.preconditions = vec!["P".to_string()];

        let mut p = make_task_with_parent("P", "Z");
        p.preconditions = vec!["A".to_string()];

        let mut b = make_task_with_parent("B", "Z");
        b.preconditions = vec!["A".to_string()];
        let mut e = make_task_with_parent("E", "Z");
        e.preconditions = vec!["B".to_string()];

        let a = make_task_with_parent("A", "Z");

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
        let output = render_component(&graph);
        let stripped = strip_ansi(&output);

        println!("\n=== Task Types and States ===\n{}", stripped);

        assert!(stripped.contains("[bug]"));
        assert!(stripped.contains("[epic]"));
    }
}
