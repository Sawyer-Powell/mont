use std::collections::HashMap;
use thiserror::Error;

use crate::task::Task;

pub type TaskGraph = HashMap<String, Task>;

#[derive(Error, Debug, PartialEq)]
pub enum ValidationError {
    #[error("task '{task_id}' references invalid before target '{before_id}'")]
    InvalidBefore { task_id: String, before_id: String },
    #[error("task '{task_id}' references invalid after dependency '{after_id}'")]
    InvalidAfter {
        task_id: String,
        after_id: String,
    },
    #[error("task '{task_id}' has gate '{after_id}' as after dependency (use validations instead)")]
    AfterIsGate {
        task_id: String,
        after_id: String,
    },
    #[error("task '{task_id}' references non-existent validation '{validation_id}'")]
    ValidationNotFound {
        task_id: String,
        validation_id: String,
    },
    #[error("task '{task_id}' references validation '{validation_id}' which is not a gate")]
    InvalidValidation {
        task_id: String,
        validation_id: String,
    },
    #[error("task '{task_id}' references gate '{validation_id}' which has a before target (must be root gate)")]
    ValidationNotRootGate {
        task_id: String,
        validation_id: String,
    },
    #[error("cycle detected in task graph")]
    CycleDetected,
    #[error("duplicate task id '{0}'")]
    DuplicateTaskId(String),
}

/// Validates a single task's references against a graph.
///
/// Checks that:
/// - Before reference points to an existing task
/// - All after references point to existing tasks
/// - Non-gate tasks cannot have gates as after dependencies
/// - All validation references point to tasks marked as gates
/// - All validation references point to root gates (no before target)
pub fn validate_task(task: &Task, graph: &TaskGraph) -> Result<(), ValidationError> {
    for before_id in &task.before {
        if !graph.contains_key(before_id) {
            return Err(ValidationError::InvalidBefore {
                task_id: task.id.clone(),
                before_id: before_id.clone(),
            });
        }
    }

    for after_id in &task.after {
        let Some(after_task) = graph.get(after_id) else {
            return Err(ValidationError::InvalidAfter {
                task_id: task.id.clone(),
                after_id: after_id.clone(),
            });
        };

        if !task.is_gate() && after_task.is_gate() {
            return Err(ValidationError::AfterIsGate {
                task_id: task.id.clone(),
                after_id: after_id.clone(),
            });
        }
    }

    for validation in &task.validations {
        let Some(gate) = graph.get(&validation.id) else {
            return Err(ValidationError::ValidationNotFound {
                task_id: task.id.clone(),
                validation_id: validation.id.clone(),
            });
        };

        if !gate.is_gate() {
            return Err(ValidationError::InvalidValidation {
                task_id: task.id.clone(),
                validation_id: validation.id.clone(),
            });
        }

        if !gate.before.is_empty() {
            return Err(ValidationError::ValidationNotRootGate {
                task_id: task.id.clone(),
                validation_id: validation.id.clone(),
            });
        }
    }

    Ok(())
}

/// Validates the entire task graph.
///
/// Validates:
/// - No duplicate task IDs
/// - All task references are valid (via validate_task)
/// - The graph forms a DAG (no cycles)
pub fn validate_graph(tasks: Vec<Task>) -> Result<TaskGraph, ValidationError> {
    let mut graph: TaskGraph = HashMap::new();

    for task in tasks {
        if graph.contains_key(&task.id) {
            return Err(ValidationError::DuplicateTaskId(task.id));
        }
        graph.insert(task.id.clone(), task);
    }

    for task in graph.values() {
        validate_task(task, &graph)?;
    }

    if has_cycle(&graph) {
        return Err(ValidationError::CycleDetected);
    }

    Ok(graph)
}

#[derive(Clone, Copy, PartialEq)]
enum Color {
    White,
    Gray,
    Black,
}

fn has_cycle(graph: &TaskGraph) -> bool {
    let mut colors: HashMap<String, Color> = HashMap::new();
    for id in graph.keys() {
        colors.insert(id.clone(), Color::White);
    }

    for id in graph.keys() {
        if colors[id] == Color::White && dfs_cycle(graph, id, &mut colors) {
            return true;
        }
    }

    false
}

fn dfs_cycle(graph: &TaskGraph, task_id: &str, colors: &mut HashMap<String, Color>) -> bool {
    colors.insert(task_id.to_string(), Color::Gray);

    let task = &graph[task_id];

    let neighbors = task.before.iter().chain(task.after.iter());
    for neighbor_id in neighbors {
        let neighbor_color = colors.get(neighbor_id).copied().unwrap_or(Color::Black);

        match neighbor_color {
            Color::Gray => return true,
            Color::White => {
                if dfs_cycle(graph, neighbor_id, colors) {
                    return true;
                }
            }
            Color::Black => {}
        }
    }

    colors.insert(task_id.to_string(), Color::Black);
    false
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
            complete: false,
            in_progress: None,
            task_type: TaskType::Task,
            description: String::new(),
        }
    }

    fn make_gate(id: &str) -> Task {
        Task {
            id: id.to_string(),
            before: vec![],
            after: vec![],
            validations: vec![],
            title: None,
            complete: false,
            in_progress: None,
            task_type: TaskType::Gate,
            description: String::new(),
        }
    }

    #[test]
    fn test_validate_task_valid() {
        let before_target = make_task("before-target");
        let gate = make_gate("gate");
        let mut task = make_task("task");
        task.before = vec!["before-target".to_string()];
        task.validations = vec![ValidationItem {
            id: "gate".to_string(),
            status: ValidationStatus::Pending,
        }];

        let mut graph = TaskGraph::new();
        graph.insert("before-target".to_string(), before_target);
        graph.insert("gate".to_string(), gate);
        graph.insert("task".to_string(), task.clone());

        assert!(validate_task(&task, &graph).is_ok());
    }

    #[test]
    fn test_validate_task_invalid_before() {
        let mut task = make_task("task");
        task.before = vec!["nonexistent".to_string()];

        let mut graph = TaskGraph::new();
        graph.insert("task".to_string(), task.clone());

        assert_eq!(
            validate_task(&task, &graph),
            Err(ValidationError::InvalidBefore {
                task_id: "task".to_string(),
                before_id: "nonexistent".to_string(),
            })
        );
    }

    #[test]
    fn test_validate_task_invalid_after() {
        let mut task = make_task("task");
        task.after = vec!["nonexistent".to_string()];

        let mut graph = TaskGraph::new();
        graph.insert("task".to_string(), task.clone());

        assert_eq!(
            validate_task(&task, &graph),
            Err(ValidationError::InvalidAfter {
                task_id: "task".to_string(),
                after_id: "nonexistent".to_string(),
            })
        );
    }

    #[test]
    fn test_validate_task_after_is_gate() {
        let gate = make_gate("gate");
        let mut task = make_task("task");
        task.after = vec!["gate".to_string()];

        let mut graph = TaskGraph::new();
        graph.insert("gate".to_string(), gate);
        graph.insert("task".to_string(), task.clone());

        assert_eq!(
            validate_task(&task, &graph),
            Err(ValidationError::AfterIsGate {
                task_id: "task".to_string(),
                after_id: "gate".to_string(),
            })
        );
    }

    #[test]
    fn test_validate_graph_duplicate_id() {
        let task1 = make_task("task");
        let task2 = make_task("task");

        let result = validate_graph(vec![task1, task2]);
        assert_eq!(
            result,
            Err(ValidationError::DuplicateTaskId("task".to_string()))
        );
    }

    #[test]
    fn test_validate_graph_cycle() {
        let mut a = make_task("a");
        let mut b = make_task("b");
        a.before = vec!["b".to_string()];
        b.before = vec!["a".to_string()];

        let result = validate_graph(vec![a, b]);
        assert_eq!(result, Err(ValidationError::CycleDetected));
    }

    #[test]
    fn test_validate_graph_valid() {
        let before_target = make_task("before-target");
        let mut child = make_task("child");
        child.before = vec!["before-target".to_string()];

        let result = validate_graph(vec![before_target, child]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2);
    }
}
