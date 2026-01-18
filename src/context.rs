use std::path::PathBuf;

use crate::graph::{form_graph, GraphReadError, TaskGraph};
use crate::task;

/// Central application context holding the task graph and configuration.
///
/// MontContext is the main entry point for all task operations. It manages:
/// - The task graph (tasks and their dependencies)
/// - The path to the tasks directory
#[derive(Debug)]
pub struct MontContext {
    /// The task graph containing all tasks and their relationships.
    pub graph: TaskGraph,
    /// Path to the .tasks directory.
    pub tasks_dir: PathBuf,
}

impl MontContext {
    /// Create a new MontContext with an empty graph.
    pub fn new(tasks_dir: PathBuf) -> Self {
        Self {
            graph: TaskGraph::new(),
            tasks_dir,
        }
    }

    /// Create a MontContext with an existing graph.
    pub fn with_graph(tasks_dir: PathBuf, graph: TaskGraph) -> Self {
        Self { graph, tasks_dir }
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

            match task::parse(&content) {
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

        // Validate the graph
        match form_graph(tasks) {
            Ok(graph) => Ok(Self { graph, tasks_dir }),
            Err(e) => {
                errors.add_validation_error(e);
                Err(errors)
            }
        }
    }

    /// Save dirty tasks to the tasks directory.
    ///
    /// Only tasks marked as dirty are processed:
    /// - Deleted tasks have their files removed from disk
    /// - Modified tasks are written to disk
    ///
    /// After a successful save, the dirty set is cleared and deleted tasks
    /// are purged from memory.
    ///
    /// Returns the number of tasks processed, or an error if any operation fails.
    pub fn save(&mut self) -> Result<usize, std::io::Error> {
        let dirty_tasks: Vec<_> = self.graph.dirty_tasks().into_iter().cloned().collect();
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

        self.graph.clear_dirty();
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::{Status, Task, TaskType};
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
    fn test_save_writes_dirty_tasks() {
        let temp_dir = TempDir::new().unwrap();
        let tasks_dir = temp_dir.path().to_path_buf();

        let mut ctx = MontContext::new(tasks_dir.clone());
        ctx.graph.insert(make_task("task1"));
        ctx.graph.insert(make_task("task2"));

        let count = ctx.save().unwrap();
        assert_eq!(count, 2);
        assert!(!ctx.graph.has_dirty());

        // Verify files were written
        assert!(tasks_dir.join("task1.md").exists());
        assert!(tasks_dir.join("task2.md").exists());
    }

    #[test]
    fn test_save_only_dirty_tasks() {
        let temp_dir = TempDir::new().unwrap();
        let tasks_dir = temp_dir.path().to_path_buf();

        // Create initial task
        let mut ctx = MontContext::new(tasks_dir.clone());
        ctx.graph.insert(make_task("task1"));
        ctx.save().unwrap();

        // Modify task1, add task2
        let task1 = ctx.graph.get_mut("task1").unwrap();
        task1.status = Some(Status::InProgress);
        ctx.graph.insert(make_task("task2"));

        // Both should be dirty
        assert!(ctx.graph.is_dirty("task1"));
        assert!(ctx.graph.is_dirty("task2"));

        let count = ctx.save().unwrap();
        assert_eq!(count, 2);
        assert!(!ctx.graph.has_dirty());
    }

    #[test]
    fn test_save_empty_graph() {
        let temp_dir = TempDir::new().unwrap();
        let tasks_dir = temp_dir.path().to_path_buf();

        let mut ctx = MontContext::new(tasks_dir);
        let count = ctx.save().unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_save_no_dirty_tasks() {
        let temp_dir = TempDir::new().unwrap();
        let tasks_dir = temp_dir.path().to_path_buf();

        let mut ctx = MontContext::new(tasks_dir.clone());
        ctx.graph.insert(make_task("task1"));
        ctx.save().unwrap();

        // No modifications, save again
        let count = ctx.save().unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_load_and_save_roundtrip() {
        let temp_dir = TempDir::new().unwrap();
        let tasks_dir = temp_dir.path().to_path_buf();

        // Create and save some tasks
        let mut ctx = MontContext::new(tasks_dir.clone());
        let mut task1 = make_task("task1");
        task1.status = Some(Status::Complete);
        ctx.graph.insert(task1);
        ctx.graph.insert(make_task("task2"));
        ctx.save().unwrap();

        // Load the context back
        let loaded = MontContext::load(tasks_dir).unwrap();
        assert_eq!(loaded.graph.len(), 2);
        assert!(loaded.graph.get("task1").unwrap().is_complete());
        assert!(!loaded.graph.get("task2").unwrap().is_complete());
    }

    #[test]
    fn test_save_deletes_removed_tasks() {
        let temp_dir = TempDir::new().unwrap();
        let tasks_dir = temp_dir.path().to_path_buf();

        // Create and save initial tasks
        let mut ctx = MontContext::new(tasks_dir.clone());
        ctx.graph.insert(make_task("task1"));
        ctx.graph.insert(make_task("task2"));
        ctx.save().unwrap();

        // Verify both files exist
        assert!(tasks_dir.join("task1.md").exists());
        assert!(tasks_dir.join("task2.md").exists());

        // Mark task1 as deleted
        ctx.graph.remove("task1");
        assert!(ctx.graph.get("task1").unwrap().is_deleted());

        // Save should delete task1 file
        let count = ctx.save().unwrap();
        assert_eq!(count, 1);

        // task1.md should be deleted, task2.md should remain
        assert!(!tasks_dir.join("task1.md").exists());
        assert!(tasks_dir.join("task2.md").exists());

        // task1 should be purged from memory after save
        assert!(ctx.graph.get("task1").is_none());
        assert_eq!(ctx.graph.len(), 1);
    }

    #[test]
    fn test_delete_nonexistent_task_is_noop() {
        let temp_dir = TempDir::new().unwrap();
        let tasks_dir = temp_dir.path().to_path_buf();

        let mut ctx = MontContext::new(tasks_dir);
        ctx.graph.insert(make_task("task1"));

        // Trying to delete nonexistent task returns false
        let removed = ctx.graph.remove("nonexistent");
        assert!(!removed);
    }
}
