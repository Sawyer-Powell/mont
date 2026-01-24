use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use super::task::{ParseError, Task};
use super::validations::{validate_view, ValidationError};

/// Error collecting multiple issues found when reading a task graph.
///
/// This allows batch error reporting - all errors are collected and reported
/// together rather than failing on the first error.
#[derive(Debug, Default)]
pub struct GraphReadError {
    pub io_errors: Vec<(PathBuf, std::io::Error)>,
    pub parse_errors: Vec<(PathBuf, ParseError)>,
    pub validation_errors: Vec<ValidationError>,
}

impl GraphReadError {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.io_errors.is_empty() && self.parse_errors.is_empty() && self.validation_errors.is_empty()
    }

    pub fn add_io_error(&mut self, path: PathBuf, error: std::io::Error) {
        self.io_errors.push((path, error));
    }

    pub fn add_parse_error(&mut self, path: PathBuf, error: ParseError) {
        self.parse_errors.push((path, error));
    }

    pub fn add_validation_error(&mut self, error: ValidationError) {
        self.validation_errors.push(error);
    }

    pub fn error_count(&self) -> usize {
        self.io_errors.len() + self.parse_errors.len() + self.validation_errors.len()
    }
}

impl std::fmt::Display for GraphReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let count = self.error_count();
        writeln!(f, "Found {} error(s) while reading task graph:", count)?;

        for (path, error) in &self.io_errors {
            writeln!(f, "  IO error in {}: {}", path.display(), error)?;
        }

        for (path, error) in &self.parse_errors {
            writeln!(f, "  Parse error in {}: {}", path.display(), error)?;
        }

        for error in &self.validation_errors {
            writeln!(f, "  Validation error: {}", error)?;
        }

        Ok(())
    }
}

impl std::error::Error for GraphReadError {}

/// A graph of tasks with their dependencies.
///
/// Tracks which tasks have been modified ("dirty") since the last save.
#[derive(Debug, Clone, Default)]
pub struct TaskGraph {
    tasks: HashMap<String, Task>,
    dirty: HashSet<String>,
}

impl TaskGraph {
    pub fn new() -> Self {
        Self {
            tasks: HashMap::new(),
            dirty: HashSet::new(),
        }
    }

    /// Insert a task, marking it as dirty.
    pub fn insert(&mut self, task: Task) {
        self.dirty.insert(task.id.clone());
        self.tasks.insert(task.id.clone(), task);
    }

    pub fn get(&self, id: &str) -> Option<&Task> {
        self.tasks.get(id)
    }

    /// Get a mutable reference to a task, marking it as dirty.
    pub fn get_mut(&mut self, id: &str) -> Option<&mut Task> {
        if self.tasks.contains_key(id) {
            self.dirty.insert(id.to_string());
        }
        self.tasks.get_mut(id)
    }

    pub fn contains(&self, id: &str) -> bool {
        self.tasks.contains_key(id)
    }

    /// Mark a task as deleted (soft-delete).
    ///
    /// The task remains in the graph but is flagged as deleted and marked dirty.
    /// Returns true if the task existed and was marked deleted, false otherwise.
    pub fn remove(&mut self, id: &str) -> bool {
        if let Some(task) = self.tasks.get_mut(id) {
            task.deleted = true;
            self.dirty.insert(id.to_string());
            true
        } else {
            false
        }
    }

    pub fn len(&self) -> usize {
        self.tasks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.tasks.keys()
    }

    pub fn values(&self) -> impl Iterator<Item = &Task> {
        self.tasks.values()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &Task)> {
        self.tasks.iter()
    }

    pub fn retain<F>(&mut self, f: F)
    where
        F: FnMut(&String, &mut Task) -> bool,
    {
        self.tasks.retain(f)
    }

    // ---- Dirty tracking methods ----

    /// Mark a task as dirty by ID.
    pub fn mark_dirty(&mut self, id: &str) {
        self.dirty.insert(id.to_string());
    }

    /// Check if a specific task is dirty.
    pub fn is_dirty(&self, id: &str) -> bool {
        self.dirty.contains(id)
    }

    /// Check if any tasks are dirty.
    pub fn has_dirty(&self) -> bool {
        !self.dirty.is_empty()
    }

    /// Get iterator over dirty task IDs.
    pub fn dirty_ids(&self) -> impl Iterator<Item = &String> {
        self.dirty.iter()
    }

    /// Get all dirty tasks.
    pub fn dirty_tasks(&self) -> Vec<&Task> {
        self.dirty
            .iter()
            .filter_map(|id| self.tasks.get(id))
            .collect()
    }

    /// Clear all dirty flags and remove deleted tasks from the graph.
    ///
    /// Call this after saving to disk. Deleted tasks are purged from memory
    /// since their files have been removed.
    pub fn clear_dirty(&mut self) {
        self.dirty.clear();
        self.tasks.retain(|_, task| !task.deleted);
    }

    // ---- Graph Algorithm Methods ----

    /// Computes the transitive reduction of dependency edges.
    /// Returns a map from task_id to its effective successors after removing redundant edges.
    ///
    /// For example, if A → B → C and A → C, the edge A → C is redundant.
    /// After reduction, A's effective successors are just [B] (not [B, C]).
    pub fn transitive_reduction(&self) -> HashMap<&str, Vec<&str>> {
        let task_ids: HashSet<&str> = self.tasks.keys().map(|s| s.as_str()).collect();

        // Build dependency edges: from task to tasks that depend on it
        let mut edges: HashMap<&str, HashSet<&str>> = HashMap::new();

        for task in self.tasks.values() {
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

        for task in self.tasks.values() {
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

    /// Returns task IDs in topological order.
    ///
    /// Uses Kahn's algorithm with the transitive reduction of the graph.
    pub fn topological_order(&self) -> Vec<&str> {
        if self.tasks.is_empty() {
            return Vec::new();
        }

        let effective_successors = self.transitive_reduction();

        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        for id in self.tasks.keys() {
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
            result.push(task_id);

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

    /// Find connected components in the graph using union-find.
    ///
    /// Returns groups of task IDs where tasks in each group are connected
    /// through before/after/validation relationships.
    pub fn connected_components(&self) -> Vec<Vec<&str>> {
        if self.tasks.is_empty() {
            return Vec::new();
        }

        let task_ids: Vec<&str> = self.tasks.keys().map(|s| s.as_str()).collect();
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

        for task in self.tasks.values() {
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

            for validation in &task.gates {
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
    }
}

impl PartialEq for TaskGraph {
    fn eq(&self, other: &Self) -> bool {
        // Only compare tasks, not dirty state
        self.tasks == other.tasks
    }
}

impl FromIterator<Task> for TaskGraph {
    fn from_iter<I: IntoIterator<Item = Task>>(iter: I) -> Self {
        let tasks = iter.into_iter().map(|t| (t.id.clone(), t)).collect();
        Self { tasks, dirty: HashSet::new() }
    }
}

impl FromIterator<(String, Task)> for TaskGraph {
    fn from_iter<I: IntoIterator<Item = (String, Task)>>(iter: I) -> Self {
        Self { tasks: iter.into_iter().collect(), dirty: HashSet::new() }
    }
}


// ============================================================================
// Graph Query Operations
// ============================================================================

/// Returns tasks that are available to work on (all dependencies satisfied).
///
/// A task is available if:
/// - It is not complete
/// - It is not a gate
/// - All after dependencies are complete
/// - All subtasks are complete (tasks that have this task as before target)
pub fn available_tasks(graph: &TaskGraph) -> Vec<&Task> {
    graph
        .values()
        .filter(|task| !task.is_complete() && !task.is_gate() && is_available(task, graph))
        .collect()
}

/// Check if a specific task is available to work on.
pub fn is_available(task: &Task, graph: &TaskGraph) -> bool {
    // Check all after dependencies are complete
    for after_id in &task.after {
        if let Some(after_task) = graph.get(after_id)
            && !after_task.is_complete()
        {
            return false;
        }
    }

    // Check all subtasks are complete (tasks that have this task as before target)
    for other_task in graph.values() {
        for before_id in &other_task.before {
            if before_id == &task.id && !other_task.is_complete() {
                return false;
            }
        }
    }

    true
}

/// Check if a task belongs to a fully complete group.
/// A task is in a complete group if it and all its ancestors are complete.
pub fn is_group_complete(task: &Task, graph: &TaskGraph) -> bool {
    if !task.is_complete() {
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

/// Build a TaskGraph from a list of tasks and validate it.
///
/// Checks for duplicate IDs, validates all references, and ensures no cycles.
pub fn form_graph(tasks: Vec<Task>) -> Result<TaskGraph, ValidationError> {
    let mut graph = TaskGraph::new();

    for task in tasks {
        if graph.contains(&task.id) {
            return Err(ValidationError::DuplicateTaskId(task.id));
        }
        graph.insert(task);
    }

    validate_view(&graph)?;
    graph.clear_dirty();

    Ok(graph)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::task::{TaskType, GateItem, GateStatus};

    fn make_task(id: &str) -> Task {
        Task {
            id: id.to_string(),
            new_id: None,
            before: vec![],
            after: vec![],
            gates: vec![],
            title: None,
            status: None,
            task_type: TaskType::Task,
            description: String::new(),
            deleted: false,
        }
    }

    fn make_gate(id: &str) -> Task {
        Task {
            id: id.to_string(),
            new_id: None,
            before: vec![],
            after: vec![],
            gates: vec![],
            title: None,
            status: None,
            task_type: TaskType::Gate,
            description: String::new(),
            deleted: false,
        }
    }

    fn validation(id: &str) -> GateItem {
        GateItem {
            id: id.to_string(),
            status: GateStatus::Pending,
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
            Err(ValidationError::DuplicateTaskId("task-1".to_string()))
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
            Err(ValidationError::InvalidBefore {
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
            Err(ValidationError::InvalidAfter {
                task_id: "task-1".to_string(),
                after_id: "nonexistent".to_string(),
            })
        );
    }

    #[test]
    fn test_after_is_gate() {
        let gate = make_gate("gate");
        let mut task = make_task("task");
        task.after = vec!["gate".to_string()];

        let result = form_graph(vec![gate, task]);
        assert_eq!(
            result,
            Err(ValidationError::AfterIsGate {
                task_id: "task".to_string(),
                after_id: "gate".to_string(),
            })
        );
    }

    #[test]
    fn test_valid_validation() {
        let gate = make_gate("gate");
        let mut task = make_task("task");
        task.gates = vec![validation("gate")];

        let result = form_graph(vec![gate, task]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validation_points_to_nonexistent() {
        let mut task = make_task("task");
        task.gates = vec![validation("nonexistent")];

        let result = form_graph(vec![task]);
        assert_eq!(
            result,
            Err(ValidationError::ValidationNotFound {
                task_id: "task".to_string(),
                validation_id: "nonexistent".to_string(),
            })
        );
    }

    #[test]
    fn test_validation_points_to_non_validator() {
        let not_validator = make_task("not-validator");
        let mut task = make_task("task");
        task.gates = vec![validation("not-validator")];

        let result = form_graph(vec![not_validator, task]);
        assert_eq!(
            result,
            Err(ValidationError::InvalidValidation {
                task_id: "task".to_string(),
                validation_id: "not-validator".to_string(),
            })
        );
    }

    #[test]
    fn test_validation_not_root_gate() {
        let before_target = make_task("before-target");
        let mut gate = make_gate("gate");
        gate.before = vec!["before-target".to_string()];
        let mut task = make_task("task");
        task.gates = vec![validation("gate")];

        let result = form_graph(vec![before_target, gate, task]);
        assert_eq!(
            result,
            Err(ValidationError::ValidationNotRootGate {
                task_id: "task".to_string(),
                validation_id: "gate".to_string(),
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
        assert_eq!(result, Err(ValidationError::CycleDetected));
    }

    #[test]
    fn test_cycle_two_nodes() {
        let mut a = make_task("a");
        let mut b = make_task("b");
        a.before = vec!["b".to_string()];
        b.before = vec!["a".to_string()];

        let result = form_graph(vec![a, b]);
        assert_eq!(result, Err(ValidationError::CycleDetected));
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
        assert_eq!(result, Err(ValidationError::CycleDetected));
    }

    #[test]
    fn test_cycle_via_after() {
        let mut a = make_task("a");
        let mut b = make_task("b");
        a.after = vec!["b".to_string()];
        b.after = vec!["a".to_string()];

        let result = form_graph(vec![a, b]);
        assert_eq!(result, Err(ValidationError::CycleDetected));
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
        assert_eq!(result, Err(ValidationError::CycleDetected));
    }

    #[test]
    fn test_complex_valid_graph() {
        // gate1, gate2 (root gates)
        // task1 has before target task2
        // task1 has after dependency on task3
        // task1 validates against gate1 and gate2
        let gate1 = make_gate("gate1");
        let gate2 = make_gate("gate2");
        let task2 = make_task("task2");
        let task3 = make_task("task3");
        let mut task1 = make_task("task1");
        task1.before = vec!["task2".to_string()];
        task1.after = vec!["task3".to_string()];
        task1.gates = vec![validation("gate1"), validation("gate2")];

        let result = form_graph(vec![gate1, gate2, task2, task3, task1]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 5);
    }
}
