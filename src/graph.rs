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
/// - All after dependencies are complete
/// - All subtasks are complete (tasks that have this task as before target)
pub fn available_tasks(graph: &TaskGraph) -> Vec<&Task> {
    graph
        .values()
        .filter(|task| !task.complete && !task.validator && is_available(task, graph))
        .collect()
}

/// Check if a specific task is available to work on.
pub fn is_available(task: &Task, graph: &TaskGraph) -> bool {
    // Check all after dependencies are complete
    for after_id in &task.after {
        if let Some(after_task) = graph.get(after_id)
            && !after_task.complete
        {
            return false;
        }
    }

    // Check all subtasks are complete (tasks that have this task as before target)
    for other_task in graph.values() {
        for before_id in &other_task.before {
            if before_id == &task.id && !other_task.complete {
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

    for before_id in &task.before {
        if let Some(before_target) = graph.get(before_id)
            && !is_group_complete(before_target, graph)
        {
            return false;
        }
    }

    true
}

/// Returns connected components of tasks using union-find.
/// Tasks are connected if they share before, after, or validation relationships.
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

        for before_id in &task.before {
            if let Some(&before_idx) = id_to_idx.get(before_id.as_str()) {
                union(&mut parent, task_idx, before_idx);
            }
        }

        for after_id in &task.after {
            if let Some(&after_idx) = id_to_idx.get(after_id.as_str()) {
                union(&mut parent, task_idx, after_idx);
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
/// Subtasks come before their before targets, after dependencies come before tasks that depend on them.
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

        // After: this task depends on after dependency completing
        for after_id in &task.after {
            if task_ids.contains(after_id.as_str()) {
                dependents
                    .entry(after_id.as_str())
                    .or_default()
                    .push(&task.id);
                *in_degree.entry(task.id.as_str()).or_insert(0) += 1;
            }
        }

        // Before: before targets depend on this task completing (subtask before target)
        for before_id in &task.before {
            if task_ids.contains(before_id.as_str()) {
                dependents
                    .entry(task.id.as_str())
                    .or_default()
                    .push(before_id.as_str());
                *in_degree.entry(before_id.as_str()).or_insert(0) += 1;
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

        // Before relationship: before targets depend on this task
        for before_id in &task.before {
            if task_ids.contains(before_id.as_str()) {
                edges
                    .entry(task.id.as_str())
                    .or_default()
                    .insert(before_id.as_str());
            }
        }

        // After relationship: this task depends on after dependency
        for after_id in &task.after {
            if task_ids.contains(after_id.as_str()) {
                edges
                    .entry(after_id.as_str())
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
                    other != succ && reachable.get(other).is_some_and(|r| r.contains(succ))
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
/// - All before references point to valid tasks
/// - All after references point to valid tasks
/// - Non-validator tasks cannot have validators as after dependencies
/// - All validation references point to tasks marked as validators
/// - All validation references point to root validators (no before target)
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
            before: vec![],
            after: vec![],
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
            before: vec![],
            after: vec![],
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
    fn test_valid_before() {
        let before_target = make_task("before-target");
        let mut subtask = make_task("subtask");
        subtask.before = vec!["before-target".to_string()];

        let result = form_graph(vec![before_target, subtask]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_invalid_before() {
        let mut task = make_task("task-1");
        task.before = vec!["nonexistent".to_string()];

        let result = form_graph(vec![task]);
        assert_eq!(
            result,
            Err(GraphError::InvalidBefore {
                task_id: "task-1".to_string(),
                before_id: "nonexistent".to_string(),
            })
        );
    }

    #[test]
    fn test_valid_after() {
        let dep = make_task("dep");
        let mut task = make_task("task");
        task.after = vec!["dep".to_string()];

        let result = form_graph(vec![dep, task]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_invalid_after() {
        let mut task = make_task("task-1");
        task.after = vec!["nonexistent".to_string()];

        let result = form_graph(vec![task]);
        assert_eq!(
            result,
            Err(GraphError::InvalidAfter {
                task_id: "task-1".to_string(),
                after_id: "nonexistent".to_string(),
            })
        );
    }

    #[test]
    fn test_after_is_validator() {
        let validator = make_validator("validator");
        let mut task = make_task("task");
        task.after = vec!["validator".to_string()];

        let result = form_graph(vec![validator, task]);
        assert_eq!(
            result,
            Err(GraphError::AfterIsValidator {
                task_id: "task".to_string(),
                after_id: "validator".to_string(),
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
            Err(GraphError::ValidationNotFound {
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
        let before_target = make_task("before-target");
        let mut validator = make_validator("validator");
        validator.before = vec!["before-target".to_string()];
        let mut task = make_task("task");
        task.validations = vec![validation("validator")];

        let result = form_graph(vec![before_target, validator, task]);
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
        // A -> B -> C (each subtask has one before target)
        let mut a = make_task("a");
        let mut b = make_task("b");
        let c = make_task("c");
        b.before = vec!["c".to_string()];
        a.before = vec!["b".to_string()];

        let result = form_graph(vec![a, b, c]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cycle_self_loop() {
        let mut task = make_task("task");
        task.before = vec!["task".to_string()];

        let result = form_graph(vec![task]);
        assert_eq!(result, Err(GraphError::CycleDetected));
    }

    #[test]
    fn test_cycle_two_nodes() {
        let mut a = make_task("a");
        let mut b = make_task("b");
        a.before = vec!["b".to_string()];
        b.before = vec!["a".to_string()];

        let result = form_graph(vec![a, b]);
        assert_eq!(result, Err(GraphError::CycleDetected));
    }

    #[test]
    fn test_cycle_three_nodes() {
        let mut a = make_task("a");
        let mut b = make_task("b");
        let mut c = make_task("c");
        a.before = vec!["b".to_string()];
        b.before = vec!["c".to_string()];
        c.before = vec!["a".to_string()];

        let result = form_graph(vec![a, b, c]);
        assert_eq!(result, Err(GraphError::CycleDetected));
    }

    #[test]
    fn test_cycle_via_after() {
        let mut a = make_task("a");
        let mut b = make_task("b");
        a.after = vec!["b".to_string()];
        b.after = vec!["a".to_string()];

        let result = form_graph(vec![a, b]);
        assert_eq!(result, Err(GraphError::CycleDetected));
    }

    #[test]
    fn test_cycle_mixed_before_and_after() {
        let mut a = make_task("a");
        let mut b = make_task("b");
        let mut c = make_task("c");
        a.before = vec!["b".to_string()];
        b.after = vec!["c".to_string()];
        c.before = vec!["a".to_string()];

        let result = form_graph(vec![a, b, c]);
        assert_eq!(result, Err(GraphError::CycleDetected));
    }

    #[test]
    fn test_complex_valid_graph() {
        // validator1, validator2 (root validators)
        // task1 has before target task2
        // task1 has after dependency on task3
        // task1 validates against validator1 and validator2
        let validator1 = make_validator("validator1");
        let validator2 = make_validator("validator2");
        let task2 = make_task("task2");
        let task3 = make_task("task3");
        let mut task1 = make_task("task1");
        task1.before = vec!["task2".to_string()];
        task1.after = vec!["task3".to_string()];
        task1.validations = vec![validation("validator1"), validation("validator2")];

        let result = form_graph(vec![validator1, validator2, task2, task3, task1]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 5);
    }
}
