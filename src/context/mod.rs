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
    #[allow(clippy::expect_used)] // RwLock poisoning is a bug
    pub fn begin(&self) -> Transaction {
        let inner = self.inner.read().expect("lock poisoned");
        Transaction::new(inner.version)
    }

    /// Commit a transaction, validating and applying changes atomically.
    ///
    /// If validation fails, no changes are applied and an error is returned.
    /// On success, changes are persisted to disk.
    #[allow(clippy::expect_used)] // RwLock poisoning is a bug
    pub fn commit(&self, txn: Transaction) -> Result<(), TransactionError> {
        let mut inner = self.inner.write().expect("lock poisoned");

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
                Op::Insert(task) => {
                    inner.graph.insert(task);
                }
                Op::Update { old_id, task } => {
                    if old_id != task.id {
                        inner.graph.remove(&old_id);
                    }
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
    #[allow(clippy::expect_used)] // RwLock poisoning is a bug
    pub fn graph(&self) -> impl std::ops::Deref<Target = TaskGraph> + '_ {
        struct GraphGuard<'a>(std::sync::RwLockReadGuard<'a, ContextInner>);
        impl std::ops::Deref for GraphGuard<'_> {
            type Target = TaskGraph;
            fn deref(&self) -> &Self::Target {
                &self.0.graph
            }
        }
        GraphGuard(self.inner.read().expect("lock poisoned"))
    }

    /// Get the tasks directory path.
    pub fn tasks_dir(&self) -> &PathBuf {
        &self.tasks_dir
    }

    /// Delete a task and remove all references to it from other tasks.
    ///
    /// Returns an error if the task doesn't exist.
    pub fn delete(&self, id: &str) -> Result<(), TransactionError> {
        let graph = self.graph();

        if !graph.contains(id) {
            return Err(TransactionError::TaskNotFound(id.to_string()));
        }

        let mut txn = self.begin();
        txn.rewrite_references(&*graph, id, None);
        txn.delete(id);
        drop(graph);
        self.commit(txn)
    }

    /// Insert a new task.
    ///
    /// If the task has an empty ID, generates a unique one using petname.
    /// Returns an error if a task with this ID already exists.
    ///
    /// Returns the final task ID (useful when ID was generated).
    pub fn insert(&self, mut task: Task) -> Result<String, TransactionError> {
        // Generate ID if empty, or check for duplicates
        if task.id.is_empty() {
            task.id = self.generate_id(&self.graph())?;
        } else if self.graph().contains(&task.id) {
            return Err(TransactionError::TaskAlreadyExists(task.id));
        }

        let id = task.id.clone();
        let mut txn = self.begin();
        txn.insert(task);
        self.commit(txn)?;
        Ok(id)
    }

    /// Update an existing task, optionally changing its ID.
    ///
    /// The `old_id` identifies the task to update. The `task` contains the new
    /// state (including potentially a new ID in `task.id`).
    ///
    /// If the ID changes, all references to the old ID in other tasks are
    /// automatically updated (before, after, and validations fields).
    pub fn update(&self, old_id: &str, task: Task) -> Result<(), TransactionError> {
        let graph = self.graph();

        if !graph.contains(old_id) {
            return Err(TransactionError::TaskNotFound(old_id.to_string()));
        }

        let new_id = &task.id;
        let id_changed = old_id != new_id;

        if id_changed && graph.contains(new_id) {
            return Err(TransactionError::TaskAlreadyExists(new_id.clone()));
        }

        let mut txn = self.begin();
        if id_changed {
            txn.rewrite_references(&*graph, old_id, Some(new_id));
        }
        txn.update(old_id, task);
        drop(graph);
        self.commit(txn)
    }

    /// Generate a unique task ID using petname.
    fn generate_id(&self, graph: &TaskGraph) -> Result<String, TransactionError> {
        const MAX_ATTEMPTS: u32 = 100;

        for _ in 0..MAX_ATTEMPTS {
            if let Some(candidate) = petname::petname(2, "-")
                && !graph.contains(&candidate)
            {
                return Ok(candidate);
            }
        }

        Err(TransactionError::IdGenerationFailed(MAX_ATTEMPTS))
    }

    /// Save dirty tasks to disk.
    fn save_inner(&self, graph: &mut TaskGraph) -> Result<usize, std::io::Error> {
        let dirty_tasks: Vec<_> = graph.dirty_tasks().into_iter().cloned().collect();
        let count = dirty_tasks.len();

        for task in dirty_tasks {
            let path = self.tasks_dir.join(format!("{}.md", task.id));
            if task.is_deleted() {
                // Remove the file if it exists (ignore NotFound errors)
                if let Err(e) = std::fs::remove_file(&path)
                    && e.kind() != std::io::ErrorKind::NotFound
                {
                    return Err(e);
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

/// Errors that can occur during MontContext operations.
#[derive(Debug, thiserror::Error)]
pub enum TransactionError {
    #[error("concurrent modification: expected version {expected}, found {actual}")]
    Conflict { expected: u64, actual: u64 },

    #[error("validation failed: {0}")]
    Validation(#[from] ValidationError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("task not found: {0}")]
    TaskNotFound(String),

    #[error("task already exists: {0}")]
    TaskAlreadyExists(String),

    #[error("failed to generate unique ID after {0} attempts")]
    IdGenerationFailed(u32),
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
        txn.insert(make_task("task1"));
        txn.insert(make_task("task2"));

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
        txn.insert(task);

        let result = ctx.commit(txn);
        assert!(matches!(
            result,
            Err(TransactionError::Validation(ValidationError::InvalidAfter {
                ref task_id,
                ref after_id,
            })) if task_id == "task1" && after_id == "nonexistent"
        ));

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
        txn1.insert(make_task("task1"));
        ctx.commit(txn1).unwrap();

        // Second transaction should fail due to version mismatch
        txn2.insert(make_task("task2"));
        let result = ctx.commit(txn2);
        assert!(matches!(result, Err(TransactionError::Conflict { .. })));
    }

    #[test]
    fn test_delete_in_transaction() {
        let temp_dir = TempDir::new().unwrap();
        let ctx = MontContext::new(temp_dir.path().to_path_buf());

        // Create initial task
        let mut txn = ctx.begin();
        txn.insert(make_task("task1"));
        ctx.commit(txn).unwrap();

        // Delete it
        let mut txn = ctx.begin();
        txn.delete("task1");
        ctx.commit(txn).unwrap();

        let graph = ctx.graph();
        assert!(graph.is_empty());
    }

    #[test]
    fn test_delete_removes_references() {
        let temp_dir = TempDir::new().unwrap();
        let ctx = MontContext::new(temp_dir.path().to_path_buf());

        // Create tasks with references
        let task1 = make_task("task1");
        let mut task2 = make_task("task2");
        task2.before = vec!["task1".to_string()];
        task2.after = vec!["task3".to_string()];

        let task3 = make_task("task3");

        let mut txn = ctx.begin();
        txn.insert(task1);
        txn.insert(task2);
        txn.insert(task3);
        ctx.commit(txn).unwrap();

        // Delete task1 using the direct delete method
        ctx.delete("task1").unwrap();

        let graph = ctx.graph();
        assert_eq!(graph.len(), 2);
        assert!(!graph.contains("task1"));

        // task2's before reference should be removed
        let task2 = graph.get("task2").unwrap();
        assert!(task2.before.is_empty());
        assert_eq!(task2.after, vec!["task3".to_string()]);
    }

    #[test]
    fn test_delete_nonexistent_task() {
        let temp_dir = TempDir::new().unwrap();
        let ctx = MontContext::new(temp_dir.path().to_path_buf());

        let result = ctx.delete("nonexistent");
        assert!(matches!(
            result,
            Err(TransactionError::TaskNotFound(ref id)) if id == "nonexistent"
        ));
    }

    #[test]
    fn test_insert_new_task() {
        let temp_dir = TempDir::new().unwrap();
        let ctx = MontContext::new(temp_dir.path().to_path_buf());

        let task = make_task("new-task");
        let id = ctx.insert(task).unwrap();

        assert_eq!(id, "new-task");
        let graph = ctx.graph();
        assert!(graph.contains("new-task"));
    }

    #[test]
    fn test_insert_generates_id() {
        let temp_dir = TempDir::new().unwrap();
        let ctx = MontContext::new(temp_dir.path().to_path_buf());

        let mut task = make_task("");
        task.id = String::new();
        let id = ctx.insert(task).unwrap();

        // ID should be generated (petname format: word-word)
        assert!(!id.is_empty());
        assert!(id.contains('-'));

        let graph = ctx.graph();
        assert!(graph.contains(&id));
    }

    #[test]
    fn test_insert_duplicate_fails() {
        let temp_dir = TempDir::new().unwrap();
        let ctx = MontContext::new(temp_dir.path().to_path_buf());

        let task1 = make_task("task1");
        ctx.insert(task1).unwrap();

        let task1_again = make_task("task1");
        let result = ctx.insert(task1_again);
        assert!(matches!(
            result,
            Err(TransactionError::TaskAlreadyExists(ref id)) if id == "task1"
        ));
    }

    #[test]
    fn test_update_task() {
        let temp_dir = TempDir::new().unwrap();
        let ctx = MontContext::new(temp_dir.path().to_path_buf());

        let mut task1 = make_task("task1");
        task1.title = Some("Original".to_string());
        ctx.insert(task1).unwrap();

        let mut task1_updated = make_task("task1");
        task1_updated.title = Some("Updated".to_string());
        ctx.update("task1", task1_updated).unwrap();

        let graph = ctx.graph();
        let task = graph.get("task1").unwrap();
        assert_eq!(task.title, Some("Updated".to_string()));
    }

    #[test]
    fn test_update_with_id_change() {
        let temp_dir = TempDir::new().unwrap();
        let ctx = MontContext::new(temp_dir.path().to_path_buf());

        let task = make_task("old-id");
        ctx.insert(task).unwrap();

        let mut renamed = make_task("new-id");
        renamed.title = Some("old-id title".to_string());
        ctx.update("old-id", renamed).unwrap();

        let graph = ctx.graph();
        assert!(!graph.contains("old-id"));
        assert!(graph.contains("new-id"));
    }

    #[test]
    fn test_update_updates_references() {
        let temp_dir = TempDir::new().unwrap();
        let ctx = MontContext::new(temp_dir.path().to_path_buf());

        // Create tasks with references
        let task1 = make_task("task1");
        let mut task2 = make_task("task2");
        task2.before = vec!["task1".to_string()];
        task2.after = vec!["task3".to_string()];

        let mut task3 = make_task("task3");
        task3.after = vec!["task1".to_string()];

        let mut txn = ctx.begin();
        txn.insert(task1);
        txn.insert(task2);
        txn.insert(task3);
        ctx.commit(txn).unwrap();

        // Update task1 with new ID
        let mut renamed = make_task("renamed-task");
        renamed.title = Some("task1 title".to_string());
        ctx.update("task1", renamed).unwrap();

        let graph = ctx.graph();

        // task2 should have updated before reference
        let task2 = graph.get("task2").unwrap();
        assert_eq!(task2.before, vec!["renamed-task".to_string()]);

        // task3 should have updated after reference
        let task3 = graph.get("task3").unwrap();
        assert_eq!(task3.after, vec!["renamed-task".to_string()]);
    }

    #[test]
    fn test_update_nonexistent_task() {
        let temp_dir = TempDir::new().unwrap();
        let ctx = MontContext::new(temp_dir.path().to_path_buf());

        let task = make_task("new-id");
        let result = ctx.update("nonexistent", task);
        assert!(matches!(
            result,
            Err(TransactionError::TaskNotFound(ref id)) if id == "nonexistent"
        ));
    }

    #[test]
    fn test_update_to_existing_id() {
        let temp_dir = TempDir::new().unwrap();
        let ctx = MontContext::new(temp_dir.path().to_path_buf());

        let task1 = make_task("task1");
        let task2 = make_task("task2");
        ctx.insert(task1).unwrap();
        ctx.insert(task2).unwrap();

        // Try to rename task1 to task2 (should fail)
        let renamed = make_task("task2");
        let result = ctx.update("task1", renamed);
        assert!(matches!(
            result,
            Err(TransactionError::TaskAlreadyExists(ref id)) if id == "task2"
        ));
    }
}
