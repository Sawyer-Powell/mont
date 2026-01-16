use std::collections::{HashMap, HashSet};

use crate::task::Task;
use crate::validations::{validate_graph, ValidationError};

pub type TaskGraph = HashMap<String, Task>;

pub use crate::validations::ValidationError as GraphError;

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
            if let Some(&val_idx) = id_to_idx.get(validation.id.as_str()) {
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
/// Returns a map from task_id to its effective successors after removing redundant edges.
///
/// For example, if A → B → C and A → C, the edge A → C is redundant.
/// After reduction, A's effective successors are just [B] (not [B, C]).
pub fn transitive_reduction(graph: &TaskGraph) -> HashMap<&str, Vec<&str>> {
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

    // Compute effective successors after reduction
    let mut effective_successors: HashMap<&str, Vec<&str>> = HashMap::new();

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

            // Sort for deterministic output
            reduced.sort();
            effective_successors.insert(task_id, reduced);
        } else {
            effective_successors.insert(task_id, Vec::new());
        }
    }

    effective_successors
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
pub fn form_graph(tasks: Vec<Task>) -> Result<TaskGraph, ValidationError> {
    validate_graph(tasks)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::{TaskType, ValidationItem, ValidationStatus};

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

    fn make_validator(id: &str) -> Task {
        Task {
            id: id.to_string(),
            parent: None,
            preconditions: vec![],
            validations: vec![],
            title: None,
            validator: true,
            complete: false,
            in_progress: None,
            task_type: TaskType::Feature,
            description: String::new(),
        }
    }

    fn validation(id: &str) -> ValidationItem {
        ValidationItem {
            id: id.to_string(),
            status: ValidationStatus::Pending,
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
        task.validations = vec![validation("validator")];

        let result = form_graph(vec![validator, task]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validation_points_to_nonexistent() {
        let mut task = make_task("task");
        task.validations = vec![validation("nonexistent")];

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
        task.validations = vec![validation("not-validator")];

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
        task.validations = vec![validation("validator")];

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
        task1.validations = vec![validation("validator1"), validation("validator2")];

        let result = form_graph(vec![validator1, validator2, task2, task3, task1]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 5);
    }
}
