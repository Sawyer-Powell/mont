use super::task::Task;
use super::view::GraphView;

/// An operation to be applied to the task graph.
#[derive(Debug, Clone)]
pub enum Op {
    /// Insert a new task.
    Insert(Task),
    /// Update an existing task (old_id, new task state).
    Update { old_id: String, task: Task },
    /// Delete a task by ID.
    Delete(String),
}

/// A transaction accumulating operations to be applied atomically.
///
/// Operations are validated together when the transaction is committed.
/// If validation fails, no changes are applied.
#[derive(Debug)]
pub struct Transaction {
    base_version: u64,
    ops: Vec<Op>,
}

impl Transaction {
    /// Create a new transaction with the given base version.
    pub(super) fn new(base_version: u64) -> Self {
        Self {
            base_version,
            ops: Vec::new(),
        }
    }

    /// Get the base version this transaction was created from.
    pub fn base_version(&self) -> u64 {
        self.base_version
    }

    /// Get the operations in this transaction.
    pub fn ops(&self) -> &[Op] {
        &self.ops
    }

    /// Consume the transaction and return the operations.
    pub fn into_ops(self) -> Vec<Op> {
        self.ops
    }

    /// Add an insert operation.
    ///
    /// If the task has an empty ID, one will be generated during commit.
    pub fn insert(&mut self, task: Task) {
        self.ops.push(Op::Insert(task));
    }

    /// Add an update operation.
    ///
    /// Updates an existing task identified by `old_id` to the new state in `task`.
    /// If `task.id` differs from `old_id`, this also renames the task.
    pub fn update(&mut self, old_id: impl Into<String>, task: Task) {
        self.ops.push(Op::Update {
            old_id: old_id.into(),
            task,
        });
    }

    /// Add a delete operation.
    pub fn delete(&mut self, id: impl Into<String>) {
        self.ops.push(Op::Delete(id.into()));
    }

    /// Add update ops to rewrite all references from `old_id` to `new_id`.
    ///
    /// If `new_id` is `Some`, replaces all references with the new ID.
    /// If `new_id` is `None`, removes all references entirely.
    ///
    /// This adds Update operations for all tasks in `graph` that reference `old_id`
    /// in their `before`, `after`, or `validations` fields.
    pub fn rewrite_references(
        &mut self,
        graph: &impl GraphView,
        old_id: &str,
        new_id: Option<&str>,
    ) {
        for task in graph.values() {
            if task.id == old_id {
                continue;
            }

            let has_ref = task.before.iter().any(|b| b == old_id)
                || task.after.iter().any(|a| a == old_id)
                || task.validations.iter().any(|v| v.id == old_id);

            if has_ref {
                let mut updated = task.clone();
                match new_id {
                    Some(new) => {
                        for before in &mut updated.before {
                            if before == old_id {
                                *before = new.to_string();
                            }
                        }
                        for after in &mut updated.after {
                            if after == old_id {
                                *after = new.to_string();
                            }
                        }
                        for validation in &mut updated.validations {
                            if validation.id == old_id {
                                validation.id = new.to_string();
                            }
                        }
                    }
                    None => {
                        updated.before.retain(|b| b != old_id);
                        updated.after.retain(|a| a != old_id);
                        updated.validations.retain(|v| v.id != old_id);
                    }
                }
                self.update(&task.id, updated);
            }
        }
    }
}
