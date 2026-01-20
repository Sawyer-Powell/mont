//! Global settings configuration for the task system.
//!
//! The settings file (`config.yml`) lives in the `.tasks` directory and
//! configures global behavior like default gates that must pass.

use std::path::Path;

use serde::Deserialize;

use super::TaskGraph;

/// Configuration for jj (Jujutsu) VCS integration.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct JjConfig {
    /// Whether jj operations are enabled. When false, all jj functions
    /// return no-op/happy-path results. Default: true.
    pub enabled: bool,
}

impl Default for JjConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

/// Global configuration loaded from `.tasks/config.yml`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GlobalConfig {
    /// Gate IDs that must pass for all tasks.
    #[serde(default)]
    pub default_gates: Vec<String>,

    /// Configuration for jj VCS integration.
    #[serde(default)]
    pub jj: JjConfig,
}

/// Errors that can occur when loading or validating settings.
#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    #[error("failed to read settings file: {0}")]
    Io(#[from] std::io::Error),

    #[error("failed to parse settings file: {0}")]
    Parse(#[from] serde_yaml::Error),

    #[error("default gate '{gate_id}' not found in task graph")]
    GateNotFound { gate_id: String },

    #[error("default gate '{gate_id}' is not a gate (type: {actual_type})")]
    NotAGate { gate_id: String, actual_type: String },
}

impl GlobalConfig {
    /// Load settings from a file path.
    ///
    /// Returns the default config if the file doesn't exist.
    pub fn load(path: &Path) -> Result<Self, SettingsError> {
        match std::fs::read_to_string(path) {
            Ok(content) => {
                let config: GlobalConfig = serde_yaml::from_str(&content)?;
                Ok(config)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(SettingsError::Io(e)),
        }
    }

    /// Validate the config against a task graph.
    ///
    /// Ensures all default gates exist and are actually gates.
    pub fn validate(&self, graph: &TaskGraph) -> Result<(), SettingsError> {
        for gate_id in &self.default_gates {
            match graph.get(gate_id) {
                Some(task) => {
                    if !task.is_gate() {
                        return Err(SettingsError::NotAGate {
                            gate_id: gate_id.clone(),
                            actual_type: format!("{:?}", task.task_type),
                        });
                    }
                }
                None => {
                    return Err(SettingsError::GateNotFound {
                        gate_id: gate_id.clone(),
                    });
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::task::{Task, TaskType};
    use tempfile::TempDir;

    fn make_gate(id: &str) -> Task {
        Task {
            id: id.to_string(),
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

    fn make_task(id: &str) -> Task {
        Task {
            id: id.to_string(),
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

    #[test]
    fn test_load_missing_file_returns_default() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("nonexistent.yml");

        let config = GlobalConfig::load(&path).unwrap();
        assert!(config.default_gates.is_empty());
    }

    #[test]
    fn test_load_valid_config() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("config.yml");

        std::fs::write(&path, "default_gates:\n  - test-gate\n  - lint-gate\n").unwrap();

        let config = GlobalConfig::load(&path).unwrap();
        assert_eq!(config.default_gates, vec!["test-gate", "lint-gate"]);
    }

    #[test]
    fn test_load_empty_config() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("config.yml");

        std::fs::write(&path, "").unwrap();

        let config = GlobalConfig::load(&path).unwrap();
        assert!(config.default_gates.is_empty());
    }

    #[test]
    fn test_validate_valid_gates() {
        let mut graph = TaskGraph::new();
        graph.insert(make_gate("test-gate"));
        graph.insert(make_gate("lint-gate"));

        let config = GlobalConfig {
            default_gates: vec!["test-gate".to_string(), "lint-gate".to_string()],
            ..Default::default()
        };

        assert!(config.validate(&graph).is_ok());
    }

    #[test]
    fn test_validate_missing_gate() {
        let graph = TaskGraph::new();

        let config = GlobalConfig {
            default_gates: vec!["nonexistent".to_string()],
            ..Default::default()
        };

        let err = config.validate(&graph).unwrap_err();
        assert!(matches!(err, SettingsError::GateNotFound { gate_id } if gate_id == "nonexistent"));
    }

    #[test]
    fn test_validate_not_a_gate() {
        let mut graph = TaskGraph::new();
        graph.insert(make_task("regular-task"));

        let config = GlobalConfig {
            default_gates: vec!["regular-task".to_string()],
            ..Default::default()
        };

        let err = config.validate(&graph).unwrap_err();
        assert!(matches!(err, SettingsError::NotAGate { gate_id, .. } if gate_id == "regular-task"));
    }

    #[test]
    fn test_validate_empty_config() {
        let graph = TaskGraph::new();
        let config = GlobalConfig::default();

        assert!(config.validate(&graph).is_ok());
    }

    #[test]
    fn test_load_rejects_unknown_fields() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("config.yml");

        std::fs::write(&path, "unknown_field: value\n").unwrap();

        let err = GlobalConfig::load(&path).unwrap_err();
        assert!(matches!(err, SettingsError::Parse(_)));
    }
}
