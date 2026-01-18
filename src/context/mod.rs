//! Core data model for the task graph system.
//!
//! This module contains:
//! - `Task` - Individual task with metadata and relationships
//! - `TaskGraph` - Collection of tasks with dirty tracking
//! - `MontContext` - Main interface for task graph operations
//! - `Transaction` - Atomic batch operations with validation
//! - Validation logic for ensuring graph integrity

pub(crate) mod graph;
mod task;
mod transaction;
pub(crate) mod validations;
mod view;

use std::path::PathBuf;
use std::sync::RwLock;

// Re-export public types
pub use graph::{GraphReadError, TaskGraph};
pub use task::{parse, ParseError, Status, Task, TaskType, ValidationItem, ValidationStatus};
pub use transaction::{Op, Transaction};
pub use validations::ValidationError;
pub use view::{GraphView, ValidationView};

/// Internal state protected by RwLock.
struct ContextInner {
    graph: TaskGraph,
    version: u64,
}

/// Central application context holding the task graph and configuration.
///
/// MontContext is the main entry point for all task operations. It manages:
/// - The task graph (tasks and their dependencies)
/// - The path to the tasks directory
/// - Thread-safe access via RwLock
///
/// All mutations go through transactions which validate before committing.
pub struct MontContext {
    inner: RwLock<ContextInner>,
    tasks_dir: PathBuf,
}

impl std::fmt::Debug for MontContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MontContext")
            .field("tasks_dir", &self.tasks_dir)
            .finish_non_exhaustive()
    }
}

impl MontContext {
    /// Create a new MontContext with an empty graph.
    pub fn new(tasks_dir: PathBuf) -> Self {
        Self {
            inner: RwLock::new(ContextInner {
                graph: TaskGraph::new(),
                version: 0,
            }),
            tasks_dir,
        }
    }

    /// Load a MontContext from a tasks directory.
    ///
    /// Reads all .md files from the directory, parses them, and validates
    /// the resulting graph. Uses batch error collection - all errors are
    /// gathered and returned together rather than failing on the first error.
    pub fn load(tasks_dir: PathBuf) -> Result<Self, GraphReadError> {
        let mut errors = GraphReadError::new();
        let mut tasks = Vec::new();

        // Read directory entries
        let entries = match std::fs::read_dir(&tasks_dir) {
            Ok(entries) => entries,
            Err(e) => {
                errors.add_io_error(tasks_dir.clone(), e);
                return Err(errors);
            }
        };

        // Collect and sort paths
        let mut paths: Vec<PathBuf> = entries
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|ext| ext == "md"))
            .collect();
        paths.sort();

        // Read and parse each file
        for path in paths {
            let content = match std::fs::read_to_string(&path) {
                Ok(content) => content,
                Err(e) => {
                    errors.add_io_error(path, e);
                    continue;
                }
            };

            match parse(&content) {
                Ok(parsed) => tasks.push(parsed),
                Err(e) => {
                    errors.add_parse_error(path, e);
                }
            }
        }

        // If we have IO or parse errors, return them before validation
        if !errors.is_empty() {
            return Err(errors);
        }

        // Validate and form the graph
        match graph::form_graph(tasks) {
            Ok(graph) => Ok(Self {
                inner: RwLock::new(ContextInner { graph, version: 0 }),
                tasks_dir,
            }),
            Err(e) => {
                errors.add_validation_error(e);
                Err(errors)
            }
        }
    }

    /// Begin a new transaction.
    ///
    /// The transaction accumulates operations (upsert, delete) and validates
    /// them atomically when committed.
    pub fn begin(&self) -> Transaction {
        let inner = self.inner.read().unwrap();
        Transaction::new(inner.version)
    }

    /// Commit a transaction, validating and applying changes atomically.
    ///
    /// If validation fails, no changes are applied and an error is returned.
    /// On success, changes are persisted to disk.
    pub fn commit(&self, txn: Transaction) -> Result<(), TransactionError> {
        let mut inner = self.inner.write().unwrap();

        // Check for concurrent modification
        if inner.version != txn.base_version() {
            return Err(TransactionError::Conflict {
                expected: txn.base_version(),
                actual: inner.version,
            });
        }

        // Build validation view and validate
        let view = ValidationView::new(&inner.graph, txn.ops());
        validations::validate_view(&view)?;

        // Apply changes to the graph
        for op in txn.into_ops() {
            match op {
                Op::Upsert(task) => {
                    inner.graph.insert(task);
                }
                Op::Delete(id) => {
                    inner.graph.remove(&id);
                }
            }
        }

        inner.version += 1;

        // Save to disk
        self.save_inner(&mut inner.graph)?;

        Ok(())
    }

    /// Get read-only access to the current graph state.
    ///
    /// Returns a guard that holds a read lock. The graph cannot be
    /// modified while this guard exists.
    pub fn graph(&self) -> impl std::ops::Deref<Target = TaskGraph> + '_ {
        struct GraphGuard<'a>(std::sync::RwLockReadGuard<'a, ContextInner>);
        impl std::ops::Deref for GraphGuard<'_> {
            type Target = TaskGraph;
            fn deref(&self) -> &Self::Target {
                &self.0.graph
            }
        }
        GraphGuard(self.inner.read().unwrap())
    }

    /// Get the tasks directory path.
    pub fn tasks_dir(&self) -> &PathBuf {
        &self.tasks_dir
    }

    /// Save dirty tasks to disk.
    fn save_inner(&self, graph: &mut TaskGraph) -> Result<usize, std::io::Error> {
        let dirty_tasks: Vec<_> = graph.dirty_tasks().into_iter().cloned().collect();
        let count = dirty_tasks.len();

        for task in dirty_tasks {
            let path = self.tasks_dir.join(format!("{}.md", task.id));
            if task.is_deleted() {
                // Remove the file if it exists (ignore NotFound errors)
                if let Err(e) = std::fs::remove_file(&path) {
                    if e.kind() != std::io::ErrorKind::NotFound {
                        return Err(e);
                    }
                }
            } else {
                let content = task.to_markdown();
                std::fs::write(&path, content)?;
            }
        }

        graph.clear_dirty();
        Ok(count)
    }
}

/// Errors that can occur during transaction commit.
#[derive(Debug, thiserror::Error)]
pub enum TransactionError {
    #[error("concurrent modification: expected version {expected}, found {actual}")]
    Conflict { expected: u64, actual: u64 },

    #[error("validation failed: {0}")]
    Validation(#[from] ValidationError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_task(id: &str) -> Task {
        Task {
            id: id.to_string(),
            before: vec![],
            after: vec![],
            validations: vec![],
            title: Some(format!("{} title", id)),
            status: None,
            task_type: TaskType::Task,
            description: String::new(),
            deleted: false,
        }
    }

    #[test]
    fn test_begin_and_commit() {
        let temp_dir = TempDir::new().unwrap();
        let ctx = MontContext::new(temp_dir.path().to_path_buf());

        let mut txn = ctx.begin();
        txn.upsert(make_task("task1"));
        txn.upsert(make_task("task2"));

        ctx.commit(txn).unwrap();

        let graph = ctx.graph();
        assert_eq!(graph.len(), 2);
        assert!(graph.contains("task1"));
        assert!(graph.contains("task2"));
    }

    #[test]
    fn test_commit_validates() {
        let temp_dir = TempDir::new().unwrap();
        let ctx = MontContext::new(temp_dir.path().to_path_buf());

        // Create a task that references non-existent task
        let mut task = make_task("task1");
        task.after = vec!["nonexistent".to_string()];

        let mut txn = ctx.begin();
        txn.upsert(task);

        let result = ctx.commit(txn);
        assert!(matches!(result, Err(TransactionError::Validation(_))));

        // Graph should be unchanged
        let graph = ctx.graph();
        assert!(graph.is_empty());
    }

    #[test]
    fn test_conflict_detection() {
        let temp_dir = TempDir::new().unwrap();
        let ctx = MontContext::new(temp_dir.path().to_path_buf());

        // Start two transactions
        let txn1 = ctx.begin();
        let mut txn2 = ctx.begin();

        // Commit first transaction
        let mut txn1 = txn1;
        txn1.upsert(make_task("task1"));
        ctx.commit(txn1).unwrap();

        // Second transaction should fail due to version mismatch
        txn2.upsert(make_task("task2"));
        let result = ctx.commit(txn2);
        assert!(matches!(result, Err(TransactionError::Conflict { .. })));
    }

    #[test]
    fn test_delete_in_transaction() {
        let temp_dir = TempDir::new().unwrap();
        let ctx = MontContext::new(temp_dir.path().to_path_buf());

        // Create initial task
        let mut txn = ctx.begin();
        txn.upsert(make_task("task1"));
        ctx.commit(txn).unwrap();

        // Delete it
        let mut txn = ctx.begin();
        txn.delete("task1");
        ctx.commit(txn).unwrap();

        let graph = ctx.graph();
        assert!(graph.is_empty());
    }
}
