//! Shared utilities for commands, particularly temp file management.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::context::graph::is_available;
use crate::error_fmt::{AppError, IoResultExt, ParseResultExt};
use crate::{parse, Task, TaskGraph};

/// Filter options for interactive task picker.
#[derive(Clone, Copy)]
pub enum TaskFilter {
    /// Only non-complete tasks (default for most commands)
    Active,
    /// Only in-progress tasks
    InProgress,
    /// All tasks including complete
    All,
    /// Only ready tasks (not complete, not gates, all dependencies complete)
    Ready,
    /// Only jots (non-complete)
    Jots,
}

/// Pick a task interactively using fzf.
///
/// Returns the selected task ID, or an error if:
/// - fzf is not installed
/// - User cancelled the picker
/// - No matching tasks exist
pub fn pick_task(graph: &TaskGraph, filter: TaskFilter) -> Result<String, AppError> {
    // Check if fzf is installed
    if Command::new("fzf")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_err()
    {
        return Err(AppError::FzfNotFound);
    }

    // Get filtered tasks
    let mut tasks: Vec<_> = graph
        .values()
        .filter(|t| match filter {
            TaskFilter::Active => !t.is_complete(),
            TaskFilter::InProgress => t.is_in_progress(),
            TaskFilter::All => true,
            TaskFilter::Ready => !t.is_complete() && !t.is_gate() && is_available(t, graph),
            TaskFilter::Jots => !t.is_complete() && t.is_jot(),
        })
        .collect();

    if tasks.is_empty() {
        return Err(AppError::NoActiveTasks);
    }

    tasks.sort_by(|a, b| a.id.cmp(&b.id));

    // Calculate column widths for aligned table
    let max_id_len = tasks.iter().map(|t| t.id.len()).max().unwrap_or(0);

    // Build the input for fzf as aligned table: [type]  id  title
    let lines: Vec<String> = tasks
        .iter()
        .map(|t| {
            let type_tag = match t.task_type {
                crate::TaskType::Task => "[task]",
                crate::TaskType::Jot => "[jot] ",
                crate::TaskType::Gate => "[gate]",
            };
            let title = t.title.as_deref().unwrap_or("");
            format!("{}  {:max_id_len$}  {}", type_tag, t.id, title)
        })
        .collect();

    let input = lines.join("\n");

    // Run fzf with preview
    // Use {2} to extract the ID (second field) for the preview command
    let mut child = Command::new("fzf")
        .args([
            "--preview", "mont show {2}",
            "--preview-window", "right:60%:wrap",
            "--height", "80%",
            "--layout", "reverse",
            "--border",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .with_context("failed to spawn fzf")?;

    // Write task lines to fzf's stdin
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(input.as_bytes())
            .with_context("failed to write to fzf stdin")?;
    }

    let output = child.wait_with_output()
        .with_context("failed to wait for fzf")?;

    // Reset terminal state after fzf exits - fzf uses /dev/tty for input and may
    // leave the terminal in an altered state that prevents subsequent TUI apps
    // (like Claude) from working properly
    let _ = Command::new("stty")
        .arg("sane")
        .stdin(Stdio::inherit())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    if !output.status.success() {
        return Err(AppError::PickerCancelled);
    }

    let selected_line = String::from_utf8_lossy(&output.stdout)
        .trim()
        .to_string();

    if selected_line.is_empty() {
        return Err(AppError::PickerCancelled);
    }

    // Extract the ID from the selected line: "[type] id - title"
    // The ID is the second space-separated field
    let id = selected_line
        .split_whitespace()
        .nth(1)
        .ok_or(AppError::PickerCancelled)?
        .to_string();

    Ok(id)
}

/// Resolve a list of IDs, expanding `?` placeholders via interactive picker.
///
/// Input can be:
/// - A slice of IDs where some may be `?`
/// - Each `?` triggers an fzf picker
///
/// Returns a deduplicated Vec of resolved IDs in order of first occurrence.
///
/// # Examples
/// - `["task1", "task2"]` → `["task1", "task2"]`
/// - `["?"]` → `["<picked-task>"]`
/// - `["task1", "?", "?"]` → `["task1", "<picked1>", "<picked2>"]`
pub fn resolve_ids(
    graph: &TaskGraph,
    ids: &[String],
    filter: TaskFilter,
) -> Result<Vec<String>, AppError> {
    use std::collections::HashSet;

    let mut resolved = Vec::new();
    let mut seen = HashSet::new();

    for id in ids {
        let actual_id = if id == "?" {
            pick_task(graph, filter)?
        } else {
            id.clone()
        };

        // Deduplicate while preserving order
        if seen.insert(actual_id.clone()) {
            resolved.push(actual_id);
        }
    }

    Ok(resolved)
}

/// Create a temp file containing one or more tasks.
///
/// The file is named `{ULID}_{suffix}.md` in the system temp directory.
/// Tasks are serialized as standard markdown with `---` frontmatter delimiters.
/// Multiple tasks are separated by their frontmatter delimiters.
///
/// If `comment` is provided, it will be prepended as `# ` lines at the top of the file.
pub fn make_temp_file(suffix: &str, tasks: &[Task], comment: Option<&str>) -> Result<PathBuf, AppError> {
    let ulid = ulid::Ulid::new();
    let filename = format!("{}_{}.md", ulid, suffix);
    let path = std::env::temp_dir().join(filename);

    let mut content = String::new();

    // Add comment lines at the top
    if let Some(comment_text) = comment {
        for line in comment_text.lines() {
            content.push_str("# ");
            content.push_str(line);
            content.push('\n');
        }
        content.push('\n');
    }

    // Add tasks
    let tasks_content = tasks
        .iter()
        .map(|t| t.to_markdown())
        .collect::<Vec<_>>()
        .join("\n");
    content.push_str(&tasks_content);

    std::fs::write(&path, &content)
        .with_context(&format!("failed to write temp file {}", path.display()))?;

    Ok(path)
}

/// Parse a temp file containing one or more tasks.
///
/// Each task in the file should have standard markdown frontmatter (`---` delimiters).
/// Multiple tasks are separated by their frontmatter delimiters.
pub fn parse_temp_file(path: &Path) -> Result<Vec<Task>, AppError> {
    let content = std::fs::read_to_string(path)
        .with_context(&format!("failed to read temp file {}", path.display()))?;

    parse_multi_task_content(&content, path)
}

/// Parse content containing one or more tasks.
///
/// Parsing rule:
/// - Odd `---` lines start a new task's frontmatter
/// - Even `---` lines end frontmatter, start body
/// - Content before the first `---` is ignored (allows for comments/instructions)
pub fn parse_multi_task_content(content: &str, path: &Path) -> Result<Vec<Task>, AppError> {
    let mut tasks = Vec::new();
    let mut current_task = String::new();
    let mut delimiter_count = 0;

    for line in content.lines() {
        if line.trim() == "---" {
            delimiter_count += 1;

            if delimiter_count % 2 == 1 {
                // Odd: start of new task's frontmatter
                // Save previous task if we have one
                if !current_task.is_empty() {
                    let task = parse(&current_task)
                        .with_path(&path.display().to_string())?;
                    tasks.push(task);
                    current_task = String::new();
                }
            }
            // Always add the delimiter to current task
            current_task.push_str(line);
            current_task.push('\n');
        } else if delimiter_count > 0 {
            // We're inside a task (after first ---)
            current_task.push_str(line);
            current_task.push('\n');
        }
        // Lines before the first --- are ignored (comments/instructions)
    }

    // Don't forget the last task
    if !current_task.is_empty() && delimiter_count >= 2 {
        let task = parse(&current_task)
            .with_path(&path.display().to_string())?;
        tasks.push(task);
    }

    Ok(tasks)
}

/// Find temp files matching the given suffix.
///
/// Returns paths matching `*_{suffix}.md` in the system temp directory,
/// ordered by ULID descending (most recent first).
pub fn find_temp_files(suffix: &str) -> Vec<PathBuf> {
    let temp_dir = std::env::temp_dir();
    let pattern = format!("_{}.md", suffix);

    let Ok(entries) = std::fs::read_dir(&temp_dir) else {
        return Vec::new();
    };

    let mut files: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.ends_with(&pattern))
        })
        .collect();

    // Sort by filename descending (ULIDs are lexicographically sortable by time)
    files.sort_by(|a, b| b.file_name().cmp(&a.file_name()));

    files
}

/// Find the most recent temp file matching the given suffix.
///
/// Returns the path to the most recently created temp file with the given suffix,
/// or None if no matching files exist.
pub fn find_most_recent_temp_file(suffix: &str) -> Option<PathBuf> {
    find_temp_files(suffix).into_iter().next()
}

/// Mode for the multieditor comment header.
#[derive(Debug, Clone, Copy)]
pub enum MultiEditMode {
    /// Creating new tasks (empty editor)
    Create,
    /// Editing existing tasks
    Edit,
    /// Creating with a specific type template
    CreateWithType(crate::TaskType),
}

/// Build the instruction comment for multieditor temp files.
pub fn build_multiedit_comment(mode: MultiEditMode) -> String {
    match mode {
        MultiEditMode::Create => {
            r#"Create tasks below. Each task starts with --- and ends with ---
Tasks without an id: field will get an auto-generated ID.

Example:
---
id: my-task
title: My Task Title
after:
  - dependency-task
---
Task description here."#.to_string()
        }
        MultiEditMode::Edit => {
            r#"Edit tasks below. Each task starts with --- and ends with ---
- Change any field to update
- To rename: add new_id: new-name (keeps references)
- Delete a task block to delete it
- Add new task blocks to create new tasks
- Tasks without an id: field will get an auto-generated ID"#.to_string()
        }
        MultiEditMode::CreateWithType(task_type) => {
            let type_str = match task_type {
                crate::TaskType::Task => "task",
                crate::TaskType::Jot => "jot",
                crate::TaskType::Gate => "gate",
            };
            format!(
                r#"Create {} tasks below. Each task starts with --- and ends with ---
Tasks without an id: field will get an auto-generated ID.

Example:
---
id: my-{}
title: My {} Title
type: {}
---
Description here."#,
                type_str, type_str, type_str.to_uppercase(), type_str
            )
        }
    }
}

/// Remove a temp file.
pub fn remove_temp_file(path: &Path) -> Result<(), AppError> {
    std::fs::remove_file(path)
        .with_context(&format!("failed to remove temp file {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TaskType;

    #[test]
    fn test_make_and_parse_single_task() {
        let task = Task {
            id: "test-task".to_string(),
            new_id: None,
            title: Some("Test Title".to_string()),
            description: "Test description".to_string(),
            before: vec![],
            after: vec![],
            gates: vec![],
            task_type: TaskType::Task,
            status: None,
            deleted: false,
        };

        let path = make_temp_file("test", &[task.clone()], None).unwrap();
        assert!(path.exists());

        let parsed = parse_temp_file(&path).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].id, "test-task");
        assert_eq!(parsed[0].title, Some("Test Title".to_string()));

        remove_temp_file(&path).unwrap();
    }

    #[test]
    fn test_make_and_parse_multiple_tasks() {
        let tasks = vec![
            Task {
                id: "task-one".to_string(),
                new_id: None,
                title: Some("First Task".to_string()),
                description: "First description".to_string(),
                before: vec![],
                after: vec![],
                gates: vec![],
                task_type: TaskType::Task,
                status: None,
                deleted: false,
            },
            Task {
                id: "task-two".to_string(),
                new_id: None,
                title: Some("Second Task".to_string()),
                description: "Second description".to_string(),
                before: vec![],
                after: vec!["task-one".to_string()],
                gates: vec![],
                task_type: TaskType::Task,
                status: None,
                deleted: false,
            },
        ];

        let path = make_temp_file("multi", &tasks, None).unwrap();
        let parsed = parse_temp_file(&path).unwrap();

        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].id, "task-one");
        assert_eq!(parsed[1].id, "task-two");
        assert_eq!(parsed[1].after, vec!["task-one".to_string()]);

        remove_temp_file(&path).unwrap();
    }

    #[test]
    fn test_make_with_comment() {
        let task = Task {
            id: "commented-task".to_string(),
            new_id: None,
            title: Some("Task".to_string()),
            description: String::new(),
            before: vec![],
            after: vec![],
            gates: vec![],
            task_type: TaskType::Task,
            status: None,
            deleted: false,
        };

        let comment = "Instructions for editing\nLine two of instructions";
        let path = make_temp_file("comment_test", &[task], Some(comment)).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.starts_with("# Instructions for editing\n# Line two of instructions\n"));

        let parsed = parse_temp_file(&path).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].id, "commented-task");

        remove_temp_file(&path).unwrap();
    }

    #[test]
    fn test_parse_with_leading_comments() {
        let content = r#"# Instructions
# This is a comment
# Define your tasks below

---
id: my-task
title: My Task
---
Description here
"#;
        let path = std::env::temp_dir().join("test_comments.md");
        std::fs::write(&path, content).unwrap();

        let parsed = parse_temp_file(&path).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].id, "my-task");

        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn test_find_temp_files_ordering() {
        // Create files with known ULIDs (older first)
        let suffix = "find_test";
        let temp_dir = std::env::temp_dir();

        let file1 = temp_dir.join(format!("01ARZ3NDEKTSV4RRFFQ69G5FAV_{}.md", suffix));
        let file2 = temp_dir.join(format!("01BRZ3NDEKTSV4RRFFQ69G5FAV_{}.md", suffix));
        let file3 = temp_dir.join(format!("01CRZ3NDEKTSV4RRFFQ69G5FAV_{}.md", suffix));

        std::fs::write(&file1, "").unwrap();
        std::fs::write(&file2, "").unwrap();
        std::fs::write(&file3, "").unwrap();

        let found = find_temp_files(suffix);

        // Should be ordered descending (most recent first)
        assert!(found.len() >= 3);
        let found_names: Vec<_> = found.iter().filter_map(|p| p.file_name()).collect();

        // file3 (01C...) should come before file2 (01B...) which comes before file1 (01A...)
        let pos1 = found_names.iter().position(|n| n.to_str().unwrap().contains("01ARZ"));
        let pos2 = found_names.iter().position(|n| n.to_str().unwrap().contains("01BRZ"));
        let pos3 = found_names.iter().position(|n| n.to_str().unwrap().contains("01CRZ"));

        assert!(pos3 < pos2);
        assert!(pos2 < pos1);

        // Cleanup
        std::fs::remove_file(&file1).unwrap();
        std::fs::remove_file(&file2).unwrap();
        std::fs::remove_file(&file3).unwrap();
    }
}
