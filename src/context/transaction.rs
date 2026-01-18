use super::task::Task;

/// An operation to be applied to the task graph.
#[derive(Debug, Clone)]
pub enum Op {
    /// Insert or update a task.
    Upsert(Task),
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

    /// Add an upsert operation.
    ///
    /// If the task has an empty ID, one will be generated during commit.
    pub fn upsert(&mut self, task: Task) {
        self.ops.push(Op::Upsert(task));
    }

    /// Add a delete operation.
    pub fn delete(&mut self, id: impl Into<String>) {
        self.ops.push(Op::Delete(id.into()));
    }
}
