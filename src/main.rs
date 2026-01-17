use clap::{Args, Parser, Subcommand};
use owo_colors::OwoColorize;
use std::path::PathBuf;

use mont::error_fmt::{AppError, IoResultExt, ParseResultExt, ValidationResultExt};
use mont::task::TaskType;
use mont::{graph, render, task, validations};

#[derive(Parser)]
#[command(name = "mont")]
#[command(about = "Task management and agent coordination")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

/// Shared task field arguments used by both `new` and `edit` commands
#[derive(Args, Clone)]
struct TaskFields {
    /// Title for the task
    #[arg(long, short)]
    title: Option<String>,
    /// Description (markdown body)
    #[arg(long, short)]
    description: Option<String>,
    /// Before target task IDs (comma-separated or repeat flag)
    #[arg(long, short, value_delimiter = ',')]
    before: Vec<String>,
    /// After dependency task IDs (comma-separated or repeat flag)
    #[arg(long, value_delimiter = ',')]
    after: Vec<String>,
    /// Validation task IDs (comma-separated or repeat flag)
    #[arg(long, value_delimiter = ',')]
    validation: Vec<String>,
    /// Task type (feature, bug)
    #[arg(long, short = 'T', value_parser = parse_task_type)]
    r#type: Option<TaskType>,
    /// Open in editor after creation/editing (optionally specify editor name)
    #[arg(long, short)]
    editor: Option<Option<String>>,
}

#[derive(Subcommand)]
enum Commands {
    /// List all tasks in the task graph
    List {
        /// Show completed tasks (hidden by default)
        #[arg(long)]
        show_completed: bool,
    },
    /// Show tasks ready to work on
    Ready,
    /// Validate the task graph
    Check {
        /// Specific task ID to validate (validates entire graph if not provided)
        id: Option<String>,
    },
    /// Create a new task
    New {
        /// Unique task ID (generated if not provided)
        #[arg(long)]
        id: Option<String>,
        #[command(flatten)]
        fields: TaskFields,
        /// Resume editing a temp file from a previous failed validation
        #[arg(long, conflicts_with_all = ["id", "title", "description", "before", "after", "validation", "type"])]
        resume: Option<PathBuf>,
    },
    /// Edit an existing task
    Edit {
        /// Task ID to edit (original ID when using --resume)
        id: String,
        /// New task ID (renames the task and updates references)
        #[arg(long)]
        new_id: Option<String>,
        #[command(flatten)]
        fields: TaskFields,
        /// Resume editing a temp file from a previous failed validation
        #[arg(long, conflicts_with_all = ["new_id", "title", "description", "before", "after", "validation", "type"])]
        resume: Option<PathBuf>,
    },
    /// Delete a task and remove all references to it
    Delete {
        /// Task ID to delete
        id: String,
        /// Force deletion without confirmation prompt
        #[arg(long, short)]
        force: bool,
    },
}

const TASKS_DIR: &str = ".tasks";

fn parse_task_type(s: &str) -> Result<TaskType, String> {
    match s.to_lowercase().as_str() {
        "feature" => Ok(TaskType::Feature),
        "bug" => Ok(TaskType::Bug),
        _ => Err(format!(
            "invalid task type '{}', must be one of: feature, bug",
            s
        )),
    }
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::List { show_completed } => {
            if let Err(e) = list_tasks(show_completed) {
                eprint!("{}", e);
                std::process::exit(1);
            }
        }
        Commands::Ready => {
            if let Err(e) = ready_tasks() {
                eprint!("{}", e);
                std::process::exit(1);
            }
        }
        Commands::Check { id } => {
            if let Err(e) = check_tasks(id.as_deref()) {
                eprint!("{}", e);
                std::process::exit(1);
            }
        }
        Commands::New {
            id,
            fields,
            resume,
        } => {
            if let Err(e) = new_task(TASKS_DIR, id, fields, resume) {
                eprint!("{}", e);
                std::process::exit(1);
            }
        }
        Commands::Edit {
            id,
            new_id,
            fields,
            resume,
        } => {
            if let Err(e) = edit_task(TASKS_DIR, &id, new_id, fields, resume) {
                eprint!("{}", e);
                std::process::exit(1);
            }
        }
        Commands::Delete { id, force } => {
            if let Err(e) = delete_task(TASKS_DIR, &id, force) {
                eprint!("{}", e);
                std::process::exit(1);
            }
        }
    }
}

fn list_tasks(show_completed: bool) -> Result<(), AppError> {
    let dir = PathBuf::from(TASKS_DIR);

    if !dir.exists() {
        return Err(AppError::DirNotFound(TASKS_DIR.to_string()));
    }

    let mut paths: Vec<_> = std::fs::read_dir(&dir)
        .with_context(&format!("failed to read directory {}", TASKS_DIR))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "md"))
        .collect();
    paths.sort();

    let mut tasks = Vec::new();
    for path in &paths {
        let path_str = path.display().to_string();
        let content = std::fs::read_to_string(path).with_context(&format!("failed to read {}", path_str))?;
        let parsed = task::parse(&content).with_path(&path_str)?;
        tasks.push(parsed);
    }

    if tasks.is_empty() {
        println!("No tasks found in {}", TASKS_DIR);
        return Ok(());
    }

    let validated = graph::form_graph(tasks).with_tasks_dir(TASKS_DIR)?;

    let output = render::render_task_graph(&validated, show_completed);
    print!("{}", output);

    Ok(())
}

fn ready_tasks() -> Result<(), AppError> {
    let dir = PathBuf::from(TASKS_DIR);

    if !dir.exists() {
        return Err(AppError::DirNotFound(TASKS_DIR.to_string()));
    }

    let mut paths: Vec<_> = std::fs::read_dir(&dir)
        .with_context(&format!("failed to read directory {}", TASKS_DIR))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "md"))
        .collect();
    paths.sort();

    let mut tasks = Vec::new();
    for path in &paths {
        let path_str = path.display().to_string();
        let content = std::fs::read_to_string(path).with_context(&format!("failed to read {}", path_str))?;
        let parsed = task::parse(&content).with_path(&path_str)?;
        tasks.push(parsed);
    }

    if tasks.is_empty() {
        println!("No ready tasks");
        return Ok(());
    }

    let validated = graph::form_graph(tasks).with_tasks_dir(TASKS_DIR)?;
    let mut ready: Vec<_> = graph::available_tasks(&validated);
    ready.sort_by(|a, b| a.id.cmp(&b.id));

    if ready.is_empty() {
        println!("No ready tasks");
        return Ok(());
    }

    let max_id_len = ready.iter().map(|t| t.id.len()).max().unwrap_or(0);
    let max_title_len = ready
        .iter()
        .map(|t| render::truncate_title(t.title.as_deref().unwrap_or("")).len())
        .max()
        .unwrap_or(0);

    for task in ready {
        let title = render::truncate_title(task.title.as_deref().unwrap_or(""));
        let type_tag = match task.task_type {
            TaskType::Bug => format!("{}", "[bug]".red().bold()),
            TaskType::Feature => format!("{}", "[feature]".bright_green().bold()),
        };
        println!(
            "{}  {:max_title_len$}  {}",
            format!("{:max_id_len$}", task.id).bright_green().bold(),
            title,
            type_tag
        );
    }

    Ok(())
}

fn check_tasks(id: Option<&str>) -> Result<(), AppError> {
    let dir = PathBuf::from(TASKS_DIR);

    if !dir.exists() {
        return Err(AppError::DirNotFound(TASKS_DIR.to_string()));
    }

    let mut paths: Vec<_> = std::fs::read_dir(&dir)
        .with_context(&format!("failed to read directory {}", TASKS_DIR))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "md"))
        .collect();
    paths.sort();

    let mut tasks = Vec::new();
    for path in &paths {
        let path_str = path.display().to_string();
        let content = std::fs::read_to_string(path).with_context(&format!("failed to read {}", path_str))?;
        let parsed = task::parse(&content).with_path(&path_str)?;
        tasks.push(parsed);
    }

    if tasks.is_empty() {
        println!("No tasks found in {}", TASKS_DIR);
        return Ok(());
    }

    match id {
        Some(task_id) => check_single_task(&tasks, task_id),
        None => check_full_graph(tasks),
    }
}

fn check_single_task(tasks: &[task::Task], task_id: &str) -> Result<(), AppError> {
    let graph: validations::TaskGraph = tasks.iter().map(|t| (t.id.clone(), t.clone())).collect();

    let Some(task) = graph.get(task_id) else {
        return Err(AppError::TaskNotFound {
            task_id: task_id.to_string(),
            tasks_dir: TASKS_DIR.to_string(),
        });
    };

    validations::validate_task(task, &graph).with_tasks_dir(TASKS_DIR)?;

    println!("ok: task '{}' is valid", task_id);
    Ok(())
}

fn check_full_graph(tasks: Vec<task::Task>) -> Result<(), AppError> {
    let count = tasks.len();
    validations::validate_graph(tasks).with_tasks_dir(TASKS_DIR)?;
    println!("ok: {} tasks validated", count);
    Ok(())
}

fn new_task(
    tasks_dir: &str,
    id: Option<String>,
    fields: TaskFields,
    resume: Option<PathBuf>,
) -> Result<(), AppError> {
    let dir = PathBuf::from(tasks_dir);

    if !dir.exists() {
        std::fs::create_dir_all(&dir)
            .with_context(&format!("failed to create directory {}", tasks_dir))?;
    }

    // Resume mode: re-open a temp file from a previous failed validation
    if let Some(temp_path) = resume {
        let editor_name = fields.editor.flatten();
        return resume_from_temp(tasks_dir, &dir, &temp_path, editor_name.as_deref());
    }

    let existing_tasks = load_existing_tasks(&dir)?;
    let existing_ids: std::collections::HashSet<_> =
        existing_tasks.iter().map(|t| t.id.as_str()).collect();

    let task_id = resolve_task_id(id, fields.title.as_deref(), &existing_ids)?;

    let content = build_task_content(
        &task_id,
        &fields.title,
        &fields.description,
        &fields.before,
        &fields.after,
        &fields.validation,
        &fields.r#type,
    );

    // Editor mode: create temp file, open editor, validate on save
    if let Some(editor_opt) = fields.editor {
        let editor_name = editor_opt.as_deref();
        return create_with_editor(tasks_dir, &dir, &task_id, &content, editor_name);
    }

    // Non-editor mode: validate in memory, then write
    let new_task = task::parse(&content).with_path(&format!("{}/{}.md", tasks_dir, task_id))?;

    let mut all_tasks = existing_tasks;
    all_tasks.push(new_task);
    validations::validate_graph(all_tasks).with_tasks_dir(tasks_dir)?;

    let file_path = dir.join(format!("{}.md", task_id));
    std::fs::write(&file_path, &content)
        .with_context(&format!("failed to write {}", file_path.display()))?;

    println!("created: {}", file_path.display().to_string().bright_green());

    Ok(())
}

fn edit_task(
    tasks_dir: &str,
    id: &str,
    new_id: Option<String>,
    fields: TaskFields,
    resume: Option<PathBuf>,
) -> Result<(), AppError> {
    let dir = PathBuf::from(tasks_dir);

    if !dir.exists() {
        return Err(AppError::DirNotFound(tasks_dir.to_string()));
    }

    // Resume mode: re-open a temp file from a previous failed validation
    if let Some(temp_path) = resume {
        let editor_name = fields.editor.flatten();
        return resume_edit_from_temp(tasks_dir, &dir, id, &temp_path, editor_name.as_deref());
    }

    let file_path = dir.join(format!("{}.md", id));
    if !file_path.exists() {
        return Err(AppError::TaskNotFound {
            task_id: id.to_string(),
            tasks_dir: tasks_dir.to_string(),
        });
    }

    let original_content = std::fs::read_to_string(&file_path)
        .with_context(&format!("failed to read {}", file_path.display()))?;

    let original_task = task::parse(&original_content)
        .with_path(&file_path.display().to_string())?;

    let final_id = new_id.as_deref().unwrap_or(id);

    // Editor mode: copy to temp, open editor, validate on save
    if let Some(editor_opt) = fields.editor {
        let editor_name = editor_opt.as_deref();
        return edit_with_editor(tasks_dir, &dir, id, final_id, &original_content, editor_name);
    }

    // Check if any fields were provided (non-editor mode requires at least one change)
    let has_changes = new_id.is_some()
        || fields.title.is_some()
        || fields.description.is_some()
        || !fields.before.is_empty()
        || !fields.after.is_empty()
        || !fields.validation.is_empty()
        || fields.r#type.is_some();

    if !has_changes {
        return Err(AppError::NoChangesProvided);
    }

    // Build updated content by merging fields
    let updated_content = merge_task_content(
        &original_task,
        &original_content,
        final_id,
        &fields,
    );

    // Validate the updated task
    let updated_task = task::parse(&updated_content)
        .with_path(&format!("{}/{}.md", tasks_dir, final_id))?;

    // Load all tasks, replacing the original with the updated one
    let mut all_tasks = load_existing_tasks(&dir)?;
    all_tasks.retain(|t| t.id != id);
    all_tasks.push(updated_task);

    // If ID changed, update references in other tasks
    if let Some(ref new_id_val) = new_id {
        update_task_references(&dir, id, new_id_val, &mut all_tasks)?;
    }

    // Validate the entire graph
    validations::validate_graph(all_tasks).with_tasks_dir(tasks_dir)?;

    // Write the updated task
    let final_path = dir.join(format!("{}.md", final_id));
    std::fs::write(&final_path, &updated_content)
        .with_context(&format!("failed to write {}", final_path.display()))?;

    // If ID changed, remove the old file
    if new_id.is_some() && final_id != id {
        std::fs::remove_file(&file_path)
            .with_context(&format!("failed to remove {}", file_path.display()))?;
        println!("renamed: {} -> {}", id.bright_yellow(), final_id.bright_green());
    } else {
        println!("updated: {}", final_path.display().to_string().bright_green());
    }

    Ok(())
}

fn delete_task(tasks_dir: &str, id: &str, force: bool) -> Result<(), AppError> {
    let dir = PathBuf::from(tasks_dir);

    if !dir.exists() {
        return Err(AppError::DirNotFound(tasks_dir.to_string()));
    }

    let file_path = dir.join(format!("{}.md", id));
    if !file_path.exists() {
        return Err(AppError::TaskNotFound {
            task_id: id.to_string(),
            tasks_dir: tasks_dir.to_string(),
        });
    }

    let all_tasks = load_existing_tasks(&dir)?;

    // Find all references to this task
    let mut references: Vec<(String, String)> = Vec::new(); // (task_id, reference_type)

    for task in &all_tasks {
        if task.id == id {
            continue;
        }

        if task.before.contains(&id.to_string()) {
            references.push((task.id.clone(), "before".to_string()));
        }

        for dep in &task.after {
            if dep == id {
                references.push((task.id.clone(), "after".to_string()));
            }
        }

        for val in &task.validations {
            if val.id == id {
                references.push((task.id.clone(), "validation".to_string()));
            }
        }
    }

    // Show summary and ask for confirmation
    if !force {
        println!("{} {}", "Deleting task:".bold(), id.bright_yellow());

        if references.is_empty() {
            println!("  No other tasks reference this task.");
        } else {
            println!("\n{}:", "References to remove".bold());
            for (task_id, ref_type) in &references {
                println!("  {} {} reference in {}", "•".red(), ref_type, task_id.cyan());
            }
        }

        println!();
        print!("Continue? [y/N] ");
        std::io::Write::flush(&mut std::io::stdout())
            .with_context("failed to flush stdout")?;

        let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .with_context("failed to read input")?;

        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Aborted.");
            return Ok(());
        }
    }

    // Remove references from other tasks
    for (task_id, _) in &references {
        let task_path = dir.join(format!("{}.md", task_id));
        let content = std::fs::read_to_string(&task_path)
            .with_context(&format!("failed to read {}", task_path.display()))?;

        let updated = remove_reference_from_content(&content, id);

        std::fs::write(&task_path, &updated)
            .with_context(&format!("failed to write {}", task_path.display()))?;

        println!("  updated: {}", task_id.cyan());
    }

    // Delete the task file
    std::fs::remove_file(&file_path)
        .with_context(&format!("failed to remove {}", file_path.display()))?;

    println!("{} {}", "deleted:".red(), id.bright_yellow());

    Ok(())
}

fn remove_reference_from_content(content: &str, id_to_remove: &str) -> String {
    let mut result = String::new();
    let mut in_frontmatter = false;
    let mut delimiter_count = 0;
    let mut in_list_section = false;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed == "---" {
            delimiter_count += 1;
            in_frontmatter = delimiter_count == 1;
            result.push_str(line);
            result.push('\n');
            in_list_section = false;
            continue;
        }

        if in_frontmatter && delimiter_count < 2 {
            // Handle before: id_to_remove - remove the entire line
            if trimmed.starts_with("before:") {
                let value = trimmed.strip_prefix("before:").unwrap().trim();
                if value == id_to_remove {
                    continue; // Skip this line entirely
                }
            }

            // Track if we're in a list section (after:, validations:)
            if trimmed.ends_with(':') && !trimmed.contains(' ') {
                in_list_section = trimmed == "after:" || trimmed == "validations:";
            }

            // Handle list items: - id_to_remove
            if in_list_section && trimmed.starts_with("- ") {
                let value = trimmed.strip_prefix("- ").unwrap().trim();
                if value == id_to_remove {
                    continue; // Skip this line entirely
                }
            }
        }

        result.push_str(line);
        result.push('\n');
    }

    // Remove trailing newline if original didn't have one
    if !content.ends_with('\n') && result.ends_with('\n') {
        result.pop();
    }

    result
}

fn edit_with_editor(
    tasks_dir_str: &str,
    tasks_dir: &PathBuf,
    original_id: &str,
    task_id: &str,
    content: &str,
    editor_name: Option<&str>,
) -> Result<(), AppError> {
    let temp_dir = std::env::temp_dir();
    let temp_path = temp_dir.join(format!("mont-edit-{}.md", task_id));

    std::fs::write(&temp_path, content)
        .with_context(&format!("failed to write temp file {}", temp_path.display()))?;

    let mut cmd = mont::resolve_editor(editor_name, &temp_path)?;
    cmd.status()
        .with_context("failed to run editor")?;

    validate_and_update_edited(tasks_dir_str, tasks_dir, original_id, &temp_path, editor_name)
}

fn resume_edit_from_temp(
    tasks_dir_str: &str,
    tasks_dir: &PathBuf,
    original_id: &str,
    temp_path: &PathBuf,
    editor_name: Option<&str>,
) -> Result<(), AppError> {
    if !temp_path.exists() {
        return Err(AppError::TempFileNotFound(temp_path.display().to_string()));
    }

    // Verify the original task exists
    let original_path = tasks_dir.join(format!("{}.md", original_id));
    if !original_path.exists() {
        return Err(AppError::TaskNotFound {
            task_id: original_id.to_string(),
            tasks_dir: tasks_dir_str.to_string(),
        });
    }

    let mut cmd = mont::resolve_editor(editor_name, temp_path)?;
    cmd.status()
        .with_context("failed to run editor")?;

    validate_and_update_edited(tasks_dir_str, tasks_dir, original_id, temp_path, editor_name)
}

fn validate_and_update_edited(
    tasks_dir_str: &str,
    tasks_dir: &PathBuf,
    original_id: &str,
    temp_path: &PathBuf,
    editor_name: Option<&str>,
) -> Result<(), AppError> {
    let content = std::fs::read_to_string(temp_path)
        .with_context(&format!("failed to read temp file {}", temp_path.display()))?;

    let temp_path_str = temp_path.display().to_string();
    let parsed = match task::parse(&content).with_path(&temp_path_str) {
        Ok(t) => t,
        Err(e) => {
            return Err(AppError::EditTempValidationFailed {
                error: Box::new(e),
                original_id: original_id.to_string(),
                temp_path: temp_path_str,
                editor_name: editor_name.map(String::from),
            });
        }
    };

    let mut all_tasks = load_existing_tasks(tasks_dir)?;

    // Check if new ID conflicts with existing (excluding original)
    let id_changed = parsed.id != original_id;
    if id_changed {
        let existing_ids: std::collections::HashSet<_> =
            all_tasks.iter().filter(|t| t.id != original_id).map(|t| t.id.as_str()).collect();
        if existing_ids.contains(parsed.id.as_str()) {
            return Err(AppError::EditTempValidationFailed {
                error: Box::new(AppError::IdAlreadyExists(parsed.id.clone())),
                original_id: original_id.to_string(),
                temp_path: temp_path_str,
                editor_name: editor_name.map(String::from),
            });
        }
    }

    // Replace original task with updated one
    all_tasks.retain(|t| t.id != original_id);
    all_tasks.push(parsed.clone());

    // If ID changed, update references in other tasks
    if id_changed {
        update_task_references(tasks_dir, original_id, &parsed.id, &mut all_tasks)?;
    }

    // Validate the entire graph
    if let Err(e) = validations::validate_graph(all_tasks).with_tasks_dir(tasks_dir_str) {
        return Err(AppError::EditTempValidationFailed {
            error: Box::new(e),
            original_id: original_id.to_string(),
            temp_path: temp_path_str,
            editor_name: editor_name.map(String::from),
        });
    }

    // Write the updated task
    let final_path = tasks_dir.join(format!("{}.md", parsed.id));
    std::fs::write(&final_path, &content)
        .with_context(&format!("failed to write {}", final_path.display()))?;

    // If ID changed, remove the old file
    if id_changed {
        let old_path = tasks_dir.join(format!("{}.md", original_id));
        std::fs::remove_file(&old_path)
            .with_context(&format!("failed to remove {}", old_path.display()))?;
        println!("renamed: {} -> {}", original_id.bright_yellow(), parsed.id.bright_green());
    } else {
        println!("updated: {}", final_path.display().to_string().bright_green());
    }

    // Clean up temp file
    std::fs::remove_file(temp_path)
        .with_context(&format!("failed to remove temp file {}", temp_path.display()))?;

    Ok(())
}

fn merge_task_content(
    original: &task::Task,
    original_content: &str,
    new_id: &str,
    fields: &TaskFields,
) -> String {
    // Use provided fields or fall back to original values
    let title = fields.title.as_ref().or(original.title.as_ref());
    let task_type = fields.r#type.as_ref().or(Some(&original.task_type));

    // For lists, use provided if non-empty, otherwise keep original
    let before_targets: Vec<String> = if !fields.before.is_empty() {
        fields.before.clone()
    } else {
        original.before.clone()
    };

    let after_deps: Vec<String> = if !fields.after.is_empty() {
        fields.after.clone()
    } else {
        original.after.clone()
    };

    let validations_list: Vec<String> = if !fields.validation.is_empty() {
        fields.validation.iter().map(|v| v.clone()).collect()
    } else {
        original.validations.iter().map(|v| v.id.clone()).collect()
    };

    // Extract the body (content after frontmatter)
    let body = if fields.description.is_some() {
        fields.description.clone()
    } else {
        extract_body(original_content)
    };

    build_task_content(
        new_id,
        &title.cloned(),
        &body,
        &before_targets,
        &after_deps,
        &validations_list,
        &task_type.cloned(),
    )
}

fn extract_body(content: &str) -> Option<String> {
    // Find the end of frontmatter (second ---)
    let mut delimiter_count = 0;
    let mut end_idx = 0;
    for (i, line) in content.lines().enumerate() {
        if line.trim() == "---" {
            delimiter_count += 1;
            if delimiter_count == 2 {
                // Find byte position after this line
                end_idx = content.lines().take(i + 1).map(|l| l.len() + 1).sum::<usize>();
                break;
            }
        }
    }

    if delimiter_count < 2 {
        return None;
    }

    let body = content[end_idx..].trim();
    if body.is_empty() {
        None
    } else {
        Some(body.to_string())
    }
}

fn update_task_references(
    tasks_dir: &PathBuf,
    old_id: &str,
    new_id: &str,
    all_tasks: &mut Vec<task::Task>,
) -> Result<(), AppError> {
    for task in all_tasks.iter_mut() {
        let mut changed = false;

        // Update before references
        for target in task.before.iter_mut() {
            if target == old_id {
                *target = new_id.to_string();
                changed = true;
            }
        }

        // Update after dependencies
        for dep in task.after.iter_mut() {
            if dep == old_id {
                *dep = new_id.to_string();
                changed = true;
            }
        }

        // Update validations
        for val in task.validations.iter_mut() {
            if val.id == old_id {
                val.id = new_id.to_string();
                changed = true;
            }
        }

        // If task was modified, rewrite its file
        if changed {
            let task_path = tasks_dir.join(format!("{}.md", task.id));
            let content = std::fs::read_to_string(&task_path)
                .with_context(&format!("failed to read {}", task_path.display()))?;

            // Simple string replacement in the file content
            let updated_content = update_references_in_content(&content, old_id, new_id);

            std::fs::write(&task_path, &updated_content)
                .with_context(&format!("failed to write {}", task_path.display()))?;

            println!("  updated reference in: {}", task.id.cyan());
        }
    }

    Ok(())
}

fn update_references_in_content(content: &str, old_id: &str, new_id: &str) -> String {
    // Update references in YAML frontmatter
    // This handles: before: old_id, - old_id in lists
    let mut result = String::new();
    let mut in_frontmatter = false;
    let mut delimiter_count = 0;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed == "---" {
            delimiter_count += 1;
            in_frontmatter = delimiter_count == 1;
            result.push_str(line);
            result.push('\n');
            continue;
        }

        if in_frontmatter && delimiter_count < 2 {
            // Handle before: old_id
            if trimmed.starts_with("before:") {
                let value = trimmed.strip_prefix("before:").unwrap().trim();
                if value == old_id {
                    let indent = line.len() - line.trim_start().len();
                    result.push_str(&" ".repeat(indent));
                    result.push_str(&format!("before: {}\n", new_id));
                    continue;
                }
            }

            // Handle list items: - old_id
            if trimmed.starts_with("- ") {
                let value = trimmed.strip_prefix("- ").unwrap().trim();
                if value == old_id {
                    let indent = line.len() - line.trim_start().len();
                    result.push_str(&" ".repeat(indent));
                    result.push_str(&format!("- {}\n", new_id));
                    continue;
                }
            }
        }

        result.push_str(line);
        result.push('\n');
    }

    // Remove trailing newline if original didn't have one
    if !content.ends_with('\n') && result.ends_with('\n') {
        result.pop();
    }

    result
}

fn create_with_editor(
    tasks_dir_str: &str,
    tasks_dir: &PathBuf,
    task_id: &str,
    initial_content: &str,
    editor_name: Option<&str>,
) -> Result<(), AppError> {
    let temp_dir = std::env::temp_dir();
    let temp_path = temp_dir.join(format!("mont-task-{}.md", task_id));

    std::fs::write(&temp_path, initial_content)
        .with_context(&format!("failed to write temp file {}", temp_path.display()))?;

    let mut cmd = mont::resolve_editor(editor_name, &temp_path)?;
    cmd.status()
        .with_context("failed to run editor")?;

    validate_and_copy_temp(tasks_dir_str, tasks_dir, &temp_path, editor_name)
}

fn resume_from_temp(
    tasks_dir_str: &str,
    tasks_dir: &PathBuf,
    temp_path: &PathBuf,
    editor_name: Option<&str>,
) -> Result<(), AppError> {
    if !temp_path.exists() {
        return Err(AppError::TempFileNotFound(temp_path.display().to_string()));
    }

    let mut cmd = mont::resolve_editor(editor_name, temp_path)?;
    cmd.status()
        .with_context("failed to run editor")?;

    validate_and_copy_temp(tasks_dir_str, tasks_dir, temp_path, editor_name)
}

fn validate_and_copy_temp(
    tasks_dir_str: &str,
    tasks_dir: &PathBuf,
    temp_path: &PathBuf,
    editor_name: Option<&str>,
) -> Result<(), AppError> {
    let content = std::fs::read_to_string(temp_path)
        .with_context(&format!("failed to read temp file {}", temp_path.display()))?;

    let temp_path_str = temp_path.display().to_string();
    let parsed = match task::parse(&content).with_path(&temp_path_str) {
        Ok(t) => t,
        Err(e) => {
            return Err(AppError::TempValidationFailed {
                error: Box::new(e),
                temp_path: temp_path_str,
                editor_name: editor_name.map(String::from),
            });
        }
    };

    let existing_tasks = load_existing_tasks(tasks_dir)?;
    let existing_ids: std::collections::HashSet<_> =
        existing_tasks.iter().map(|t| t.id.as_str()).collect();

    if existing_ids.contains(parsed.id.as_str()) {
        return Err(AppError::TempValidationFailed {
            error: Box::new(AppError::IdAlreadyExists(parsed.id.clone())),
            temp_path: temp_path_str,
            editor_name: editor_name.map(String::from),
        });
    }

    let mut all_tasks = existing_tasks;
    all_tasks.push(parsed.clone());

    if let Err(e) = validations::validate_graph(all_tasks).with_tasks_dir(tasks_dir_str) {
        return Err(AppError::TempValidationFailed {
            error: Box::new(e),
            temp_path: temp_path_str,
            editor_name: editor_name.map(String::from),
        });
    }

    let final_path = tasks_dir.join(format!("{}.md", parsed.id));
    std::fs::write(&final_path, &content)
        .with_context(&format!("failed to write {}", final_path.display()))?;

    std::fs::remove_file(temp_path)
        .with_context(&format!("failed to remove temp file {}", temp_path.display()))?;

    println!("created: {}", final_path.display().to_string().bright_green());

    Ok(())
}

fn load_existing_tasks(dir: &PathBuf) -> Result<Vec<task::Task>, AppError> {
    let mut paths: Vec<_> = std::fs::read_dir(dir)
        .with_context(&format!("failed to read directory {}", TASKS_DIR))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "md"))
        .collect();
    paths.sort();

    let mut tasks = Vec::new();
    for path in &paths {
        let path_str = path.display().to_string();
        let content =
            std::fs::read_to_string(path).with_context(&format!("failed to read {}", path_str))?;
        let parsed = task::parse(&content).with_path(&path_str)?;
        tasks.push(parsed);
    }

    Ok(tasks)
}

fn resolve_task_id(
    id: Option<String>,
    title: Option<&str>,
    existing_ids: &std::collections::HashSet<&str>,
) -> Result<String, AppError> {
    if let Some(id) = id {
        return Ok(id);
    }

    if title.is_none() {
        return Err(AppError::IdOrTitleRequired);
    }

    const MAX_ATTEMPTS: u32 = 100;

    for _ in 0..MAX_ATTEMPTS {
        let Some(candidate) = petname::petname(2, "-") else {
            continue;
        };
        if !existing_ids.contains(candidate.as_str()) {
            return Ok(candidate);
        }
    }

    Err(AppError::IdGenerationFailed {
        attempts: MAX_ATTEMPTS,
    })
}

fn build_task_content(
    id: &str,
    title: &Option<String>,
    description: &Option<String>,
    before: &[String],
    after: &[String],
    validations_list: &[String],
    task_type: &Option<TaskType>,
) -> String {
    let mut content = String::new();
    content.push_str("---\n");
    content.push_str(&format!("id: {}\n", id));

    if let Some(t) = title {
        content.push_str(&format!("title: {}\n", t));
    }

    if let Some(tt) = task_type {
        let type_str = match tt {
            TaskType::Feature => "feature",
            TaskType::Bug => "bug",
        };
        content.push_str(&format!("type: {}\n", type_str));
    }

    if !before.is_empty() {
        content.push_str("before:\n");
        for target in before {
            content.push_str(&format!("  - {}\n", target));
        }
    }

    if !after.is_empty() {
        content.push_str("after:\n");
        for dep in after {
            content.push_str(&format!("  - {}\n", dep));
        }
    }

    if !validations_list.is_empty() {
        content.push_str("validations:\n");
        for val in validations_list {
            content.push_str(&format!("  - {}\n", val));
        }
    }

    content.push_str("---\n\n");

    if let Some(d) = description {
        content.push_str(d);
        content.push('\n');
    }

    content
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_temp_tasks_dir() -> TempDir {
        tempfile::tempdir().expect("failed to create temp dir")
    }

    fn empty_fields() -> TaskFields {
        TaskFields {
            title: None,
            description: None,
            before: vec![],
            after: vec![],
            validation: vec![],
            r#type: None,
            editor: None,
        }
    }

    // Tests for mont new
    #[test]
    fn test_new_task_creates_file() {
        let temp_dir = create_temp_tasks_dir();
        let tasks_dir = temp_dir.path().to_str().unwrap();

        let fields = TaskFields {
            title: Some("Test task".to_string()),
            ..empty_fields()
        };

        let result = new_task(tasks_dir, Some("test-task".to_string()), fields, None);
        assert!(result.is_ok());

        let file_path = temp_dir.path().join("test-task.md");
        assert!(file_path.exists());

        let content = std::fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("id: test-task"));
        assert!(content.contains("title: Test task"));
    }

    #[test]
    fn test_new_task_duplicate_id_fails() {
        let temp_dir = create_temp_tasks_dir();
        let tasks_dir = temp_dir.path().to_str().unwrap();

        // Create first task
        let fields1 = TaskFields {
            title: Some("First task".to_string()),
            ..empty_fields()
        };
        new_task(tasks_dir, Some("my-task".to_string()), fields1, None).unwrap();

        // Try to create second task with same ID
        let fields2 = TaskFields {
            title: Some("Second task".to_string()),
            ..empty_fields()
        };
        let result = new_task(tasks_dir, Some("my-task".to_string()), fields2, None);
        assert!(result.is_err());
    }

    // Tests for mont edit
    #[test]
    fn test_edit_task_updates_title() {
        let temp_dir = create_temp_tasks_dir();
        let tasks_dir = temp_dir.path().to_str().unwrap();

        // Create a task
        let fields = TaskFields {
            title: Some("Original title".to_string()),
            ..empty_fields()
        };
        new_task(tasks_dir, Some("edit-test".to_string()), fields, None).unwrap();

        // Edit the task
        let edit_fields = TaskFields {
            title: Some("Updated title".to_string()),
            ..empty_fields()
        };
        let result = edit_task(tasks_dir, "edit-test", None, edit_fields, None);
        assert!(result.is_ok());

        let content = std::fs::read_to_string(temp_dir.path().join("edit-test.md")).unwrap();
        assert!(content.contains("title: Updated title"));
    }

    #[test]
    fn test_edit_task_rename_propagates_references() {
        let temp_dir = create_temp_tasks_dir();
        let tasks_dir = temp_dir.path().to_str().unwrap();

        // Create parent task
        let parent_fields = TaskFields {
            title: Some("Parent".to_string()),
            ..empty_fields()
        };
        new_task(tasks_dir, Some("parent-task".to_string()), parent_fields, None).unwrap();

        // Create child task with before reference
        let child_fields = TaskFields {
            title: Some("Child".to_string()),
            before: vec!["parent-task".to_string()],
            ..empty_fields()
        };
        new_task(tasks_dir, Some("child-task".to_string()), child_fields, None).unwrap();

        // Rename parent
        let result = edit_task(
            tasks_dir,
            "parent-task",
            Some("renamed-parent".to_string()),
            empty_fields(),
            None,
        );
        assert!(result.is_ok());

        // Verify old file removed, new file exists
        assert!(!temp_dir.path().join("parent-task.md").exists());
        assert!(temp_dir.path().join("renamed-parent.md").exists());

        // Verify child's before reference was updated
        let child_content = std::fs::read_to_string(temp_dir.path().join("child-task.md")).unwrap();
        assert!(child_content.contains("- renamed-parent"));
        assert!(!child_content.contains("- parent-task"));
    }
}
