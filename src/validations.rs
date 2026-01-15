use std::collections::HashMap;
use thiserror::Error;

use crate::task::Task;

pub type TaskGraph = HashMap<String, Task>;

#[derive(Error, Debug, PartialEq)]
pub enum ValidationError {
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

/// Validates a single task's references against a graph.
///
/// Checks that:
/// - Parent reference points to an existing task
/// - All precondition references point to existing tasks
/// - Non-validator tasks cannot have validators as preconditions
/// - All validation references point to tasks marked as validators
/// - All validation references point to root validators (no parents)
pub fn validate_task(task: &Task, graph: &TaskGraph) -> Result<(), ValidationError> {
    if let Some(parent_id) = &task.parent {
        if !graph.contains_key(parent_id) {
            return Err(ValidationError::InvalidParent {
                task_id: task.id.clone(),
                parent_id: parent_id.clone(),
            });
        }
    }

    for precondition_id in &task.preconditions {
        let Some(precondition) = graph.get(precondition_id) else {
            return Err(ValidationError::InvalidPrecondition {
                task_id: task.id.clone(),
                precondition_id: precondition_id.clone(),
            });
        };

        if !task.validator && precondition.validator {
            return Err(ValidationError::PreconditionIsValidator {
                task_id: task.id.clone(),
                precondition_id: precondition_id.clone(),
            });
        }
    }

    for validation_id in &task.validations {
        let Some(validator) = graph.get(validation_id) else {
            return Err(ValidationError::InvalidValidation {
                task_id: task.id.clone(),
                validation_id: validation_id.clone(),
            });
        };

        if !validator.validator {
            return Err(ValidationError::InvalidValidation {
                task_id: task.id.clone(),
                validation_id: validation_id.clone(),
            });
        }

        if validator.parent.is_some() {
            return Err(ValidationError::ValidationNotRootValidator {
                task_id: task.id.clone(),
                validation_id: validation_id.clone(),
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

    let neighbors = task.parent.iter().chain(task.preconditions.iter());
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
    fn test_validate_task_valid() {
        let parent = make_task("parent");
        let validator = make_validator("validator");
        let mut task = make_task("task");
        task.parent = Some("parent".to_string());
        task.validations = vec!["validator".to_string()];

        let mut graph = TaskGraph::new();
        graph.insert("parent".to_string(), parent);
        graph.insert("validator".to_string(), validator);
        graph.insert("task".to_string(), task.clone());

        assert!(validate_task(&task, &graph).is_ok());
    }

    #[test]
    fn test_validate_task_invalid_parent() {
        let mut task = make_task("task");
        task.parent = Some("nonexistent".to_string());

        let mut graph = TaskGraph::new();
        graph.insert("task".to_string(), task.clone());

        assert_eq!(
            validate_task(&task, &graph),
            Err(ValidationError::InvalidParent {
                task_id: "task".to_string(),
                parent_id: "nonexistent".to_string(),
            })
        );
    }

    #[test]
    fn test_validate_task_invalid_precondition() {
        let mut task = make_task("task");
        task.preconditions = vec!["nonexistent".to_string()];

        let mut graph = TaskGraph::new();
        graph.insert("task".to_string(), task.clone());

        assert_eq!(
            validate_task(&task, &graph),
            Err(ValidationError::InvalidPrecondition {
                task_id: "task".to_string(),
                precondition_id: "nonexistent".to_string(),
            })
        );
    }

    #[test]
    fn test_validate_task_precondition_is_validator() {
        let validator = make_validator("validator");
        let mut task = make_task("task");
        task.preconditions = vec!["validator".to_string()];

        let mut graph = TaskGraph::new();
        graph.insert("validator".to_string(), validator);
        graph.insert("task".to_string(), task.clone());

        assert_eq!(
            validate_task(&task, &graph),
            Err(ValidationError::PreconditionIsValidator {
                task_id: "task".to_string(),
                precondition_id: "validator".to_string(),
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
        a.parent = Some("b".to_string());
        b.parent = Some("a".to_string());

        let result = validate_graph(vec![a, b]);
        assert_eq!(result, Err(ValidationError::CycleDetected));
    }

    #[test]
    fn test_validate_graph_valid() {
        let parent = make_task("parent");
        let mut child = make_task("child");
        child.parent = Some("parent".to_string());

        let result = validate_graph(vec![parent, child]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2);
    }
}
