use serde::Deserialize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("missing frontmatter delimiters")]
    MissingFrontmatter,
    #[error("invalid yaml: {0}")]
    InvalidYaml(#[from] serde_yaml::Error),
    #[error("validator task '{0}' must not have preconditions")]
    ValidatorWithPreconditions(String),
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Task {
    pub id: String,
    pub parent: Option<String>,
    #[serde(default)]
    pub preconditions: Vec<String>,
    #[serde(default)]
    pub validations: Vec<String>,
    pub title: Option<String>,
    #[serde(default)]
    pub validator: bool,
    #[serde(skip)]
    pub description: String,
}

/// Parses a task markdown file, extracting frontmatter and description.
///
/// # Examples
///
/// Parsing a valid task:
/// ```
/// use mont::task::{parse, ParseError};
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
/// assert_eq!(task.validations, vec!["val1"]);
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
        assert_eq!(task.validations, vec!["val1"]);
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
    }
}
