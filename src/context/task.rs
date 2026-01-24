use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum TaskType {
    Jot,
    #[default]
    Task,
    Gate,
}

/// Task status - only stored statuses. "Ready" is computed from the graph.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    InProgress,
    Stopped,
    Complete,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum GateStatus {
    #[default]
    Pending,
    Passed,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GateItem {
    pub id: String,
    pub status: GateStatus,
}

impl<'de> Deserialize<'de> for GateItem {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, MapAccess, Visitor};

        struct GateItemVisitor;

        impl<'de> Visitor<'de> for GateItemVisitor {
            type Value = GateItem;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a string or a map with {id: status}")
            }

            fn visit_str<E>(self, value: &str) -> Result<GateItem, E>
            where
                E: de::Error,
            {
                Ok(GateItem {
                    id: value.to_string(),
                    status: GateStatus::Pending,
                })
            }

            fn visit_map<M>(self, mut map: M) -> Result<GateItem, M::Error>
            where
                M: MapAccess<'de>,
            {
                let Some((id, status)) = map.next_entry::<String, GateStatus>()? else {
                    return Err(de::Error::custom("expected a single key-value pair"));
                };
                Ok(GateItem { id, status })
            }
        }

        deserializer.deserialize_any(GateItemVisitor)
    }
}

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("missing frontmatter delimiters")]
    MissingFrontmatter,
    #[error("invalid yaml: {0}")]
    InvalidYaml(#[from] serde_yaml::Error),
    #[error("task id cannot be empty")]
    EmptyId,
    #[error("task id '{0}' is reserved")]
    ReservedId(String),
    #[error("gate '{0}' must not have after dependencies")]
    GateWithAfter(String),
    #[error("gate '{0}' cannot be marked complete")]
    GateMarkedComplete(String),
    #[error("jot '{0}' cannot have gates")]
    JotWithGates(String),
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Task {
    pub id: String,
    /// New ID for renaming. Only used in multieditor, not persisted.
    #[serde(default)]
    pub new_id: Option<String>,
    /// This task must complete before these referenced tasks
    #[serde(default)]
    pub before: Vec<String>,
    /// This task can only start after these tasks complete
    #[serde(default)]
    pub after: Vec<String>,
    #[serde(default)]
    pub gates: Vec<GateItem>,
    pub title: Option<String>,
    /// Task status: None means pending (ready if no blockers), Some(status) for explicit state
    #[serde(default)]
    pub status: Option<Status>,
    #[serde(default, rename = "type")]
    pub task_type: TaskType,
    #[serde(skip)]
    pub description: String,
    /// Internal flag for soft-deletion. Not persisted to markdown.
    #[serde(skip)]
    pub deleted: bool,
}

impl Task {
    pub fn gate_ids(&self) -> impl Iterator<Item = &str> {
        self.gates.iter().map(|v| v.id.as_str())
    }

    /// Returns true if this task is a gate (validator)
    pub fn is_gate(&self) -> bool {
        self.task_type == TaskType::Gate
    }

    /// Returns true if this task is a jot
    pub fn is_jot(&self) -> bool {
        self.task_type == TaskType::Jot
    }

    /// Returns true if this task is marked complete
    pub fn is_complete(&self) -> bool {
        self.status == Some(Status::Complete)
    }

    /// Returns true if this task is marked in progress
    pub fn is_in_progress(&self) -> bool {
        self.status == Some(Status::InProgress)
    }

    /// Returns true if this task is marked stopped
    pub fn is_stopped(&self) -> bool {
        self.status == Some(Status::Stopped)
    }

    /// Returns true if this task is marked for deletion
    pub fn is_deleted(&self) -> bool {
        self.deleted
    }

    /// Serialize this task to markdown format.
    ///
    /// Produces the frontmatter YAML block followed by the description.
    pub fn to_markdown(&self) -> String {
        let mut content = String::new();
        content.push_str("---\n");
        if !self.id.is_empty() {
            content.push_str(&format!("id: {}\n", self.id));
        }

        if let Some(t) = &self.title {
            content.push_str(&format!("title: {}\n", t));
        }

        match self.task_type {
            TaskType::Task => {} // default, don't write
            TaskType::Jot => content.push_str("type: jot\n"),
            TaskType::Gate => content.push_str("type: gate\n"),
        }

        if let Some(status) = &self.status {
            let status_str = match status {
                Status::InProgress => "inprogress",
                Status::Stopped => "stopped",
                Status::Complete => "complete",
            };
            content.push_str(&format!("status: {}\n", status_str));
        }

        if !self.before.is_empty() {
            content.push_str("before:\n");
            for target in &self.before {
                content.push_str(&format!("  - {}\n", target));
            }
        }

        if !self.after.is_empty() {
            content.push_str("after:\n");
            for dep in &self.after {
                content.push_str(&format!("  - {}\n", dep));
            }
        }

        if !self.gates.is_empty() {
            content.push_str("gates:\n");
            for val in &self.gates {
                match val.status {
                    GateStatus::Pending => {
                        content.push_str(&format!("  - {}\n", val.id));
                    }
                    GateStatus::Passed => {
                        content.push_str(&format!("  - {}: passed\n", val.id));
                    }
                    GateStatus::Failed => {
                        content.push_str(&format!("  - {}: failed\n", val.id));
                    }
                    GateStatus::Skipped => {
                        content.push_str(&format!("  - {}: skipped\n", val.id));
                    }
                }
            }
        }

        content.push_str("---\n\n");

        if !self.description.is_empty() {
            content.push_str(&self.description);
            content.push('\n');
        }

        content
    }
}

/// Parses a task markdown file, extracting frontmatter and description.
///
/// # Examples
///
/// Parsing a valid task:
/// ```
/// use mont::{parse, ParseError, GateStatus};
///
/// let content = r#"---
/// id: test-task
/// before:
///   - parent1
/// after:
///   - pre1
/// gates:
///   - val1
/// title: Test Task
/// ---
///
/// This is the task description.
/// "#;
///
/// let task = parse(content).unwrap();
/// assert_eq!(task.id, "test-task");
/// assert_eq!(task.before, vec!["parent1".to_string()]);
/// assert_eq!(task.after, vec!["pre1"]);
/// assert_eq!(task.gates.len(), 1);
/// assert_eq!(task.gates[0].id, "val1");
/// assert_eq!(task.gates[0].status, GateStatus::Pending);
/// assert_eq!(task.title, Some("Test Task".to_string()));
/// assert!(!task.is_gate());
/// assert_eq!(task.description, "This is the task description.");
/// ```
///
/// Parsing a gate (no after dependencies allowed):
/// ```
/// use mont::{parse, ParseError};
///
/// let content = r#"---
/// id: test-gate
/// type: gate
/// before:
///   - parent1
/// ---
///
/// Gate description.
/// "#;
///
/// let task = parse(content).unwrap();
/// assert!(task.is_gate());
/// assert_eq!(task.before, vec!["parent1".to_string()]);
/// ```
///
/// Missing frontmatter returns an error:
/// ```
/// use mont::{parse, ParseError};
///
/// let result = parse("No frontmatter here");
/// assert!(matches!(result, Err(ParseError::MissingFrontmatter)));
/// ```
///
/// Missing required `id` field returns an error:
/// ```
/// use mont::{parse, ParseError};
///
/// let content = r#"---
/// title: Task without id
/// ---
///
/// Some description.
/// "#;
///
/// let result = parse(content);
/// assert!(matches!(result, Err(ParseError::InvalidYaml(_))));
/// ```
///
/// Gate with after dependencies returns an error:
/// ```
/// use mont::{parse, ParseError};
///
/// let content = r#"---
/// id: bad-gate
/// type: gate
/// after:
///   - some-task
/// ---
///
/// This should fail.
/// "#;
///
/// let result = parse(content);
/// assert!(matches!(result, Err(ParseError::GateWithAfter(_))));
/// ```
pub fn parse(content: &str) -> Result<Task, ParseError> {
    let Some(start) = content.find("---") else {
        return Err(ParseError::MissingFrontmatter);
    };
    let after_first = start + 3;
    let Some(end) = content[after_first..].find("---") else {
        return Err(ParseError::MissingFrontmatter);
    };
    let yaml = &content[after_first..after_first + end];
    let description = content[after_first + end + 3..].trim().to_string();

    let mut task: Task = serde_yaml::from_str(yaml)?;
    task.description = description;

    // Validate reserved IDs
    if task.id == "?" {
        return Err(ParseError::ReservedId(task.id));
    }

    if task.is_gate() && !task.after.is_empty() {
        return Err(ParseError::GateWithAfter(task.id));
    }

    if task.is_gate() && task.is_complete() {
        return Err(ParseError::GateMarkedComplete(task.id));
    }

    if task.is_jot() && !task.gates.is_empty() {
        return Err(ParseError::JotWithGates(task.id));
    }

    Ok(task)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_task() {
        let content = r#"---
id: test-task
before:
  - task1
after:
  - dep1
gates:
  - val1
title: Test Task
---

Task description here.
"#;
        let task = parse(content).unwrap();
        assert_eq!(task.id, "test-task");
        assert_eq!(task.before, vec!["task1".to_string()]);
        assert_eq!(task.after, vec!["dep1"]);
        assert_eq!(task.gates.len(), 1);
        assert_eq!(task.gates[0].id, "val1");
        assert_eq!(task.gates[0].status, GateStatus::Pending);
        assert_eq!(task.title, Some("Test Task".to_string()));
        assert!(!task.is_gate());
        assert_eq!(task.description, "Task description here.");
    }

    #[test]
    fn test_parse_gate_without_after() {
        let content = r#"---
id: my-gate
type: gate
before:
  - task1
---

Gate description.
"#;
        let task = parse(content).unwrap();
        assert_eq!(task.id, "my-gate");
        assert!(task.is_gate());
        assert_eq!(task.before, vec!["task1".to_string()]);
        assert!(task.after.is_empty());
    }

    #[test]
    fn test_parse_gate_with_after_fails() {
        let content = r#"---
id: bad-gate
type: gate
after:
  - some-task
---

Should fail.
"#;
        let result = parse(content);
        assert!(matches!(
            result,
            Err(ParseError::GateWithAfter(id)) if id == "bad-gate"
        ));
    }

    #[test]
    fn test_parse_gate_marked_complete_fails() {
        let content = r#"---
id: complete-gate
type: gate
status: complete
---

Should fail.
"#;
        let result = parse(content);
        assert!(matches!(
            result,
            Err(ParseError::GateMarkedComplete(id)) if id == "complete-gate"
        ));
    }

    #[test]
    fn test_parse_jot_with_gates_fails() {
        let content = r#"---
id: bad-jot
type: jot
gates:
  - some-gate
---

Jots cannot have gates.
"#;
        let result = parse(content);
        assert!(matches!(
            result,
            Err(ParseError::JotWithGates(id)) if id == "bad-jot"
        ));
    }

    #[test]
    fn test_parse_missing_frontmatter() {
        let result = parse("No frontmatter here");
        assert!(matches!(result, Err(ParseError::MissingFrontmatter)));
    }

    #[test]
    fn test_parse_missing_closing_delimiter() {
        let content = "---\nid: test\nNo closing delimiter";
        let result = parse(content);
        assert!(matches!(result, Err(ParseError::MissingFrontmatter)));
    }

    #[test]
    fn test_parse_missing_id() {
        let content = r#"---
title: No id
---

Description.
"#;
        let result = parse(content);
        assert!(matches!(result, Err(ParseError::InvalidYaml(_))));
    }

    #[test]
    fn test_parse_empty_optional_fields() {
        let content = r#"---
id: minimal
---

Minimal task.
"#;
        let task = parse(content).unwrap();
        assert_eq!(task.id, "minimal");
        assert!(task.before.is_empty());
        assert!(task.after.is_empty());
        assert!(task.gates.is_empty());
        assert!(task.title.is_none());
        assert!(!task.is_gate());
        assert!(!task.is_complete());
    }

    #[test]
    fn test_parse_status_complete() {
        let content = r#"---
id: done-task
status: complete
---

A completed task.
"#;
        let task = parse(content).unwrap();
        assert_eq!(task.id, "done-task");
        assert!(task.is_complete());
    }

    #[test]
    fn test_parse_no_status() {
        let content = r#"---
id: pending-task
---

A pending task.
"#;
        let task = parse(content).unwrap();
        assert_eq!(task.id, "pending-task");
        assert!(!task.is_complete());
        assert!(!task.is_in_progress());
        assert!(!task.is_stopped());
    }

    #[test]
    fn test_parse_status_in_progress() {
        let content = r#"---
id: in-progress-task
status: inprogress
---

A task in progress.
"#;
        let task = parse(content).unwrap();
        assert_eq!(task.id, "in-progress-task");
        assert!(task.is_in_progress());
    }

    #[test]
    fn test_parse_status_stopped() {
        let content = r#"---
id: stopped-task
status: stopped
---

A stopped task.
"#;
        let task = parse(content).unwrap();
        assert_eq!(task.id, "stopped-task");
        assert!(task.is_stopped());
    }

    #[test]
    fn test_parse_type_task() {
        let content = r#"---
id: implement-feature
title: Implement the login feature
type: task
---

Implement login functionality.
"#;
        let task = parse(content).unwrap();
        assert_eq!(task.id, "implement-feature");
        assert_eq!(task.task_type, TaskType::Task);
        assert_eq!(
            task.title,
            Some("Implement the login feature".to_string())
        );
    }

    #[test]
    fn test_parse_type_gate() {
        let content = r#"---
id: run-tests
type: gate
---

A gate task.
"#;
        let task = parse(content).unwrap();
        assert_eq!(task.id, "run-tests");
        assert_eq!(task.task_type, TaskType::Gate);
    }

    #[test]
    fn test_parse_type_defaults_to_task() {
        let content = r#"---
id: regular-task
---

No type field specified.
"#;
        let task = parse(content).unwrap();
        assert_eq!(task.id, "regular-task");
        assert_eq!(task.task_type, TaskType::Task);
    }

    #[test]
    fn test_parse_type_jot() {
        let content = r#"---
id: quick-idea
title: Quick idea for later
type: jot
---

Just a quick idea to capture.
"#;
        let task = parse(content).unwrap();
        assert_eq!(task.id, "quick-idea");
        assert_eq!(task.task_type, TaskType::Jot);
        assert_eq!(task.title, Some("Quick idea for later".to_string()));
    }

    #[test]
    fn test_parse_validation_with_status_passed() {
        let content = r#"---
id: test-task
gates:
  - val1: passed
---

Task with passed validation.
"#;
        let task = parse(content).unwrap();
        assert_eq!(task.gates.len(), 1);
        assert_eq!(task.gates[0].id, "val1");
        assert_eq!(task.gates[0].status, GateStatus::Passed);
    }

    #[test]
    fn test_parse_validation_with_status_failed() {
        let content = r#"---
id: test-task
gates:
  - val1: failed
---

Task with failed validation.
"#;
        let task = parse(content).unwrap();
        assert_eq!(task.gates.len(), 1);
        assert_eq!(task.gates[0].id, "val1");
        assert_eq!(task.gates[0].status, GateStatus::Failed);
    }

    #[test]
    fn test_parse_validation_with_status_skipped() {
        let content = r#"---
id: test-task
gates:
  - val1: skipped
---

Task with skipped validation.
"#;
        let task = parse(content).unwrap();
        assert_eq!(task.gates.len(), 1);
        assert_eq!(task.gates[0].id, "val1");
        assert_eq!(task.gates[0].status, GateStatus::Skipped);
    }

    #[test]
    fn test_parse_mixed_gates() {
        let content = r#"---
id: test-task
gates:
  - val1
  - val2: passed
  - val3: failed
  - val4: skipped
---

Task with mixed validation statuses.
"#;
        let task = parse(content).unwrap();
        assert_eq!(task.gates.len(), 4);

        assert_eq!(task.gates[0].id, "val1");
        assert_eq!(task.gates[0].status, GateStatus::Pending);

        assert_eq!(task.gates[1].id, "val2");
        assert_eq!(task.gates[1].status, GateStatus::Passed);

        assert_eq!(task.gates[2].id, "val3");
        assert_eq!(task.gates[2].status, GateStatus::Failed);

        assert_eq!(task.gates[3].id, "val4");
        assert_eq!(task.gates[3].status, GateStatus::Skipped);
    }

    #[test]
    fn test_gate_ids_helper() {
        let content = r#"---
id: test-task
gates:
  - val1
  - val2: passed
  - val3: failed
---

Task description.
"#;
        let task = parse(content).unwrap();
        let ids: Vec<&str> = task.gate_ids().collect();
        assert_eq!(ids, vec!["val1", "val2", "val3"]);
    }

    #[test]
    fn test_to_markdown_minimal() {
        let task = Task {
            id: "minimal".to_string(),
            new_id: None,
            before: vec![],
            after: vec![],
            gates: vec![],
            title: None,
            status: None,
            task_type: TaskType::Task,
            description: String::new(),
            deleted: false,
        };
        let markdown = task.to_markdown();
        let parsed = parse(&markdown).unwrap();
        assert_eq!(parsed.id, "minimal");
        assert!(parsed.title.is_none());
        assert_eq!(parsed.task_type, TaskType::Task);
    }

    #[test]
    fn test_to_markdown_roundtrip_full() {
        // Use Task type since jots cannot have gates
        let task = Task {
            id: "full-task".to_string(),
            new_id: None,
            before: vec!["parent1".to_string(), "parent2".to_string()],
            after: vec!["dep1".to_string()],
            gates: vec![
                GateItem {
                    id: "val1".to_string(),
                    status: GateStatus::Pending,
                },
                GateItem {
                    id: "val2".to_string(),
                    status: GateStatus::Passed,
                },
                GateItem {
                    id: "val3".to_string(),
                    status: GateStatus::Failed,
                },
            ],
            title: Some("Full Task Title".to_string()),
            status: Some(Status::InProgress),
            task_type: TaskType::Task,
            description: "This is the description.".to_string(),
            deleted: false,
        };
        let markdown = task.to_markdown();
        let parsed = parse(&markdown).unwrap();

        assert_eq!(parsed.id, task.id);
        assert_eq!(parsed.title, task.title);
        assert_eq!(parsed.task_type, TaskType::Task);
        assert_eq!(parsed.status, Some(Status::InProgress));
        assert_eq!(parsed.before, task.before);
        assert_eq!(parsed.after, task.after);
        assert_eq!(parsed.gates.len(), 3);
        assert_eq!(parsed.gates[0].status, GateStatus::Pending);
        assert_eq!(parsed.gates[1].status, GateStatus::Passed);
        assert_eq!(parsed.gates[2].status, GateStatus::Failed);
        assert_eq!(parsed.description, task.description);
    }

    #[test]
    fn test_to_markdown_gate() {
        let task = Task {
            id: "my-gate".to_string(),
            new_id: None,
            before: vec!["consumer".to_string()],
            after: vec![],
            gates: vec![],
            title: Some("Gate Title".to_string()),
            status: None,
            task_type: TaskType::Gate,
            description: "Gate description.".to_string(),
            deleted: false,
        };
        let markdown = task.to_markdown();
        let parsed = parse(&markdown).unwrap();
        assert_eq!(parsed.task_type, TaskType::Gate);
        assert!(parsed.is_gate());
    }

    #[test]
    fn test_to_markdown_complete_status() {
        let task = Task {
            id: "done-task".to_string(),
            new_id: None,
            before: vec![],
            after: vec![],
            gates: vec![],
            title: Some("Completed Task".to_string()),
            status: Some(Status::Complete),
            task_type: TaskType::Task,
            description: String::new(),
            deleted: false,
        };
        let markdown = task.to_markdown();
        let parsed = parse(&markdown).unwrap();
        assert!(parsed.is_complete());
    }

    #[test]
    fn test_to_markdown_stopped_status() {
        let task = Task {
            id: "stopped-task".to_string(),
            new_id: None,
            before: vec![],
            after: vec![],
            gates: vec![],
            title: None,
            status: Some(Status::Stopped),
            task_type: TaskType::Task,
            description: String::new(),
            deleted: false,
        };
        let markdown = task.to_markdown();
        let parsed = parse(&markdown).unwrap();
        assert!(parsed.is_stopped());
    }
}
