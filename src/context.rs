use std::path::PathBuf;

use crate::graph::TaskGraph;

/// Central application context holding the task graph and configuration.
///
/// MontContext is the main entry point for all task operations. It manages:
/// - The task graph (tasks and their dependencies)
/// - The path to the tasks directory
///
/// Future tasks will add load/save methods and mutation operations.
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
}
