use serde::Deserialize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("missing frontmatter delimiters")]
    MissingFrontmatter,
    #[error("invalid yaml: {0}")]
    InvalidYaml(#[from] serde_yaml::Error),
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Task {
    pub id: String,
    #[serde(default)]
    pub subtasks: Vec<String>,
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
/// subtasks:
///   - sub1
///   - sub2
/// preconditions:
///   - pre1
/// validations:
///   - val1
/// title: Test Task
/// validator: true
/// ---
///
/// This is the task description.
/// "#;
///
/// let task = parse(content).unwrap();
/// assert_eq!(task.id, "test-task");
/// assert_eq!(task.subtasks, vec!["sub1", "sub2"]);
/// assert_eq!(task.preconditions, vec!["pre1"]);
/// assert_eq!(task.validations, vec!["val1"]);
/// assert_eq!(task.title, Some("Test Task".to_string()));
/// assert!(task.validator);
/// assert_eq!(task.description, "This is the task description.");
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
    Ok(task)
}
