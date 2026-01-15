use std::collections::{HashMap, HashSet};
use thiserror::Error;

use crate::task::Task;

pub type TaskGraph = HashMap<String, Task>;

// ============================================================================
// Graph Query Operations
// ============================================================================

/// Returns tasks that are available to work on (all dependencies satisfied).
///
/// A task is available if:
/// - It is not complete
/// - It is not a validator
/// - All preconditions are complete
/// - All children are complete
pub fn available_tasks(graph: &TaskGraph) -> Vec<&Task> {
    graph
        .values()
        .filter(|task| !task.complete && !task.validator && is_available(task, graph))
        .collect()
}

/// Check if a specific task is available to work on.
pub fn is_available(task: &Task, graph: &TaskGraph) -> bool {
    // Check all preconditions are complete
    for precond_id in &task.preconditions {
        if let Some(precond) = graph.get(precond_id) {
            if !precond.complete {
                return false;
            }
        }
    }

    // Check all children are complete (tasks that have this task as parent)
    for other_task in graph.values() {
        if let Some(parent_id) = &other_task.parent {
            if parent_id == &task.id && !other_task.complete {
                return false;
            }
        }
    }

    true
}

/// Check if a task belongs to a fully complete group.
/// A task is in a complete group if it and all its ancestors are complete.
pub fn is_group_complete(task: &Task, graph: &TaskGraph) -> bool {
    if !task.complete {
        return false;
    }

    if let Some(parent_id) = &task.parent {
        if let Some(parent) = graph.get(parent_id) {
            return is_group_complete(parent, graph);
        }
    }

    true
}

/// Returns connected components of tasks using union-find.
/// Tasks are connected if they share parent, precondition, or validation relationships.
pub fn connected_components(graph: &TaskGraph) -> Vec<Vec<&str>> {
    let task_ids: Vec<&str> = graph.keys().map(|s| s.as_str()).collect();
    if task_ids.is_empty() {
        return Vec::new();
    }

    let id_to_idx: HashMap<&str, usize> = task_ids
        .iter()
        .enumerate()
        .map(|(i, &id)| (id, i))
        .collect();

    // Union-find parent array
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

    // Union tasks that are connected
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
            if let Some(&val_idx) = id_to_idx.get(validation.as_str()) {
                union(&mut parent, task_idx, val_idx);
            }
        }
    }

    // Group by component
    let mut components: HashMap<usize, Vec<&str>> = HashMap::new();
    for (i, &id) in task_ids.iter().enumerate() {
        let root = find(&mut parent, i);
        components.entry(root).or_default().push(id);
    }

    components.into_values().collect()
}

/// Returns tasks in topological order (dependencies before dependents).
/// Children come before parents, preconditions come before tasks that depend on them.
pub fn topological_sort(graph: &TaskGraph) -> Vec<&Task> {
    if graph.is_empty() {
        return Vec::new();
    }

    let task_ids: HashSet<&str> = graph.keys().map(|s| s.as_str()).collect();

    let mut dependents: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut in_degree: HashMap<&str, usize> = HashMap::new();

    for task in graph.values() {
        in_degree.entry(task.id.as_str()).or_insert(0);
        dependents.entry(task.id.as_str()).or_default();

        // Preconditions: this task depends on precondition completing
        for precond in &task.preconditions {
            if task_ids.contains(precond.as_str()) {
                dependents
                    .entry(precond.as_str())
                    .or_default()
                    .push(&task.id);
                *in_degree.entry(task.id.as_str()).or_insert(0) += 1;
            }
        }

        // Parent: parent depends on this task completing (child before parent)
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

    // Kahn's algorithm with sorted queue for determinism
    let mut queue: Vec<&str> = in_degree
        .iter()
        .filter(|(_, deg)| **deg == 0)
        .map(|(&id, _)| id)
        .collect();
    queue.sort();

    let mut result: Vec<&Task> = Vec::new();

    while let Some(task_id) = queue.pop() {
        if let Some(task) = graph.get(task_id) {
            result.push(task);
        }

        if let Some(deps) = dependents.get(task_id) {
            for &dep in deps {
                if let Some(deg) = in_degree.get_mut(dep) {
                    *deg -= 1;
                    if *deg == 0 {
                        let pos = queue.partition_point(|&x| x > dep);
                        queue.insert(pos, dep);
                    }
                }
            }
        }
    }

    result
}

/// Computes the transitive reduction of dependency edges.
/// Returns a map from task_id to its "effective parent" after removing redundant edges.
///
/// For example, if A → B → C and A → C, the edge A → C is redundant.
/// After reduction, A's effective parent is B (not C).
pub fn transitive_reduction(graph: &TaskGraph) -> HashMap<&str, Option<&str>> {
    let task_ids: HashSet<&str> = graph.keys().map(|s| s.as_str()).collect();

    // Build dependency edges: from task to tasks that depend on it
    let mut edges: HashMap<&str, HashSet<&str>> = HashMap::new();

    for task in graph.values() {
        edges.entry(task.id.as_str()).or_default();

        // Parent relationship: parent depends on this task
        if let Some(parent) = &task.parent {
            if task_ids.contains(parent.as_str()) {
                edges
                    .entry(task.id.as_str())
                    .or_default()
                    .insert(parent.as_str());
            }
        }

        // Precondition relationship: this task depends on precondition
        for precond in &task.preconditions {
            if task_ids.contains(precond.as_str()) {
                edges
                    .entry(precond.as_str())
                    .or_default()
                    .insert(task.id.as_str());
            }
        }
    }

    // Compute reachability for each node
    let mut reachable: HashMap<&str, HashSet<&str>> = HashMap::new();

    for &start in task_ids.iter() {
        let mut visited = HashSet::new();
        let mut stack = vec![start];

        while let Some(node) = stack.pop() {
            if visited.contains(node) {
                continue;
            }
            visited.insert(node);

            if let Some(neighbors) = edges.get(node) {
                for &neighbor in neighbors {
                    if !visited.contains(neighbor) {
                        stack.push(neighbor);
                    }
                }
            }
        }

        visited.remove(start);
        reachable.insert(start, visited);
    }

    // Compute effective parent after reduction
    let mut effective_parent: HashMap<&str, Option<&str>> = HashMap::new();

    for task in graph.values() {
        let task_id = task.id.as_str();

        if let Some(direct_successors) = edges.get(task_id) {
            // Find successors not reachable through other successors
            let mut reduced: Vec<&str> = Vec::new();

            for &succ in direct_successors {
                let reachable_via_other = direct_successors.iter().any(|&other| {
                    other != succ && reachable.get(other).map_or(false, |r| r.contains(succ))
                });

                if !reachable_via_other {
                    reduced.push(succ);
                }
            }

            effective_parent.insert(
                task_id,
                match reduced.len() {
                    0 => None,
                    1 => Some(reduced[0]),
                    _ => {
                        // Multiple successors - prefer original parent if in list
                        let original = task.parent.as_ref().map(|p| p.as_str());
                        if let Some(p) = original {
                            if reduced.contains(&p) {
                                Some(p)
                            } else {
                                Some(reduced[0])
                            }
                        } else {
                            Some(reduced[0])
                        }
                    }
                },
            );
        } else {
            effective_parent.insert(task_id, None);
        }
    }

    effective_parent
}

#[derive(Error, Debug, PartialEq)]
pub enum GraphError {
    #[error("task '{task_id}' references invalid parent '{parent_id}'")]
    InvalidParent { task_id: String, parent_id: String },
    #[error("task '{task_id}' references invalid precondition '{precondition_id}'")]
    InvalidPrecondition {
        task_id: String,
        precondition_id: String,
    },
    #[error("task '{task_id}' has validator '{precondition_id}' as precondition (use validations instead)")]
    PreconditionIsValidator {
        task_id: String,
        precondition_id: String,
    },
    #[error("task '{task_id}' references validation '{validation_id}' which is not a validator")]
    InvalidValidation {
        task_id: String,
        validation_id: String,
    },
    #[error("task '{task_id}' references validator '{validation_id}' which has parents (must be root validator)")]
    ValidationNotRootValidator {
        task_id: String,
        validation_id: String,
    },
    #[error("cycle detected in task graph")]
    CycleDetected,
    #[error("duplicate task id '{0}'")]
    DuplicateTaskId(String),
}

/// Forms a validated task graph from a list of tasks.
///
/// Validates:
/// - No duplicate task IDs
/// - All parent references point to valid tasks
/// - All precondition references point to valid tasks
/// - Non-validator tasks cannot have validators as preconditions
/// - All validation references point to tasks marked as validators
/// - All validation references point to root validators (no parents)
/// - The graph forms a DAG (no cycles)
pub fn form_graph(tasks: Vec<Task>) -> Result<TaskGraph, GraphError> {
    let mut graph: TaskGraph = HashMap::new();

    // Build the graph and check for duplicates
    for task in tasks {
        if graph.contains_key(&task.id) {
            return Err(GraphError::DuplicateTaskId(task.id));
        }
        graph.insert(task.id.clone(), task);
    }

    // Validate all references
    for task in graph.values() {
        // Check parent
        if let Some(parent_id) = &task.parent {
            if !graph.contains_key(parent_id) {
                return Err(GraphError::InvalidParent {
                    task_id: task.id.clone(),
                    parent_id: parent_id.clone(),
                });
            }
        }

        // Check preconditions
        for precondition_id in &task.preconditions {
            let Some(precondition) = graph.get(precondition_id) else {
                return Err(GraphError::InvalidPrecondition {
                    task_id: task.id.clone(),
                    precondition_id: precondition_id.clone(),
                });
            };

            // Non-validator tasks cannot have validators as preconditions
            if !task.validator && precondition.validator {
                return Err(GraphError::PreconditionIsValidator {
                    task_id: task.id.clone(),
                    precondition_id: precondition_id.clone(),
                });
            }
        }

        // Check validations
        for validation_id in &task.validations {
            let Some(validator) = graph.get(validation_id) else {
                return Err(GraphError::InvalidValidation {
                    task_id: task.id.clone(),
                    validation_id: validation_id.clone(),
                });
            };

            if !validator.validator {
                return Err(GraphError::InvalidValidation {
                    task_id: task.id.clone(),
                    validation_id: validation_id.clone(),
                });
            }

            if validator.parent.is_some() {
                return Err(GraphError::ValidationNotRootValidator {
                    task_id: task.id.clone(),
                    validation_id: validation_id.clone(),
                });
            }
        }
    }

    // Check for cycles using DFS with 3-color algorithm
    if has_cycle(&graph) {
        return Err(GraphError::CycleDetected);
    }

    Ok(graph)
}

#[derive(Clone, Copy, PartialEq)]
enum Color {
    White, // Unvisited
    Gray,  // Currently visiting (in stack)
    Black, // Finished visiting
}

fn has_cycle(graph: &TaskGraph) -> bool {
    let mut colors: HashMap<String, Color> = HashMap::new();
    for id in graph.keys() {
        colors.insert(id.clone(), Color::White);
    }

    for id in graph.keys() {
        if colors[id] == Color::White {
            if dfs_cycle(graph, id, &mut colors) {
                return true;
            }
        }
    }

    false
}

fn dfs_cycle(graph: &TaskGraph, task_id: &str, colors: &mut HashMap<String, Color>) -> bool {
    colors.insert(task_id.to_string(), Color::Gray);

    let task = &graph[task_id];

    // Check edges: parent and preconditions form the dependency graph
    let neighbors = task.parent.iter().chain(task.preconditions.iter());
    for neighbor_id in neighbors {
        let neighbor_color = colors.get(neighbor_id).copied().unwrap_or(Color::Black);

        match neighbor_color {
            Color::Gray => return true, // Back edge = cycle
            Color::White => {
                if dfs_cycle(graph, neighbor_id, colors) {
                    return true;
                }
            }
            Color::Black => {} // Already processed, skip
        }
    }

    colors.insert(task_id.to_string(), Color::Black);
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::TaskType;

    fn make_task(id: &str) -> Task {
        Task {
            id: id.to_string(),
            parent: None,
            preconditions: vec![],
            validations: vec![],
            title: None,
            validator: false,
            complete: false,
            task_type: TaskType::Feature,
            description: String::new(),
        }
    }

    fn make_validator(id: &str) -> Task {
        Task {
            id: id.to_string(),
            parent: None,
            preconditions: vec![],
            validations: vec![],
            title: None,
            validator: true,
            complete: false,
            task_type: TaskType::Feature,
            description: String::new(),
        }
    }

    #[test]
    fn test_empty_graph() {
        let result = form_graph(vec![]);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_single_task() {
        let task = make_task("task-1");
        let result = form_graph(vec![task]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }

    #[test]
    fn test_duplicate_task_id() {
        let task1 = make_task("task-1");
        let task2 = make_task("task-1");
        let result = form_graph(vec![task1, task2]);
        assert_eq!(
            result,
            Err(GraphError::DuplicateTaskId("task-1".to_string()))
        );
    }

    #[test]
    fn test_valid_parent() {
        let parent = make_task("parent");
        let mut child = make_task("child");
        child.parent = Some("parent".to_string());

        let result = form_graph(vec![parent, child]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_invalid_parent() {
        let mut task = make_task("task-1");
        task.parent = Some("nonexistent".to_string());

        let result = form_graph(vec![task]);
        assert_eq!(
            result,
            Err(GraphError::InvalidParent {
                task_id: "task-1".to_string(),
                parent_id: "nonexistent".to_string(),
            })
        );
    }

    #[test]
    fn test_valid_precondition() {
        let precond = make_task("precond");
        let mut task = make_task("task");
        task.preconditions = vec!["precond".to_string()];

        let result = form_graph(vec![precond, task]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_invalid_precondition() {
        let mut task = make_task("task-1");
        task.preconditions = vec!["nonexistent".to_string()];

        let result = form_graph(vec![task]);
        assert_eq!(
            result,
            Err(GraphError::InvalidPrecondition {
                task_id: "task-1".to_string(),
                precondition_id: "nonexistent".to_string(),
            })
        );
    }

    #[test]
    fn test_precondition_is_validator() {
        let validator = make_validator("validator");
        let mut task = make_task("task");
        task.preconditions = vec!["validator".to_string()];

        let result = form_graph(vec![validator, task]);
        assert_eq!(
            result,
            Err(GraphError::PreconditionIsValidator {
                task_id: "task".to_string(),
                precondition_id: "validator".to_string(),
            })
        );
    }

    #[test]
    fn test_valid_validation() {
        let validator = make_validator("validator");
        let mut task = make_task("task");
        task.validations = vec!["validator".to_string()];

        let result = form_graph(vec![validator, task]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validation_points_to_nonexistent() {
        let mut task = make_task("task");
        task.validations = vec!["nonexistent".to_string()];

        let result = form_graph(vec![task]);
        assert_eq!(
            result,
            Err(GraphError::InvalidValidation {
                task_id: "task".to_string(),
                validation_id: "nonexistent".to_string(),
            })
        );
    }

    #[test]
    fn test_validation_points_to_non_validator() {
        let not_validator = make_task("not-validator");
        let mut task = make_task("task");
        task.validations = vec!["not-validator".to_string()];

        let result = form_graph(vec![not_validator, task]);
        assert_eq!(
            result,
            Err(GraphError::InvalidValidation {
                task_id: "task".to_string(),
                validation_id: "not-validator".to_string(),
            })
        );
    }

    #[test]
    fn test_validation_not_root_validator() {
        let parent = make_task("parent");
        let mut validator = make_validator("validator");
        validator.parent = Some("parent".to_string());
        let mut task = make_task("task");
        task.validations = vec!["validator".to_string()];

        let result = form_graph(vec![parent, validator, task]);
        assert_eq!(
            result,
            Err(GraphError::ValidationNotRootValidator {
                task_id: "task".to_string(),
                validation_id: "validator".to_string(),
            })
        );
    }

    #[test]
    fn test_valid_dag() {
        // A -> B -> C (each child has one parent)
        let mut a = make_task("a");
        let mut b = make_task("b");
        let c = make_task("c");
        b.parent = Some("c".to_string());
        a.parent = Some("b".to_string());

        let result = form_graph(vec![a, b, c]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cycle_self_loop() {
        let mut task = make_task("task");
        task.parent = Some("task".to_string());

        let result = form_graph(vec![task]);
        assert_eq!(result, Err(GraphError::CycleDetected));
    }

    #[test]
    fn test_cycle_two_nodes() {
        let mut a = make_task("a");
        let mut b = make_task("b");
        a.parent = Some("b".to_string());
        b.parent = Some("a".to_string());

        let result = form_graph(vec![a, b]);
        assert_eq!(result, Err(GraphError::CycleDetected));
    }

    #[test]
    fn test_cycle_three_nodes() {
        let mut a = make_task("a");
        let mut b = make_task("b");
        let mut c = make_task("c");
        a.parent = Some("b".to_string());
        b.parent = Some("c".to_string());
        c.parent = Some("a".to_string());

        let result = form_graph(vec![a, b, c]);
        assert_eq!(result, Err(GraphError::CycleDetected));
    }

    #[test]
    fn test_cycle_via_preconditions() {
        let mut a = make_task("a");
        let mut b = make_task("b");
        a.preconditions = vec!["b".to_string()];
        b.preconditions = vec!["a".to_string()];

        let result = form_graph(vec![a, b]);
        assert_eq!(result, Err(GraphError::CycleDetected));
    }

    #[test]
    fn test_cycle_mixed_parent_and_preconditions() {
        let mut a = make_task("a");
        let mut b = make_task("b");
        let mut c = make_task("c");
        a.parent = Some("b".to_string());
        b.preconditions = vec!["c".to_string()];
        c.parent = Some("a".to_string());

        let result = form_graph(vec![a, b, c]);
        assert_eq!(result, Err(GraphError::CycleDetected));
    }

    #[test]
    fn test_complex_valid_graph() {
        // validator1, validator2 (root validators)
        // task1 has parent task2
        // task1 has precondition on task3
        // task1 validates against validator1 and validator2
        let validator1 = make_validator("validator1");
        let validator2 = make_validator("validator2");
        let task2 = make_task("task2");
        let task3 = make_task("task3");
        let mut task1 = make_task("task1");
        task1.parent = Some("task2".to_string());
        task1.preconditions = vec!["task3".to_string()];
        task1.validations = vec!["validator1".to_string(), "validator2".to_string()];

        let result = form_graph(vec![validator1, validator2, task2, task3, task1]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 5);
    }
}
