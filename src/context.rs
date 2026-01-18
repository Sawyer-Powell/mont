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
}
