use std::collections::HashMap;
use thiserror::Error;

use crate::task::Task;

pub type TaskGraph = HashMap<String, Task>;

#[derive(Error, Debug, PartialEq)]
pub enum GraphError {
    #[error("task '{task_id}' references invalid parent '{parent_id}'")]
    InvalidParent { task_id: String, parent_id: String },
    #[error("task '{task_id}' references invalid precondition '{precondition_id}'")]
    InvalidPrecondition {
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
            if !graph.contains_key(precondition_id) {
                return Err(GraphError::InvalidPrecondition {
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

    fn make_task(id: &str) -> Task {
        Task {
            id: id.to_string(),
            parent: None,
            preconditions: vec![],
            validations: vec![],
            title: None,
            validator: false,
            complete: false,
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
