use std::collections::{HashMap, HashSet};

use super::graph::TaskGraph;
use super::task::Task;
use super::transaction::Op;

/// Trait for types that provide a view of the task graph.
///
/// This allows validation logic to work with both the actual TaskGraph
/// and a ValidationView that shows proposed changes.
pub trait GraphView {
    /// Get a task by ID.
    fn get(&self, id: &str) -> Option<&Task>;

    /// Check if a task exists.
    fn contains(&self, id: &str) -> bool {
        self.get(id).is_some()
    }

    /// Iterate over all tasks.
    fn values(&self) -> Box<dyn Iterator<Item = &Task> + '_>;

    /// Get all task IDs.
    fn keys(&self) -> Box<dyn Iterator<Item = &str> + '_>;

    /// Get the number of tasks.
    fn len(&self) -> usize {
        self.values().count()
    }

    /// Check if the graph is empty.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl GraphView for TaskGraph {
    fn get(&self, id: &str) -> Option<&Task> {
        TaskGraph::get(self, id).filter(|t| !t.is_deleted())
    }

    fn values(&self) -> Box<dyn Iterator<Item = &Task> + '_> {
        Box::new(TaskGraph::values(self).filter(|t| !t.is_deleted()))
    }

    fn keys(&self) -> Box<dyn Iterator<Item = &str> + '_> {
        Box::new(
            TaskGraph::iter(self)
                .filter(|(_, t)| !t.is_deleted())
                .map(|(k, _)| k.as_str()),
        )
    }

    fn len(&self) -> usize {
        TaskGraph::values(self).filter(|t| !t.is_deleted()).count()
    }
}

/// A lightweight view of the task graph with proposed changes overlaid.
///
/// This allows validation to see what the graph would look like after
/// applying a transaction, without actually modifying the graph.
pub struct ValidationView<'a> {
    base: &'a TaskGraph,
    inserts: HashMap<String, &'a Task>,
    deletes: HashSet<&'a str>,
}

impl<'a> ValidationView<'a> {
    /// Create a new validation view by overlaying operations on the base graph.
    pub fn new(base: &'a TaskGraph, ops: &'a [Op]) -> Self {
        let mut inserts: HashMap<String, &'a Task> = HashMap::new();
        let mut deletes: HashSet<&'a str> = HashSet::new();

        for op in ops {
            match op {
                Op::Insert(task) => {
                    deletes.remove(task.id.as_str());
                    inserts.insert(task.id.clone(), task);
                }
                Op::Update { old_id, task } => {
                    // If ID changed, mark old as deleted
                    if old_id != &task.id {
                        inserts.remove(old_id);
                        deletes.insert(old_id.as_str());
                    }
                    // Insert the updated task
                    deletes.remove(task.id.as_str());
                    inserts.insert(task.id.clone(), task);
                }
                Op::Delete(id) => {
                    inserts.remove(id);
                    deletes.insert(id.as_str());
                }
            }
        }

        Self {
            base,
            inserts,
            deletes,
        }
    }
}

impl GraphView for ValidationView<'_> {
    fn get(&self, id: &str) -> Option<&Task> {
        // Explicitly deleted in this transaction
        if self.deletes.contains(id) {
            return None;
        }
        // Upserted in this transaction (overrides base)
        if let Some(task) = self.inserts.get(id) {
            return Some(task);
        }
        // Fall back to base (excluding soft-deleted)
        self.base.get(id).filter(|t| !t.is_deleted())
    }

    fn values(&self) -> Box<dyn Iterator<Item = &Task> + '_> {
        // Base tasks: not soft-deleted, not txn-deleted, not overridden by upsert
        let base_tasks = self
            .base
            .values()
            .filter(|t| !t.is_deleted())
            .filter(|t| !self.deletes.contains(t.id.as_str()))
            .filter(|t| !self.inserts.contains_key(&t.id));

        // Plus all inserted tasks
        let inserted = self.inserts.values().copied();

        Box::new(base_tasks.chain(inserted))
    }

    fn keys(&self) -> Box<dyn Iterator<Item = &str> + '_> {
        // Base task IDs: not soft-deleted, not txn-deleted, not overridden
        let base_keys = self
            .base
            .iter()
            .filter(|(_, t)| !t.is_deleted())
            .filter(|(id, _)| !self.deletes.contains(id.as_str()))
            .filter(|(id, _)| !self.inserts.contains_key(*id))
            .map(|(id, _)| id.as_str());

        // Plus inserted task IDs
        let inserted_keys = self.inserts.keys().map(|s| s.as_str());

        Box::new(base_keys.chain(inserted_keys))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::task::TaskType;

    fn make_task(id: &str) -> Task {
        Task {
            id: id.to_string(),
            new_id: None,
            before: vec![],
            after: vec![],
            gates: vec![],
            title: Some(format!("{} title", id)),
            status: None,
            task_type: TaskType::Task,
            description: String::new(),
            deleted: false,
        }
    }

    #[test]
    fn test_view_upsert_new_task() {
        let graph = TaskGraph::new();
        let task = make_task("new-task");
        let ops = vec![Op::Insert(task)];

        let view = ValidationView::new(&graph, &ops);

        assert!(view.contains("new-task"));
        assert_eq!(view.len(), 1);
    }

    #[test]
    fn test_view_upsert_overrides_base() {
        let mut graph = TaskGraph::new();
        let mut original = make_task("task1");
        original.title = Some("Original".to_string());
        graph.insert(original);
        graph.clear_dirty();

        let mut updated = make_task("task1");
        updated.title = Some("Updated".to_string());
        let ops = vec![Op::Insert(updated)];

        let view = ValidationView::new(&graph, &ops);

        let task = view.get("task1").unwrap();
        assert_eq!(task.title, Some("Updated".to_string()));
        assert_eq!(view.len(), 1);
    }

    #[test]
    fn test_view_delete_removes_from_base() {
        let mut graph = TaskGraph::new();
        graph.insert(make_task("task1"));
        graph.insert(make_task("task2"));
        graph.clear_dirty();

        let ops = vec![Op::Delete("task1".to_string())];

        let view = ValidationView::new(&graph, &ops);

        assert!(!view.contains("task1"));
        assert!(view.contains("task2"));
        assert_eq!(view.len(), 1);
    }

    #[test]
    fn test_view_delete_then_upsert() {
        let mut graph = TaskGraph::new();
        graph.insert(make_task("task1"));
        graph.clear_dirty();

        let ops = vec![
            Op::Delete("task1".to_string()),
            Op::Insert(make_task("task1")),
        ];

        let view = ValidationView::new(&graph, &ops);

        // Task should exist (upsert after delete)
        assert!(view.contains("task1"));
    }

    #[test]
    fn test_view_upsert_then_delete() {
        let graph = TaskGraph::new();

        let ops = vec![
            Op::Insert(make_task("task1")),
            Op::Delete("task1".to_string()),
        ];

        let view = ValidationView::new(&graph, &ops);

        // Task should not exist (delete after upsert)
        assert!(!view.contains("task1"));
        assert_eq!(view.len(), 0);
    }
}
