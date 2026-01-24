use std::collections::HashMap;
use thiserror::Error;

use super::task::Task;
use super::view::GraphView;

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

/// Validates a GraphView (TaskGraph, ValidationView, or any other implementation).
///
/// Checks that:
/// - All task references (before, after, validations) point to existing tasks
/// - Non-gate tasks cannot have gates as after dependencies
/// - Validation references point to root gates (gates without before targets)
/// - The graph forms a DAG (no cycles)
///
/// Deleted tasks are skipped and not validated.
pub fn validate_view<V: GraphView>(view: &V) -> Result<(), ValidationError> {
    for task in view.values() {
        validate_task_in_view(task, view)?;
    }

    if has_cycle(view) {
        return Err(ValidationError::CycleDetected);
    }

    Ok(())
}

/// Validates a single task's references against a GraphView.
fn validate_task_in_view<V: GraphView>(task: &Task, view: &V) -> Result<(), ValidationError> {
    // Skip validation of deleted tasks
    if task.is_deleted() {
        return Ok(());
    }

    for before_id in &task.before {
        if view.get(before_id).is_none() {
            return Err(ValidationError::InvalidBefore {
                task_id: task.id.clone(),
                before_id: before_id.clone(),
            });
        }
    }

    for after_id in &task.after {
        let Some(after_task) = view.get(after_id) else {
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

    for validation in &task.gates {
        let Some(gate) = view.get(&validation.id) else {
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

#[derive(Clone, Copy, PartialEq)]
enum Color {
    White,
    Gray,
    Black,
}

fn has_cycle<V: GraphView>(view: &V) -> bool {
    let mut colors: HashMap<String, Color> = HashMap::new();
    for id in view.keys() {
        colors.insert(id.to_string(), Color::White);
    }

    for id in view.keys() {
        if colors[id] == Color::White && dfs_cycle(view, id, &mut colors) {
            return true;
        }
    }

    false
}

fn dfs_cycle<V: GraphView>(view: &V, task_id: &str, colors: &mut HashMap<String, Color>) -> bool {
    colors.insert(task_id.to_string(), Color::Gray);

    let Some(task) = view.get(task_id) else {
        return false;
    };

    let neighbors = task.before.iter().chain(task.after.iter());
    for neighbor_id in neighbors {
        let neighbor_color = colors.get(neighbor_id.as_str()).copied().unwrap_or(Color::Black);

        match neighbor_color {
            Color::Gray => return true,
            Color::White => {
                if dfs_cycle(view, neighbor_id, colors) {
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
    use crate::context::graph::TaskGraph;
    use crate::context::task::{TaskType, GateItem, GateStatus};

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

    #[test]
    fn test_validate_view_valid() {
        let before_target = make_task("before-target");
        let gate = make_gate("gate");
        let mut task = make_task("task");
        task.before = vec!["before-target".to_string()];
        task.gates = vec![GateItem {
            id: "gate".to_string(),
            status: GateStatus::Pending,
        }];

        let mut graph = TaskGraph::new();
        graph.insert(before_target);
        graph.insert(gate);
        graph.insert(task);

        assert!(validate_view(&graph).is_ok());
    }

    #[test]
    fn test_validate_view_invalid_before() {
        let mut task = make_task("task");
        task.before = vec!["nonexistent".to_string()];

        let mut graph = TaskGraph::new();
        graph.insert(task);

        assert_eq!(
            validate_view(&graph),
            Err(ValidationError::InvalidBefore {
                task_id: "task".to_string(),
                before_id: "nonexistent".to_string(),
            })
        );
    }

    #[test]
    fn test_validate_view_invalid_after() {
        let mut task = make_task("task");
        task.after = vec!["nonexistent".to_string()];

        let mut graph = TaskGraph::new();
        graph.insert(task);

        assert_eq!(
            validate_view(&graph),
            Err(ValidationError::InvalidAfter {
                task_id: "task".to_string(),
                after_id: "nonexistent".to_string(),
            })
        );
    }

    #[test]
    fn test_validate_view_after_is_gate() {
        let gate = make_gate("gate");
        let mut task = make_task("task");
        task.after = vec!["gate".to_string()];

        let mut graph = TaskGraph::new();
        graph.insert(gate);
        graph.insert(task);

        assert_eq!(
            validate_view(&graph),
            Err(ValidationError::AfterIsGate {
                task_id: "task".to_string(),
                after_id: "gate".to_string(),
            })
        );
    }

    #[test]
    fn test_validate_view_cycle() {
        let mut a = make_task("a");
        let mut b = make_task("b");
        a.before = vec!["b".to_string()];
        b.before = vec!["a".to_string()];

        let mut graph = TaskGraph::new();
        graph.insert(a);
        graph.insert(b);

        assert_eq!(validate_view(&graph), Err(ValidationError::CycleDetected));
    }

    #[test]
    fn test_validate_view_valid_dag() {
        let before_target = make_task("before-target");
        let mut child = make_task("child");
        child.before = vec!["before-target".to_string()];

        let mut graph = TaskGraph::new();
        graph.insert(before_target);
        graph.insert(child);

        assert!(validate_view(&graph).is_ok());
    }

    #[test]
    fn test_validate_view_reference_to_deleted_before() {
        let mut target = make_task("target");
        target.deleted = true;

        let mut task = make_task("task");
        task.before = vec!["target".to_string()];

        let mut graph = TaskGraph::new();
        graph.insert(target);
        graph.insert(task);

        assert_eq!(
            validate_view(&graph),
            Err(ValidationError::InvalidBefore {
                task_id: "task".to_string(),
                before_id: "target".to_string(),
            })
        );
    }

    #[test]
    fn test_validate_view_reference_to_deleted_after() {
        let mut dep = make_task("dep");
        dep.deleted = true;

        let mut task = make_task("task");
        task.after = vec!["dep".to_string()];

        let mut graph = TaskGraph::new();
        graph.insert(dep);
        graph.insert(task);

        assert_eq!(
            validate_view(&graph),
            Err(ValidationError::InvalidAfter {
                task_id: "task".to_string(),
                after_id: "dep".to_string(),
            })
        );
    }

    #[test]
    fn test_validate_view_deleted_task_is_skipped() {
        // A deleted task should pass validation even with invalid refs
        let mut task = make_task("task");
        task.before = vec!["nonexistent".to_string()];
        task.deleted = true;

        let mut graph = TaskGraph::new();
        graph.insert(task);

        // Should pass because deleted tasks are skipped
        assert!(validate_view(&graph).is_ok());
    }

    #[test]
    fn test_validate_view_reference_to_deleted_validation() {
        let mut gate = make_gate("gate");
        gate.deleted = true;

        let mut task = make_task("task");
        task.gates = vec![GateItem {
            id: "gate".to_string(),
            status: GateStatus::Pending,
        }];

        let mut graph = TaskGraph::new();
        graph.insert(gate);
        graph.insert(task);

        assert_eq!(
            validate_view(&graph),
            Err(ValidationError::ValidationNotFound {
                task_id: "task".to_string(),
                validation_id: "gate".to_string(),
            })
        );
    }
}
