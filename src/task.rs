use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum TaskType {
    Bug,
    Epic,
    #[default]
    Feature,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ValidationStatus {
    #[default]
    Pending,
    Passed,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValidationItem {
    pub id: String,
    pub status: ValidationStatus,
}

impl<'de> Deserialize<'de> for ValidationItem {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, MapAccess, Visitor};

        struct ValidationItemVisitor;

        impl<'de> Visitor<'de> for ValidationItemVisitor {
            type Value = ValidationItem;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a string or a map with {id: status}")
            }

            fn visit_str<E>(self, value: &str) -> Result<ValidationItem, E>
            where
                E: de::Error,
            {
                Ok(ValidationItem {
                    id: value.to_string(),
                    status: ValidationStatus::Pending,
                })
            }

            fn visit_map<M>(self, mut map: M) -> Result<ValidationItem, M::Error>
            where
                M: MapAccess<'de>,
            {
                let Some((id, status)) = map.next_entry::<String, ValidationStatus>()? else {
                    return Err(de::Error::custom("expected a single key-value pair"));
                };
                Ok(ValidationItem { id, status })
            }
        }

        deserializer.deserialize_any(ValidationItemVisitor)
    }
}

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("missing frontmatter delimiters")]
    MissingFrontmatter,
    #[error("invalid yaml: {0}")]
    InvalidYaml(#[from] serde_yaml::Error),
    #[error("validator task '{0}' must not have preconditions")]
    ValidatorWithPreconditions(String),
    #[error("validator task '{0}' cannot be marked complete")]
    ValidatorMarkedComplete(String),
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Task {
    pub id: String,
    pub parent: Option<String>,
    #[serde(default)]
    pub preconditions: Vec<String>,
    #[serde(default)]
    pub validations: Vec<ValidationItem>,
    pub title: Option<String>,
    #[serde(default)]
    pub validator: bool,
    #[serde(default)]
    pub complete: bool,
    #[serde(default)]
    pub in_progress: Option<u32>,
    #[serde(default, rename = "type")]
    pub task_type: TaskType,
    #[serde(skip)]
    pub description: String,
}

impl Task {
    pub fn validation_ids(&self) -> impl Iterator<Item = &str> {
        self.validations.iter().map(|v| v.id.as_str())
    }
}

/// Parses a task markdown file, extracting frontmatter and description.
///
/// # Examples
///
/// Parsing a valid task:
/// ```
/// use mont::task::{parse, ParseError, ValidationStatus};
///
/// let content = r#"---
/// id: test-task
/// parent: parent1
/// preconditions:
///   - pre1
/// validations:
///   - val1
/// title: Test Task
/// ---
///
/// This is the task description.
/// "#;
///
/// let task = parse(content).unwrap();
/// assert_eq!(task.id, "test-task");
/// assert_eq!(task.parent, Some("parent1".to_string()));
/// assert_eq!(task.preconditions, vec!["pre1"]);
/// assert_eq!(task.validations.len(), 1);
/// assert_eq!(task.validations[0].id, "val1");
/// assert_eq!(task.validations[0].status, ValidationStatus::Pending);
/// assert_eq!(task.title, Some("Test Task".to_string()));
/// assert!(!task.validator);
/// assert_eq!(task.description, "This is the task description.");
/// ```
///
/// Parsing a validator task (no preconditions allowed):
/// ```
/// use mont::task::{parse, ParseError};
///
/// let content = r#"---
/// id: test-validator
/// validator: true
/// parent: parent1
/// ---
///
/// Validator description.
/// "#;
///
/// let task = parse(content).unwrap();
/// assert!(task.validator);
/// assert_eq!(task.parent, Some("parent1".to_string()));
/// ```
///
/// Missing frontmatter returns an error:
/// ```
/// use mont::task::{parse, ParseError};
///
/// let result = parse("No frontmatter here");
/// assert!(matches!(result, Err(ParseError::MissingFrontmatter)));
/// ```
///
/// Missing required `id` field returns an error:
/// ```
/// use mont::task::{parse, ParseError};
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
/// Validator with preconditions returns an error:
/// ```
/// use mont::task::{parse, ParseError};
///
/// let content = r#"---
/// id: bad-validator
/// validator: true
/// preconditions:
///   - some-task
/// ---
///
/// This should fail.
/// "#;
///
/// let result = parse(content);
/// assert!(matches!(result, Err(ParseError::ValidatorWithPreconditions(_))));
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

    if task.validator && !task.preconditions.is_empty() {
        return Err(ParseError::ValidatorWithPreconditions(task.id));
    }

    if task.validator && task.complete {
        return Err(ParseError::ValidatorMarkedComplete(task.id));
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
parent: parent1
preconditions:
  - pre1
validations:
  - val1
title: Test Task
---

Task description here.
"#;
        let task = parse(content).unwrap();
        assert_eq!(task.id, "test-task");
        assert_eq!(task.parent, Some("parent1".to_string()));
        assert_eq!(task.preconditions, vec!["pre1"]);
        assert_eq!(task.validations.len(), 1);
        assert_eq!(task.validations[0].id, "val1");
        assert_eq!(task.validations[0].status, ValidationStatus::Pending);
        assert_eq!(task.title, Some("Test Task".to_string()));
        assert!(!task.validator);
        assert_eq!(task.description, "Task description here.");
    }

    #[test]
    fn test_parse_validator_without_preconditions() {
        let content = r#"---
id: my-validator
validator: true
parent: parent1
---

Validator description.
"#;
        let task = parse(content).unwrap();
        assert_eq!(task.id, "my-validator");
        assert!(task.validator);
        assert_eq!(task.parent, Some("parent1".to_string()));
        assert!(task.preconditions.is_empty());
    }

    #[test]
    fn test_parse_validator_with_preconditions_fails() {
        let content = r#"---
id: bad-validator
validator: true
preconditions:
  - some-task
---

Should fail.
"#;
        let result = parse(content);
        assert!(matches!(
            result,
            Err(ParseError::ValidatorWithPreconditions(id)) if id == "bad-validator"
        ));
    }

    #[test]
    fn test_parse_validator_marked_complete_fails() {
        let content = r#"---
id: complete-validator
validator: true
complete: true
---

Should fail.
"#;
        let result = parse(content);
        assert!(matches!(
            result,
            Err(ParseError::ValidatorMarkedComplete(id)) if id == "complete-validator"
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
        assert!(task.parent.is_none());
        assert!(task.preconditions.is_empty());
        assert!(task.validations.is_empty());
        assert!(task.title.is_none());
        assert!(!task.validator);
        assert!(!task.complete);
    }

    #[test]
    fn test_parse_complete_true() {
        let content = r#"---
id: done-task
complete: true
---

A completed task.
"#;
        let task = parse(content).unwrap();
        assert_eq!(task.id, "done-task");
        assert!(task.complete);
    }

    #[test]
    fn test_parse_complete_false() {
        let content = r#"---
id: incomplete-task
complete: false
---

An incomplete task.
"#;
        let task = parse(content).unwrap();
        assert_eq!(task.id, "incomplete-task");
        assert!(!task.complete);
    }

    #[test]
    fn test_parse_in_progress() {
        let content = r#"---
id: in-progress-task
in_progress: 1
---

A task in progress.
"#;
        let task = parse(content).unwrap();
        assert_eq!(task.id, "in-progress-task");
        assert_eq!(task.in_progress, Some(1));
    }

    #[test]
    fn test_parse_in_progress_incremented() {
        let content = r#"---
id: multi-revision-task
in_progress: 3
---

A task worked on across multiple revisions.
"#;
        let task = parse(content).unwrap();
        assert_eq!(task.id, "multi-revision-task");
        assert_eq!(task.in_progress, Some(3));
    }

    #[test]
    fn test_parse_no_in_progress() {
        let content = r#"---
id: normal-task
---

A normal task without in_progress.
"#;
        let task = parse(content).unwrap();
        assert_eq!(task.id, "normal-task");
        assert!(task.in_progress.is_none());
    }

    #[test]
    fn test_parse_type_bug() {
        let content = r#"---
id: fix-login-crash
title: Fix crash on login with empty password
type: bug
---

App crashes when user submits login form with empty password field.
"#;
        let task = parse(content).unwrap();
        assert_eq!(task.id, "fix-login-crash");
        assert_eq!(task.task_type, TaskType::Bug);
        assert_eq!(
            task.title,
            Some("Fix crash on login with empty password".to_string())
        );
    }

    #[test]
    fn test_parse_type_epic() {
        let content = r#"---
id: user-auth
title: User Authentication
type: epic
---

Epic for all authentication features.
"#;
        let task = parse(content).unwrap();
        assert_eq!(task.id, "user-auth");
        assert_eq!(task.task_type, TaskType::Epic);
    }

    #[test]
    fn test_parse_type_feature() {
        let content = r#"---
id: add-button
type: feature
---

A feature task.
"#;
        let task = parse(content).unwrap();
        assert_eq!(task.id, "add-button");
        assert_eq!(task.task_type, TaskType::Feature);
    }

    #[test]
    fn test_parse_type_defaults_to_feature() {
        let content = r#"---
id: regular-task
---

No type field specified.
"#;
        let task = parse(content).unwrap();
        assert_eq!(task.id, "regular-task");
        assert_eq!(task.task_type, TaskType::Feature);
    }

    #[test]
    fn test_parse_validation_with_status_passed() {
        let content = r#"---
id: test-task
validations:
  - val1: passed
---

Task with passed validation.
"#;
        let task = parse(content).unwrap();
        assert_eq!(task.validations.len(), 1);
        assert_eq!(task.validations[0].id, "val1");
        assert_eq!(task.validations[0].status, ValidationStatus::Passed);
    }

    #[test]
    fn test_parse_validation_with_status_failed() {
        let content = r#"---
id: test-task
validations:
  - val1: failed
---

Task with failed validation.
"#;
        let task = parse(content).unwrap();
        assert_eq!(task.validations.len(), 1);
        assert_eq!(task.validations[0].id, "val1");
        assert_eq!(task.validations[0].status, ValidationStatus::Failed);
    }

    #[test]
    fn test_parse_validation_with_status_skipped() {
        let content = r#"---
id: test-task
validations:
  - val1: skipped
---

Task with skipped validation.
"#;
        let task = parse(content).unwrap();
        assert_eq!(task.validations.len(), 1);
        assert_eq!(task.validations[0].id, "val1");
        assert_eq!(task.validations[0].status, ValidationStatus::Skipped);
    }

    #[test]
    fn test_parse_mixed_validations() {
        let content = r#"---
id: test-task
validations:
  - val1
  - val2: passed
  - val3: failed
  - val4: skipped
---

Task with mixed validation statuses.
"#;
        let task = parse(content).unwrap();
        assert_eq!(task.validations.len(), 4);

        assert_eq!(task.validations[0].id, "val1");
        assert_eq!(task.validations[0].status, ValidationStatus::Pending);

        assert_eq!(task.validations[1].id, "val2");
        assert_eq!(task.validations[1].status, ValidationStatus::Passed);

        assert_eq!(task.validations[2].id, "val3");
        assert_eq!(task.validations[2].status, ValidationStatus::Failed);

        assert_eq!(task.validations[3].id, "val4");
        assert_eq!(task.validations[3].status, ValidationStatus::Skipped);
    }

    #[test]
    fn test_validation_ids_helper() {
        let content = r#"---
id: test-task
validations:
  - val1
  - val2: passed
  - val3: failed
---

Task description.
"#;
        let task = parse(content).unwrap();
        let ids: Vec<&str> = task.validation_ids().collect();
        assert_eq!(ids, vec!["val1", "val2", "val3"]);
    }
}
