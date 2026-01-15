use owo_colors::OwoColorize;
use std::collections::{HashMap, HashSet};

use crate::graph::{self, TaskGraph};
use crate::task::{Task, TaskType};

/// Renders a task graph to a string for display.
///
/// Tasks are displayed in dependency order (things you can start first at top).
/// Both `parent` and `preconditions` are treated as dependencies:
/// - If A has parent B, then B depends on A (children come before parents)
/// - If A has precondition B, then A depends on B (B comes before A)
pub fn render_task_graph(tasks: &[Task]) -> String {
    if tasks.is_empty() {
        return String::new();
    }

    // Build a graph for queries
    let graph: TaskGraph = tasks
        .iter()
        .cloned()
        .map(|t| (t.id.clone(), t))
        .collect();

    // Separate validators and regular tasks
    let (validators, regular): (Vec<_>, Vec<_>) = tasks.iter().partition(|t| t.validator);

    // Topologically sort each group, keeping connected components together
    let sorted_validators = sort_by_component(&validators, &graph);
    let sorted_regular = sort_by_component(&regular, &graph);

    // Split regular tasks into active and complete groups
    let (active, complete): (Vec<_>, Vec<_>) = sorted_regular
        .into_iter()
        .partition(|t| !graph::is_group_complete(t, &graph));

    let mut output = String::new();

    // Render active regular tasks first
    render_section(&mut output, &active, &graph);

    // Blank line before validators if both exist
    if !active.is_empty() && !sorted_validators.is_empty() {
        output.push('\n');
    }

    // Render validators
    render_section(&mut output, &sorted_validators, &graph);

    // Blank line before complete tasks if needed
    if (!active.is_empty() || !sorted_validators.is_empty()) && !complete.is_empty() {
        output.push('\n');
    }

    // Render complete regular tasks last
    render_section(&mut output, &complete, &graph);

    output
}

// ============================================================================
// Sorting
// ============================================================================

/// Sort tasks by component, with complete components last.
fn sort_by_component<'a>(tasks: &[&'a Task], graph: &TaskGraph) -> Vec<&'a Task> {
    if tasks.is_empty() {
        return Vec::new();
    }

    let component_map = build_component_map(tasks);

    // Group tasks by component
    let mut by_component: HashMap<usize, Vec<&Task>> = HashMap::new();
    for &task in tasks {
        let comp = component_map.get(task.id.as_str()).copied().unwrap_or(0);
        by_component.entry(comp).or_default().push(task);
    }

    // Check if a component is fully complete
    let is_complete = |comp_id: &usize| -> bool {
        by_component
            .get(comp_id)
            .map_or(false, |tasks| tasks.iter().all(|t| t.complete))
    };

    // Sort: incomplete first, complete last
    let mut comp_ids: Vec<usize> = by_component.keys().copied().collect();
    comp_ids.sort_by(|a, b| {
        let a_complete = is_complete(a);
        let b_complete = is_complete(b);
        match (a_complete, b_complete) {
            (false, true) => std::cmp::Ordering::Less,
            (true, false) => std::cmp::Ordering::Greater,
            _ => a.cmp(b),
        }
    });

    // Topologically sort each component
    let mut result = Vec::new();
    for comp_id in comp_ids {
        let comp_tasks = &by_component[&comp_id];
        let sorted = topological_sort_subset(comp_tasks, graph);
        result.extend(sorted);
    }

    result
}

/// Topological sort for a subset of tasks.
/// Bugs are prioritized to appear first when they have no unsatisfied dependencies.
fn topological_sort_subset<'a>(tasks: &[&'a Task], _graph: &TaskGraph) -> Vec<&'a Task> {
    let task_ids: HashSet<&str> = tasks.iter().map(|t| t.id.as_str()).collect();
    let task_map: HashMap<&str, &Task> = tasks.iter().map(|t| (t.id.as_str(), *t)).collect();

    let mut dependents: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut in_degree: HashMap<&str, usize> = HashMap::new();

    for &task in tasks {
        in_degree.entry(task.id.as_str()).or_insert(0);
        dependents.entry(task.id.as_str()).or_default();

        for precond in &task.preconditions {
            if task_ids.contains(precond.as_str()) {
                dependents
                    .entry(precond.as_str())
                    .or_default()
                    .push(&task.id);
                *in_degree.entry(task.id.as_str()).or_insert(0) += 1;
            }
        }

        if let Some(parent) = &task.parent {
            if task_ids.contains(parent.as_str()) {
                dependents
                    .entry(task.id.as_str())
                    .or_default()
                    .push(parent.as_str());
                *in_degree.entry(parent.as_str()).or_insert(0) += 1;
            }
        }
    }

    // Sort comparator: bugs first, then alphabetically
    // Returns (priority, id) where priority 0 = bug, 1 = other
    let is_bug = |id: &str| -> bool {
        task_map
            .get(id)
            .map(|t| t.task_type == TaskType::Bug)
            .unwrap_or(false)
    };

    let mut queue: Vec<&str> = in_degree
        .iter()
        .filter(|(_, deg)| **deg == 0)
        .map(|(&id, _)| id)
        .collect();
    // Sort: bugs first (false < true when reversed), then alphabetically
    // We sort ascending by (!is_bug, id), so bugs come first
    queue.sort_by(|a, b| {
        let a_priority = (!is_bug(a), *a);
        let b_priority = (!is_bug(b), *b);
        a_priority.cmp(&b_priority)
    });

    let mut result: Vec<&Task> = Vec::new();

    while let Some(task_id) = queue.pop() {
        if let Some(&task) = task_map.get(task_id) {
            result.push(task);
        }

        if let Some(deps) = dependents.get(task_id) {
            for &dep in deps {
                if let Some(deg) = in_degree.get_mut(dep) {
                    *deg -= 1;
                    if *deg == 0 {
                        // Insert maintaining sort order
                        let dep_priority = (!is_bug(dep), dep);
                        let pos = queue.partition_point(|&x| {
                            let x_priority = (!is_bug(x), x);
                            x_priority < dep_priority
                        });
                        queue.insert(pos, dep);
                    }
                }
            }
        }
    }

    result
}

/// Build component map for a subset of tasks using union-find.
fn build_component_map<'a>(tasks: &[&'a Task]) -> HashMap<&'a str, usize> {
    let task_ids: Vec<&str> = tasks.iter().map(|t| t.id.as_str()).collect();
    let id_to_idx: HashMap<&str, usize> =
        task_ids.iter().enumerate().map(|(i, &id)| (id, i)).collect();

    let mut parent: Vec<usize> = (0..tasks.len()).collect();

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

    for &task in tasks {
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
            if let Some(&val_idx) = id_to_idx.get(validation.as_str()) {
                union(&mut parent, task_idx, val_idx);
            }
        }
    }

    let mut result = HashMap::new();
    for (i, &id) in task_ids.iter().enumerate() {
        result.insert(id, find(&mut parent, i));
    }

    result
}

// ============================================================================
// Rendering
// ============================================================================

fn render_section(output: &mut String, tasks: &[&Task], graph: &TaskGraph) {
    if tasks.is_empty() {
        return;
    }

    // Build a sub-graph for transitive reduction on just this section
    let section_graph: TaskGraph = tasks
        .iter()
        .cloned()
        .map(|t| (t.id.clone(), t.clone()))
        .collect();
    let effective_successors = graph::transitive_reduction(&section_graph);

    // Active lines: maps column index to the task_id that line is heading towards
    let mut active_lines: Vec<Option<&str>> = Vec::new();

    for (_idx, task) in tasks.iter().enumerate() {
        let task_id = task.id.as_str();

        // Find columns of children's lines pointing to this task
        let child_columns: Vec<usize> = active_lines
            .iter()
            .enumerate()
            .filter(|(_, target)| **target == Some(task_id))
            .map(|(col, _)| col)
            .collect();

        // Determine this task's column
        let successors = effective_successors
            .get(task_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[]);
        let has_successor_in_section = !successors.is_empty();

        let task_column = if !has_successor_in_section {
            0
        } else if let Some(col) = find_column_for_parent(&active_lines, task_id) {
            col
        } else if let Some(&first_child_col) = child_columns.first() {
            first_child_col
        } else {
            active_lines
                .iter()
                .position(|x| x.is_none())
                .unwrap_or_else(|| {
                    active_lines.push(None);
                    active_lines.len() - 1
                })
        };

        // Render merge lines if multiple children
        if child_columns.len() > 1 {
            render_merge_lines(output, &active_lines, &child_columns, task_column);
        }

        // Build prefix
        let graph_prefix = build_graph_prefix(&active_lines, task_column);

        // Close lines from children
        for &col in &child_columns {
            if col < active_lines.len() {
                active_lines[col] = None;
            }
        }

        // Start lines to this task's effective successors
        for (i, &successor) in successors.iter().enumerate() {
            let col = if i == 0 {
                task_column
            } else {
                // Find or create a new column for additional successors
                active_lines
                    .iter()
                    .position(|x| x.is_none())
                    .unwrap_or_else(|| {
                        active_lines.push(None);
                        active_lines.len() - 1
                    })
            };
            while active_lines.len() <= col {
                active_lines.push(None);
            }
            active_lines[col] = Some(successor);
        }

        render_task_line(output, task, &graph_prefix, graph);

        // If multiple successors, render fork lines
        if successors.len() > 1 {
            let fork_columns: Vec<usize> = active_lines
                .iter()
                .enumerate()
                .filter(|(_, target)| successors.contains(&target.unwrap_or("")))
                .map(|(col, _)| col)
                .collect();
            if fork_columns.len() > 1 {
                render_fork_lines(output, &active_lines, &fork_columns, task_column);
            }
        }
    }
}

fn render_fork_lines(
    output: &mut String,
    active_lines: &[Option<&str>],
    fork_columns: &[usize],
    source_column: usize,
) {
    if fork_columns.len() <= 1 {
        return;
    }

    let min_col = *fork_columns.iter().min().unwrap();
    let max_col = *fork_columns.iter().max().unwrap();

    let mut line = String::new();
    for col in 0..=max_col.max(active_lines.len().saturating_sub(1)) {
        if col == source_column {
            if fork_columns.contains(&col) {
                line.push_str("├─");
            } else {
                line.push_str("│ ");
            }
        } else if fork_columns.contains(&col) {
            if col == max_col {
                line.push_str("╮ ");
            } else if col > min_col {
                line.push_str("┬─");
            } else {
                line.push_str("├─");
            }
        } else if col > min_col && col < max_col {
            line.push_str("──");
        } else if active_lines.get(col).copied().flatten().is_some() {
            line.push_str("│ ");
        } else {
            line.push_str("  ");
        }
    }

    if !line.trim().is_empty() {
        output.push_str(&line.trim_end());
        output.push('\n');
    }
}

fn find_column_for_parent(active_lines: &[Option<&str>], parent_id: &str) -> Option<usize> {
    active_lines
        .iter()
        .position(|target| *target == Some(parent_id))
}

fn render_merge_lines(
    output: &mut String,
    active_lines: &[Option<&str>],
    child_columns: &[usize],
    target_column: usize,
) {
    if child_columns.len() <= 1 {
        return;
    }

    let min_col = *child_columns.iter().min().unwrap();
    let max_col = *child_columns.iter().max().unwrap();

    let mut line = String::new();
    for col in 0..=max_col.max(active_lines.len().saturating_sub(1)) {
        if col == target_column {
            if child_columns.contains(&col) {
                line.push_str("├─");
            } else {
                line.push_str("│ ");
            }
        } else if child_columns.contains(&col) {
            if col == max_col {
                line.push_str("╯ ");
            } else if col > min_col {
                line.push_str("┴─");
            } else {
                line.push_str("├─");
            }
        } else if col > min_col && col < max_col {
            line.push_str("──");
        } else if active_lines.get(col).copied().flatten().is_some() {
            line.push_str("│ ");
        } else {
            line.push_str("  ");
        }
    }

    if !line.trim().is_empty() {
        output.push_str(&line.trim_end());
        output.push('\n');
    }
}

fn build_graph_prefix(active_lines: &[Option<&str>], task_column: usize) -> String {
    let mut prefix = String::new();

    for col in 0..active_lines.len().max(task_column + 1) {
        if col == task_column {
            break;
        } else if active_lines.get(col).copied().flatten().is_some() {
            prefix.push_str("│ ");
        } else {
            prefix.push_str("  ");
        }
    }

    prefix
}

const MAX_TITLE_LEN: usize = 60;

fn truncate_title(title: &str) -> String {
    if title.len() <= MAX_TITLE_LEN {
        title.to_string()
    } else {
        format!("{}…", &title[..MAX_TITLE_LEN - 1])
    }
}

fn render_task_line(output: &mut String, task: &Task, graph_prefix: &str, graph: &TaskGraph) {
    let is_available = !task.complete && !task.validator && graph::is_available(task, graph);
    let is_bug = task.task_type == TaskType::Bug;
    let is_epic = task.task_type == TaskType::Epic;

    let marker = if task.validator {
        "◈".purple().to_string()
    } else if task.complete {
        "●".bright_black().to_string()
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
    } else if is_available && is_bug {
        format!("{}{}", title_truncated.red(), type_suffix)
    } else if is_available && is_epic {
        format!("{}{}", title_truncated.cyan(), type_suffix)
    } else if is_available {
        format!("{}{}", title_truncated.bright_green(), type_suffix)
    } else {
        format!("{}{}", title_truncated.bright_black(), type_suffix)
    };

    output.push_str(&format!(
        "{}{} {} {}\n",
        graph_prefix, marker, id_display, title_formatted
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_task(id: &str, title: Option<&str>, parent: Option<&str>) -> Task {
        Task {
            id: id.to_string(),
            parent: parent.map(String::from),
            preconditions: vec![],
            validations: vec![],
            title: title.map(String::from),
            validator: false,
            complete: false,
            task_type: TaskType::Feature,
            description: String::new(),
        }
    }

    fn make_validator(id: &str, title: Option<&str>) -> Task {
        Task {
            id: id.to_string(),
            parent: None,
            preconditions: vec![],
            validations: vec![],
            title: title.map(String::from),
            validator: true,
            complete: false,
            task_type: TaskType::Feature,
            description: String::new(),
        }
    }

    fn make_completed(id: &str, title: Option<&str>, parent: Option<&str>) -> Task {
        Task {
            id: id.to_string(),
            parent: parent.map(String::from),
            preconditions: vec![],
            validations: vec![],
            title: title.map(String::from),
            validator: false,
            complete: true,
            task_type: TaskType::Feature,
            description: String::new(),
        }
    }

    fn make_task_with_preconditions(
        id: &str,
        title: Option<&str>,
        parent: Option<&str>,
        preconditions: Vec<&str>,
    ) -> Task {
        Task {
            id: id.to_string(),
            parent: parent.map(String::from),
            preconditions: preconditions.into_iter().map(String::from).collect(),
            validations: vec![],
            title: title.map(String::from),
            validator: false,
            complete: false,
            task_type: TaskType::Feature,
            description: String::new(),
        }
    }

    fn make_validator_with_parent(id: &str, title: Option<&str>, parent: Option<&str>) -> Task {
        Task {
            id: id.to_string(),
            parent: parent.map(String::from),
            preconditions: vec![],
            validations: vec![],
            title: title.map(String::from),
            validator: true,
            complete: false,
            task_type: TaskType::Feature,
            description: String::new(),
        }
    }

    fn make_bug(id: &str, title: Option<&str>, parent: Option<&str>) -> Task {
        Task {
            id: id.to_string(),
            parent: parent.map(String::from),
            preconditions: vec![],
            validations: vec![],
            title: title.map(String::from),
            validator: false,
            complete: false,
            task_type: TaskType::Bug,
            description: String::new(),
        }
    }

    #[test]
    fn test_render_comprehensive_graph() {
        let tasks = vec![
            make_validator("test", Some("Run tests")),
            make_validator("build", Some("Build project")),
            make_validator_with_parent("lint", Some("Run linter"), Some("test")),
            make_task("cli-commands", Some("CLI Commands"), None),
            make_task("mont-start", Some("Implement mont start"), Some("cli-commands")),
            make_completed("mont-list", Some("Implement mont list"), Some("cli-commands")),
            make_task("mont-complete", Some("Implement mont complete"), Some("cli-commands")),
            make_completed("setup-db", Some("Setup database"), None),
            make_task_with_preconditions(
                "api-feature",
                Some("Add API feature"),
                None,
                vec!["setup-db"],
            ),
            make_task("data-migration", Some("Data Migration"), None),
            make_task("migrate-users", Some("Migrate users"), Some("data-migration")),
            make_task_with_preconditions(
                "migrate-posts",
                Some("Migrate posts"),
                Some("data-migration"),
                vec!["migrate-users"],
            ),
        ];

        let output = render_task_graph(&tasks);
        println!("\n--- Comprehensive Graph ---\n{}", output);

        for id in &[
            "cli-commands",
            "mont-start",
            "mont-list",
            "mont-complete",
            "test",
            "build",
            "lint",
            "setup-db",
            "api-feature",
            "data-migration",
            "migrate-users",
            "migrate-posts",
        ] {
            assert!(output.contains(id), "output should contain {}", id);
        }
    }

    #[test]
    fn test_render_empty() {
        let tasks: Vec<Task> = vec![];
        let output = render_task_graph(&tasks);
        assert!(output.is_empty());
    }

    #[test]
    fn test_render_single_task() {
        let tasks = vec![make_task("solo", Some("Solo Task"), None)];
        let output = render_task_graph(&tasks);
        println!("\n--- Single Task ---\n{}", output);
        assert!(output.contains("solo"));
        assert!(output.contains("Solo Task"));
    }

    #[test]
    fn test_render_deep_hierarchy() {
        let tasks = vec![
            make_task("level-1", Some("Level 1"), None),
            make_task("level-2", Some("Level 2"), Some("level-1")),
            make_task("level-3", Some("Level 3"), Some("level-2")),
            make_task("level-4", Some("Level 4"), Some("level-3")),
        ];

        let output = render_task_graph(&tasks);
        println!("\n--- Deep Hierarchy ---\n{}", output);

        let level_4_pos = output.find("level-4").unwrap();
        let level_1_pos = output.find("level-1").unwrap();
        assert!(
            level_4_pos < level_1_pos,
            "level-4 (deepest child) should come before level-1 (root parent)"
        );

        // In a single-column chain, vertical relationship is shown by task ordering
        // No explicit vertical lines needed - clean, minimal output
    }

    #[test]
    fn test_render_siblings() {
        let tasks = vec![
            make_task("parent", Some("Parent"), None),
            make_task("child-a", Some("Child A"), Some("parent")),
            make_task("child-b", Some("Child B"), Some("parent")),
        ];

        let output = render_task_graph(&tasks);
        println!("\n--- Siblings ---\n{}", output);

        let child_a_pos = output.find("child-a").unwrap();
        let child_b_pos = output.find("child-b").unwrap();
        let parent_pos = output.find("parent").unwrap();
        assert!(child_a_pos < parent_pos);
        assert!(child_b_pos < parent_pos);
    }

    #[test]
    fn test_precondition_ordering() {
        let tasks = vec![
            make_task("task-b", Some("Task B"), None),
            make_task_with_preconditions("task-a", Some("Task A"), None, vec!["task-b"]),
        ];

        let output = render_task_graph(&tasks);
        println!("\n--- Precondition ---\n{}", output);

        let b_pos = output.find("task-b").unwrap();
        let a_pos = output.find("task-a").unwrap();
        assert!(
            b_pos < a_pos,
            "task-b should come before task-a (precondition)"
        );
    }

    #[test]
    fn test_diamond_pattern() {
        let mut tasks = vec![
            make_task("bottom", Some("Bottom"), None),
            make_task("left", Some("Left"), Some("bottom")),
            make_task("right", Some("Right"), Some("bottom")),
            make_task("top", Some("Top"), Some("left")),
        ];
        tasks[3].preconditions = vec!["right".to_string()];

        let output = render_task_graph(&tasks);
        println!("\n--- Diamond Pattern ---\n{}", output);

        let top_pos = output.find("top").unwrap();
        let right_pos = output.find("right").unwrap();
        let bottom_pos = output.find("bottom").unwrap();
        assert!(right_pos < top_pos, "right should come before top");
        assert!(top_pos < bottom_pos, "top should come before bottom");
    }

    #[test]
    fn test_diamond_fork_pattern() {
        // Bug report scenario: precondition forks to two tasks that merge back
        // task-parent (root)
        // task-precondition (parent: task-parent)
        // task-a (parent: task-parent, precondition: task-precondition)
        // task-b (parent: task-parent, precondition: task-precondition)
        let tasks = vec![
            make_task("task-parent", Some("Parent"), None),
            make_task("task-precondition", Some("Precondition"), Some("task-parent")),
            make_task_with_preconditions(
                "task-a",
                Some("Task A"),
                Some("task-parent"),
                vec!["task-precondition"],
            ),
            make_task_with_preconditions(
                "task-b",
                Some("Task B"),
                Some("task-parent"),
                vec!["task-precondition"],
            ),
        ];

        let output = render_task_graph(&tasks);
        println!("\n--- Diamond Fork Pattern ---\n{}", output);

        // Verify ordering
        let precond_pos = output.find("task-precondition").unwrap();
        let a_pos = output.find("task-a").unwrap();
        let b_pos = output.find("task-b").unwrap();
        let parent_pos = output.find("task-parent").unwrap();

        assert!(
            precond_pos < a_pos,
            "task-precondition should come before task-a"
        );
        assert!(
            precond_pos < b_pos,
            "task-precondition should come before task-b"
        );
        assert!(a_pos < parent_pos, "task-a should come before task-parent");
        assert!(b_pos < parent_pos, "task-b should come before task-parent");

        // Verify fork is rendered (both task-a and task-b should have lines to them)
        // The fork should show visual connection from precondition to both a and b
        assert!(
            output.contains("├") || output.contains("╮"),
            "should have fork/merge markers showing diamond shape"
        );
    }

    #[test]
    fn test_wide_tree() {
        let tasks = vec![
            make_task("root", Some("Root"), None),
            make_task("child-1", Some("Child 1"), Some("root")),
            make_task("child-2", Some("Child 2"), Some("root")),
            make_task("child-3", Some("Child 3"), Some("root")),
            make_task("child-4", Some("Child 4"), Some("root")),
            make_task("child-5", Some("Child 5"), Some("root")),
        ];

        let output = render_task_graph(&tasks);
        println!("\n--- Wide Tree (5 children) ---\n{}", output);

        let root_pos = output.find("root").unwrap();
        for i in 1..=5 {
            let child_pos = output.find(&format!("child-{}", i)).unwrap();
            assert!(child_pos < root_pos, "child-{} should come before root", i);
        }
        assert!(output.contains("├"), "should have merge markers");
    }

    #[test]
    fn test_multiple_independent_trees() {
        let tasks = vec![
            make_task("tree1-root", Some("Tree 1 Root"), None),
            make_task("tree1-child", Some("Tree 1 Child"), Some("tree1-root")),
            make_task("tree2-root", Some("Tree 2 Root"), None),
            make_task("tree2-child-a", Some("Tree 2 Child A"), Some("tree2-root")),
            make_task("tree2-child-b", Some("Tree 2 Child B"), Some("tree2-root")),
            make_task("tree3-solo", Some("Tree 3 Solo"), None),
        ];

        let output = render_task_graph(&tasks);
        println!("\n--- Multiple Independent Trees ---\n{}", output);

        // All trees should be present, rendered compactly without separators
        assert!(output.contains("tree1-root"));
        assert!(output.contains("tree2-root"));
        assert!(output.contains("tree3-solo"));
    }

    #[test]
    fn test_chain_with_preconditions() {
        let tasks = vec![
            make_task("parent", Some("Parent"), None),
            make_task("step-1", Some("Step 1"), Some("parent")),
            make_task_with_preconditions("step-2", Some("Step 2"), Some("parent"), vec!["step-1"]),
            make_task_with_preconditions("step-3", Some("Step 3"), Some("parent"), vec!["step-2"]),
        ];

        let output = render_task_graph(&tasks);
        println!("\n--- Chain with Preconditions ---\n{}", output);

        let step1_pos = output.find("step-1").unwrap();
        let step2_pos = output.find("step-2").unwrap();
        let step3_pos = output.find("step-3").unwrap();
        let parent_pos = output.find("parent").unwrap();

        assert!(step1_pos < step2_pos, "step-1 before step-2");
        assert!(step2_pos < step3_pos, "step-2 before step-3");
        assert!(step3_pos < parent_pos, "all steps before parent");
    }

    #[test]
    fn test_mixed_complete_and_incomplete() {
        let tasks = vec![
            make_task("root", Some("Root Task"), None),
            make_completed("done-1", Some("Done 1"), Some("root")),
            make_completed("done-2", Some("Done 2"), Some("root")),
            make_task("todo-1", Some("Todo 1"), Some("root")),
            make_task("todo-2", Some("Todo 2"), Some("root")),
        ];

        let output = render_task_graph(&tasks);
        println!("\n--- Mixed Complete/Incomplete ---\n{}", output);

        assert!(output.contains("●"), "should have complete markers");
        assert!(
            output.contains("◉") || output.contains("○"),
            "should have incomplete markers"
        );
    }

    #[test]
    fn test_nested_tree() {
        let tasks = vec![
            make_task("root", Some("Root"), None),
            make_task("branch-a", Some("Branch A"), Some("root")),
            make_task("branch-b", Some("Branch B"), Some("root")),
            make_task("leaf-1", Some("Leaf 1"), Some("branch-a")),
            make_task("leaf-2", Some("Leaf 2"), Some("branch-a")),
            make_task("leaf-3", Some("Leaf 3"), Some("branch-b")),
        ];

        let output = render_task_graph(&tasks);
        println!("\n--- Nested Tree ---\n{}", output);

        let root_pos = output.find("root").unwrap();
        let branch_a_pos = output.find("branch-a").unwrap();
        let branch_b_pos = output.find("branch-b").unwrap();
        let leaf_1_pos = output.find("leaf-1").unwrap();
        let leaf_2_pos = output.find("leaf-2").unwrap();
        let leaf_3_pos = output.find("leaf-3").unwrap();

        assert!(leaf_1_pos < branch_a_pos);
        assert!(leaf_2_pos < branch_a_pos);
        assert!(leaf_3_pos < branch_b_pos);
        assert!(branch_a_pos < root_pos);
        assert!(branch_b_pos < root_pos);
    }

    #[test]
    fn test_validators_with_hierarchy() {
        let tasks = vec![
            make_validator("test-all", Some("Run all tests")),
            make_validator_with_parent("test-unit", Some("Unit tests"), Some("test-all")),
            make_validator_with_parent(
                "test-integration",
                Some("Integration tests"),
                Some("test-all"),
            ),
            make_task("feature", Some("Feature"), None),
        ];

        let output = render_task_graph(&tasks);
        println!("\n--- Validators with Hierarchy ---\n{}", output);

        assert!(output.contains("test-all"));
        assert!(output.contains("test-unit"));
        assert!(output.contains("test-integration"));
    }

    #[test]
    fn test_available_vs_blocked_display() {
        let tasks = vec![
            make_task("root", Some("Root (blocked - has children)"), None),
            make_task("available-leaf", Some("Available Leaf"), Some("root")),
            make_completed("completed-leaf", Some("Completed Leaf"), Some("root")),
            make_task_with_preconditions(
                "blocked-by-precond",
                Some("Blocked by precondition"),
                Some("root"),
                vec!["available-leaf"],
            ),
        ];

        let output = render_task_graph(&tasks);
        println!("\n--- Available vs Blocked ---\n{}", output);

        assert!(output.contains("◉"), "should have available marker");
        assert!(output.contains("○"), "should have blocked marker");
        assert!(output.contains("●"), "should have complete marker");
    }

    #[test]
    fn test_bug_task_display() {
        let tasks = vec![
            make_bug("crash-fix", Some("Fix crash on login"), None),
            make_task("feature", Some("New feature"), None),
        ];

        let output = render_task_graph(&tasks);
        println!("\n--- Bug Task Display ---\n{}", output);

        assert!(output.contains("[bug]"), "should have [bug] suffix");
        assert!(output.contains("crash-fix"));
        assert!(output.contains("Fix crash on login"));
    }

    #[test]
    fn test_bug_with_blocked_parent() {
        let tasks = vec![
            make_task("root", Some("Parent task"), None),
            make_bug("blocked-bug", Some("Blocked bug task"), Some("root")),
        ];

        let output = render_task_graph(&tasks);
        println!("\n--- Blocked Bug Task ---\n{}", output);

        // Both should be present
        assert!(output.contains("root"));
        assert!(output.contains("blocked-bug"));
        assert!(output.contains("[bug]"));
    }

    // ========================================================================
    // E2E Visual Tests - validate exact output to catch regressions
    // ========================================================================

    /// Strip ANSI escape codes for visual comparison
    fn strip_ansi(s: &str) -> String {
        let re = regex::Regex::new(r"\x1b\[[0-9;]*m").unwrap();
        re.replace_all(s, "").to_string()
    }

    #[test]
    fn test_e2e_simple_parent_child() {
        let tasks = vec![
            make_task("parent", Some("Parent Task"), None),
            make_task("child", Some("Child Task"), Some("parent")),
        ];

        let output = strip_ansi(&render_task_graph(&tasks));
        let expected = "\
◉ child Child Task
○ parent Parent Task
";
        assert_eq!(output, expected, "\n--- Got ---\n{}\n--- Expected ---\n{}", output, expected);
    }

    #[test]
    fn test_e2e_two_children_merge() {
        let tasks = vec![
            make_task("parent", Some("Parent"), None),
            make_task("child-a", Some("Child A"), Some("parent")),
            make_task("child-b", Some("Child B"), Some("parent")),
        ];

        let output = strip_ansi(&render_task_graph(&tasks));
        let expected = "\
◉ child-b Child B
│ ◉ child-a Child A
├─╯
○ parent Parent
";
        assert_eq!(output, expected, "\n--- Got ---\n{}\n--- Expected ---\n{}", output, expected);
    }

    #[test]
    fn test_e2e_diamond_pattern() {
        // Diamond: precondition forks to A and B, both merge back to parent
        // task-a and task-b are blocked (○) because precond is not complete
        let tasks = vec![
            make_task("parent", Some("Parent"), None),
            make_task("precond", Some("Precondition"), Some("parent")),
            make_task_with_preconditions("task-a", Some("Task A"), Some("parent"), vec!["precond"]),
            make_task_with_preconditions("task-b", Some("Task B"), Some("parent"), vec!["precond"]),
        ];

        let output = strip_ansi(&render_task_graph(&tasks));
        let expected = "\
◉ precond Precondition
├─╮
│ ○ task-b Task B
○ task-a Task A
├─╯
○ parent Parent
";
        assert_eq!(output, expected, "\n--- Got ---\n{}\n--- Expected ---\n{}", output, expected);
    }

    #[test]
    fn test_e2e_chain_with_fork() {
        // A chain where one task forks to two successors
        let tasks = vec![
            make_task("root", Some("Root"), None),
            make_task("middle", Some("Middle"), Some("root")),
            make_task("branch-a", Some("Branch A"), Some("middle")),
            make_task("branch-b", Some("Branch B"), Some("middle")),
        ];

        let output = strip_ansi(&render_task_graph(&tasks));
        let expected = "\
◉ branch-b Branch B
│ ◉ branch-a Branch A
├─╯
○ middle Middle
○ root Root
";
        assert_eq!(output, expected, "\n--- Got ---\n{}\n--- Expected ---\n{}", output, expected);
    }

    #[test]
    fn test_e2e_three_children() {
        let tasks = vec![
            make_task("parent", Some("Parent"), None),
            make_task("child-1", Some("Child 1"), Some("parent")),
            make_task("child-2", Some("Child 2"), Some("parent")),
            make_task("child-3", Some("Child 3"), Some("parent")),
        ];

        let output = strip_ansi(&render_task_graph(&tasks));
        let expected = "\
◉ child-3 Child 3
│ ◉ child-2 Child 2
│ │ ◉ child-1 Child 1
├─┴─╯
○ parent Parent
";
        assert_eq!(output, expected, "\n--- Got ---\n{}\n--- Expected ---\n{}", output, expected);
    }

    #[test]
    fn test_e2e_nested_hierarchy() {
        // Parent with child, child has grandchild
        let tasks = vec![
            make_task("grandparent", Some("Grandparent"), None),
            make_task("parent", Some("Parent"), Some("grandparent")),
            make_task("child", Some("Child"), Some("parent")),
        ];

        let output = strip_ansi(&render_task_graph(&tasks));
        let expected = "\
◉ child Child
○ parent Parent
○ grandparent Grandparent
";
        assert_eq!(output, expected, "\n--- Got ---\n{}\n--- Expected ---\n{}", output, expected);
    }

    #[test]
    fn test_e2e_independent_trees() {
        let tasks = vec![
            make_task("tree1-root", Some("Tree 1 Root"), None),
            make_task("tree1-child", Some("Tree 1 Child"), Some("tree1-root")),
            make_task("tree2-solo", Some("Tree 2 Solo"), None),
        ];

        let output = strip_ansi(&render_task_graph(&tasks));
        let expected = "\
◉ tree1-child Tree 1 Child
○ tree1-root Tree 1 Root
◉ tree2-solo Tree 2 Solo
";
        assert_eq!(output, expected, "\n--- Got ---\n{}\n--- Expected ---\n{}", output, expected);
    }

    #[test]
    fn test_e2e_validators_separate_section() {
        let tasks = vec![
            make_task("feature", Some("Feature"), None),
            make_validator("test", Some("Run tests")),
        ];

        let output = strip_ansi(&render_task_graph(&tasks));
        let expected = "\
◉ feature Feature

◈ test Run tests [validator]
";
        assert_eq!(output, expected, "\n--- Got ---\n{}\n--- Expected ---\n{}", output, expected);
    }
}
