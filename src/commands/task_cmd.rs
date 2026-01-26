//! Unified task command - creates, edits, and deletes tasks in one editor session.

use std::io::{self, Read, Write};
use std::path::PathBuf;

use owo_colors::OwoColorize;

use super::shared::{
    build_multiedit_comment, find_most_recent_temp_file, make_temp_file, MultiEditMode,
    parse_multi_task_content, remove_temp_file, resolve_ids, TaskFilter,
};
use crate::error_fmt::AppError;
use crate::jj;
use crate::multieditor::{apply_diff, compute_diff, fill_empty_ids, ApplyResult};
use crate::{resolve_editor, MontContext, Task, TaskType};

/// Arguments for the unified task command.
pub struct TaskArgs {
    /// Task IDs to edit (if any). If empty, opens empty multieditor.
    pub ids: Vec<String>,
    /// Task type template (task, jot, gate)
    pub task_type: Option<TaskType>,
    /// Resume editing the most recent temp file
    pub resume: bool,
    /// Resume editing a specific temp file
    pub resume_path: Option<PathBuf>,
    /// Content to use directly (for testing/internal use)
    pub content: Option<String>,
    /// Read content from stdin (LLM-friendly)
    pub stdin: bool,
    /// YAML patch to merge into task (requires single ID)
    pub patch: Option<String>,
    /// Append text to task description (requires single ID)
    pub append: Option<String>,
    /// Editor name override
    pub editor: Option<String>,
    /// Include full subgraph of each ID
    pub group: bool,
}

/// Read all content from stdin.
fn read_stdin() -> Result<String, AppError> {
    let mut content = String::new();
    io::stdin().read_to_string(&mut content).map_err(|e| AppError::Io {
        context: "failed to read from stdin".to_string(),
        source: e,
    })?;
    Ok(content)
}

/// Parse original task IDs from a temp file's comment header.
///
/// Looks for the pattern "# ORIGINAL_IDS: id1,id2,id3" in the file.
/// These are the tasks that existed when the temp file was created,
/// used to compute proper diffs on resume.
fn parse_original_ids_from_file(path: &PathBuf) -> Vec<String> {
    let Ok(content) = std::fs::read_to_string(path) else {
        return vec![];
    };
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("# ORIGINAL_IDS: ") {
            return rest.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
        }
    }
    vec![]
}

/// Resume editing from a temp file. Shared by task, jot, and distill commands.
///
/// - `suffix`: The temp file suffix (e.g., "task", "jot", "distill")
/// - `resume_path`: Explicit path to resume from, or None to find most recent
/// - `editor_name`: Optional editor override
/// - `command_name`: Command name for resume messages
fn resume_from_temp_file(
    ctx: &MontContext,
    suffix: &str,
    resume_path: Option<PathBuf>,
    editor_name: Option<&str>,
    command_name: &str,
) -> Result<(), AppError> {
    let temp_path = if let Some(path) = resume_path {
        path
    } else {
        find_most_recent_temp_file(suffix)
            .ok_or_else(|| AppError::TempFileNotFound(format!("No recent {} temp file found", suffix)))?
    };

    if !temp_path.exists() {
        return Err(AppError::TempFileNotFound(temp_path.display().to_string()));
    }

    // Try to recover original task IDs from the temp file header
    let original_ids = parse_original_ids_from_file(&temp_path);
    let graph_tasks: Vec<Task> = {
        let graph = ctx.graph();
        original_ids
            .iter()
            .filter_map(|id| graph.get(id).cloned())
            .collect()
    };

    run_editor_workflow(ctx, &temp_path, &[], &graph_tasks, editor_name, command_name)
}

/// Run the unified task command.
pub fn task(ctx: &MontContext, args: TaskArgs) -> Result<(), AppError> {
    // Resume mode: --resume or --resume-path
    if args.resume || args.resume_path.is_some() {
        return resume_mode(ctx, args);
    }

    // Content mode (internal/testing)
    if let Some(content) = args.content {
        return content_mode(ctx, &content, &args.ids);
    }

    // Stdin mode: --stdin (read content from stdin)
    if args.stdin {
        let content = read_stdin()?;
        return content_mode(ctx, &content, &args.ids);
    }

    // Resolve `?` placeholders in IDs via interactive picker
    let mut ids = if args.ids.iter().any(|id| id == "?") {
        resolve_ids(&ctx.graph(), &args.ids, TaskFilter::Active)?
    } else {
        args.ids.clone()
    };

    // Group mode: expand each ID to include its full subgraph
    if args.group && !ids.is_empty() {
        let seeds: Vec<&str> = ids.iter().map(|s| s.as_str()).collect();
        let subgraph_ids: std::collections::HashSet<String> =
            ctx.graph().subgraph(&seeds).into_iter().collect();

        // Get topological order and filter to just the subgraph
        ids = ctx
            .graph()
            .topological_order()
            .into_iter()
            .filter(|id| subgraph_ids.contains(*id))
            .map(|s| s.to_string())
            .collect();
    }

    // Patch mode: --patch (JSON merge, single ID)
    if let Some(patch) = args.patch {
        return patch_mode(ctx, &ids, &patch);
    }

    // Append mode: --append (add to description, single ID)
    if let Some(text) = args.append {
        return append_mode(ctx, &ids, &text);
    }

    // Editor mode: open editor for creating/editing tasks
    if ids.is_empty() {
        // Empty multieditor - create new tasks
        create_mode(ctx, args.task_type, args.editor.as_deref())
    } else {
        // Edit specific tasks
        edit_mode(ctx, &ids, args.editor.as_deref())
    }
}

/// Resume editing from a temp file.
fn resume_mode(ctx: &MontContext, args: TaskArgs) -> Result<(), AppError> {
    resume_from_temp_file(ctx, "task", args.resume_path, args.editor.as_deref(), "task")
}

/// Create tasks from direct content (skip editor).
fn content_mode(ctx: &MontContext, content: &str, ids: &[String]) -> Result<(), AppError> {
    let temp_path = std::env::temp_dir().join("content_mode.md");
    std::fs::write(&temp_path, content)
        .map_err(|e| AppError::Io {
            context: "failed to write content".to_string(),
            source: e,
        })?;

    let edited = parse_multi_task_content(content, &temp_path)?;

    // Get original tasks if editing
    let original: Vec<Task> = ids.iter()
        .filter_map(|id| ctx.graph().get(id).cloned())
        .collect();

    // Compute and apply diff
    let diff = compute_diff(&original, &edited);

    if diff.is_empty() {
        println!("No changes.");
        return Ok(());
    }

    // Apply changes directly (no confirmation in content mode)
    let result = apply_diff(ctx, diff)?;
    print_result(ctx, &result);
    auto_commit(ctx, &result);

    Ok(())
}

/// YAML patch struct for merging into tasks.
#[derive(serde::Deserialize, Default)]
struct TaskPatch {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    before: Option<Vec<String>>,
    #[serde(default)]
    after: Option<Vec<String>>,
    #[serde(default)]
    gates: Option<Vec<String>>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    r#type: Option<String>,
}

/// Apply a YAML patch to a single task.
fn patch_mode(ctx: &MontContext, ids: &[String], patch_yaml: &str) -> Result<(), AppError> {
    // Require exactly one ID
    if ids.len() != 1 {
        return Err(AppError::InvalidArgs(
            "--patch requires exactly one task ID".to_string(),
        ));
    }
    let original_id = &ids[0];

    // Parse the patch
    let patch: TaskPatch = serde_yaml::from_str(patch_yaml)
        .map_err(|e| AppError::InvalidArgs(format!("invalid YAML patch: {}", e)))?;

    // Get the task
    let graph = ctx.graph();
    let mut task = graph.get(original_id)
        .ok_or_else(|| AppError::TaskNotFound {
            task_id: original_id.clone(),
            tasks_dir: ctx.tasks_dir().display().to_string(),
        })?
        .clone();
    drop(graph);

    // Check if ID is being changed
    let new_id = patch.id.clone();
    let id_changed = new_id.as_ref().is_some_and(|new| new != original_id);

    // Confirm rename operation
    if let Some(new) = new_id.as_ref().filter(|_| id_changed) {
        print!(
            "Rename {} -> {}? [y/N] ",
            original_id.bright_yellow(),
            new.bright_green()
        );
        io::stdout().flush().ok();

        let mut input = String::new();
        io::stdin().read_line(&mut input).ok();
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Rename cancelled.");
            return Ok(());
        }
    }

    // Apply patch fields
    if let Some(new) = new_id {
        task.id = new;
    }
    if let Some(title) = patch.title {
        task.title = Some(title);
    }
    if let Some(desc) = patch.description {
        task.description = desc;
    }
    if let Some(before) = patch.before {
        task.before = before;
    }
    if let Some(after) = patch.after {
        task.after = after;
    }
    if let Some(gates) = patch.gates {
        use crate::{GateItem, GateStatus};
        task.gates = gates.into_iter().map(|g| GateItem {
            id: g,
            status: GateStatus::Pending,
        }).collect();
    }
    if let Some(status) = patch.status {
        use crate::Status;
        task.status = match status.to_lowercase().as_str() {
            "inprogress" | "in-progress" | "in_progress" => Some(Status::InProgress),
            "stopped" => Some(Status::Stopped),
            "complete" | "done" => Some(Status::Complete),
            "" | "pending" | "ready" => None,
            _ => return Err(AppError::InvalidArgs(format!("invalid status: {}", status))),
        };
    }
    if let Some(task_type) = patch.r#type {
        task.task_type = match task_type.to_lowercase().as_str() {
            "task" => TaskType::Task,
            "jot" => TaskType::Jot,
            "gate" => TaskType::Gate,
            _ => return Err(AppError::InvalidArgs(format!("invalid type: {}", task_type))),
        };
    }

    // Update the task (this handles reference rewriting if ID changed)
    ctx.update(original_id, task.clone())?;

    // Build result for auto-commit
    let result = ApplyResult {
        created: vec![],
        updated: vec![(original_id.clone(), task.id.clone(), id_changed)],
        deleted: vec![],
    };

    if id_changed {
        println!(
            "renamed: {} -> {}",
            original_id.bright_yellow(),
            task.id.bright_green()
        );
    } else {
        let file_path = ctx.tasks_dir().join(format!("{}.md", original_id));
        println!("updated: {}", file_path.display().to_string().bright_blue());
    }

    auto_commit(ctx, &result);

    Ok(())
}

/// Append text to a task's description.
fn append_mode(ctx: &MontContext, ids: &[String], text: &str) -> Result<(), AppError> {
    // Require exactly one ID
    if ids.len() != 1 {
        return Err(AppError::InvalidArgs(
            "--append requires exactly one task ID".to_string(),
        ));
    }
    let id = &ids[0];

    // Get the task
    let graph = ctx.graph();
    let mut task = graph.get(id)
        .ok_or_else(|| AppError::TaskNotFound {
            task_id: id.clone(),
            tasks_dir: ctx.tasks_dir().display().to_string(),
        })?
        .clone();
    drop(graph);

    // Append to description
    if !task.description.is_empty() && !task.description.ends_with('\n') {
        task.description.push('\n');
    }
    if !task.description.is_empty() {
        task.description.push('\n');
    }
    task.description.push_str(text);

    // Update the task
    ctx.update(id, task)?;

    let file_path = ctx.tasks_dir().join(format!("{}.md", id));
    println!("updated: {}", file_path.display().to_string().bright_blue());

    // Auto-commit
    let result = ApplyResult {
        created: vec![],
        updated: vec![(id.clone(), id.clone(), false)],
        deleted: vec![],
    };
    auto_commit(ctx, &result);

    Ok(())
}

/// Create new tasks in empty multieditor.
fn create_mode(
    ctx: &MontContext,
    task_type: Option<TaskType>,
    editor_name: Option<&str>,
) -> Result<(), AppError> {
    let mode = match task_type {
        Some(tt) => MultiEditMode::CreateWithType(tt),
        None => MultiEditMode::Create,
    };

    let comment = build_multiedit_comment(mode);

    // Create starter task based on type
    let starter = match task_type {
        Some(TaskType::Gate) => {
            Task {
                id: "new-gate".to_string(),
                new_id: None,
                title: Some("New Gate".to_string()),
                description: "Description here.".to_string(),
                before: vec![],
                after: vec![],
                gates: vec![],
                task_type: TaskType::Gate,
                status: None,
                deleted: false,
            }
        }
        Some(TaskType::Jot) => {
            Task {
                id: "new-jot".to_string(),
                new_id: None,
                title: Some("New Jot".to_string()),
                description: "Quick idea here.".to_string(),
                before: vec![],
                after: vec![],
                gates: vec![],
                task_type: TaskType::Jot,
                status: None,
                deleted: false,
            }
        }
        _ => {
            Task {
                id: "new-task".to_string(),
                new_id: None,
                title: Some("New Task".to_string()),
                description: "Description here.".to_string(),
                before: vec![],
                after: vec![],
                gates: vec![],
                task_type: TaskType::Task,
                status: None,
                deleted: false,
            }
        }
    };

    let temp_path = make_temp_file("task", std::slice::from_ref(&starter), Some(&comment))?;

    // template = [starter] (for no-changes check), graph = [] (all inserts)
    run_editor_workflow(ctx, &temp_path, std::slice::from_ref(&starter), &[], editor_name, "task")
}

/// Edit existing tasks.
fn edit_mode(
    ctx: &MontContext,
    ids: &[String],
    editor_name: Option<&str>,
) -> Result<(), AppError> {
    // Collect tasks to edit
    let mut original_tasks = Vec::new();
    for id in ids {
        let task = ctx
            .graph()
            .get(id)
            .ok_or_else(|| AppError::TaskNotFound {
                task_id: id.clone(),
                tasks_dir: ctx.tasks_dir().display().to_string(),
            })?
            .clone();
        original_tasks.push(task);
    }

    // Build comment with ORIGINAL_IDS header so resume can recover context
    let base_comment = build_multiedit_comment(MultiEditMode::Edit);
    let original_ids_line = format!("ORIGINAL_IDS: {}", ids.join(","));
    let comment = format!("{}\n{}", original_ids_line, base_comment);
    let temp_path = make_temp_file("task", &original_tasks, Some(&comment))?;

    // Both template and graph are the same for edit mode
    run_editor_workflow(ctx, &temp_path, &original_tasks, &original_tasks, editor_name, "task")
}

/// Core editor workflow: open editor, parse result, compute diff, confirm, apply.
///
/// - `template_tasks`: What was written to temp file (for "no changes" detection)
/// - `graph_tasks`: Tasks that exist in graph (for computing actual diff)
/// - `command_name`: Command name for resume messages (e.g., "task", "jot", "distill")
fn run_editor_workflow(
    ctx: &MontContext,
    temp_path: &PathBuf,
    template_tasks: &[Task],
    graph_tasks: &[Task],
    editor_name: Option<&str>,
    command_name: &str,
) -> Result<(), AppError> {
    let temp_path_str = temp_path.display().to_string();

    // Open editor
    let mut cmd = resolve_editor(editor_name, temp_path)?;
    cmd.status().map_err(|e| AppError::Io {
        context: "failed to run editor".to_string(),
        source: e,
    })?;

    // Parse edited content
    let edited = match parse_multi_task_content(
        &std::fs::read_to_string(temp_path).map_err(|e| AppError::Io {
            context: "failed to read temp file".to_string(),
            source: e,
        })?,
        temp_path,
    ) {
        Ok(tasks) => tasks,
        Err(e) => {
            return Err(AppError::TempValidationFailed {
                error: Box::new(e),
                temp_path: temp_path_str,
                editor_name: editor_name.map(String::from),
                command_name: command_name.to_string(),
            });
        }
    };

    // Handle empty result (user deleted everything)
    if edited.is_empty() && template_tasks.is_empty() {
        println!("No tasks defined, aborting.");
        remove_temp_file(temp_path)?;
        return Ok(());
    }

    // Check for "no changes" - compare against what was in temp file
    let template_diff = compute_diff(template_tasks, &edited);
    if template_diff.is_empty() {
        println!("No changes.");
        remove_temp_file(temp_path)?;
        return Ok(());
    }

    // For actual diff, only include tasks that exist in the graph
    // Templates (like "new-task", "new-jot") become inserts, not updates
    let mut diff = {
        let graph = ctx.graph();
        let real_originals: Vec<Task> = graph_tasks
            .iter()
            .filter(|t| graph.contains(&t.id))
            .cloned()
            .collect();
        compute_diff(&real_originals, &edited)
        // graph (read lock) is dropped here before apply_diff needs write lock
    };

    // Fill in empty IDs before showing summary so user sees actual IDs
    fill_empty_ids(ctx, &mut diff)?;

    // Show summary and confirm
    print_diff_summary(&diff, graph_tasks, &edited);

    if !confirm_changes()? {
        println!(
            "Changes not applied. Resume with: {} or {}",
            format!("mont {} -r", command_name).cyan(),
            format!("mont {} --resume-path {}", command_name, temp_path_str).dimmed()
        );
        return Ok(());
    }

    // Apply changes
    match apply_diff(ctx, diff) {
        Ok(result) => {
            print_result(ctx, &result);
            auto_commit(ctx, &result);
            remove_temp_file(temp_path)?;
            Ok(())
        }
        Err(e) => {
            Err(AppError::TempValidationFailed {
                error: Box::new(e),
                temp_path: temp_path_str,
                editor_name: editor_name.map(String::from),
                command_name: command_name.to_string(),
            })
        }
    }
}

/// Print a summary of changes to be made.
fn print_diff_summary(
    diff: &crate::multieditor::MultiEditDiff,
    original: &[Task],
    _edited: &[Task],
) {
    println!("\n{}", "Changes detected:".bold());

    // Created tasks
    if !diff.inserts.is_empty() {
        let ids: Vec<_> = diff.inserts.iter().map(|t| t.id.as_str()).collect();
        println!("  {}: {}", "created".bright_green(), ids.join(", "));
    }

    // Updated tasks
    let mut renamed = Vec::new();
    let mut updated = Vec::new();
    for (original_id, new_task) in &diff.updates {
        if original_id != &new_task.id {
            renamed.push(format!("{} -> {}", original_id, new_task.id));
        } else {
            updated.push(original_id.as_str());
        }
    }
    if !updated.is_empty() {
        println!("  {}: {}", "updated".bright_blue(), updated.join(", "));
    }
    if !renamed.is_empty() {
        println!("  {}: {}", "renamed".bright_yellow(), renamed.join(", "));
    }

    // Deleted tasks
    if !diff.deletes.is_empty() {
        // Get titles for deleted tasks
        let deleted_info: Vec<String> = diff.deletes.iter().map(|id| {
            if let Some(task) = original.iter().find(|t| &t.id == id) {
                if let Some(title) = &task.title {
                    format!("{} ({})", id, title)
                } else {
                    id.clone()
                }
            } else {
                id.clone()
            }
        }).collect();
        println!("  {}: {}", "deleted".bright_red(), deleted_info.join(", "));
    }

    // Warning when creates == deletes and types match (potential forgotten rename)
    // Skip if types differ (e.g., distilling jot to task is intentional)
    if diff.inserts.len() == 1 && diff.deletes.len() == 1 {
        let deleted_type = original.iter()
            .find(|t| t.id == diff.deletes[0])
            .map(|t| t.task_type);
        let inserted_type = Some(diff.inserts[0].task_type);

        if deleted_type == inserted_type {
            println!(
                "  {}: Did you mean to rename? Use {} field instead of changing id.",
                "warning".yellow().bold(),
                "new_id".cyan()
            );
        }
    }

    println!();
}

/// Prompt user to confirm changes.
fn confirm_changes() -> Result<bool, AppError> {
    print!("Apply these changes? [y/N] ");
    io::stdout().flush().map_err(|e| AppError::Io {
        context: "failed to flush stdout".to_string(),
        source: e,
    })?;

    let mut input = String::new();
    io::stdin().read_line(&mut input).map_err(|e| AppError::Io {
        context: "failed to read input".to_string(),
        source: e,
    })?;

    let input = input.trim().to_lowercase();
    Ok(input == "y" || input == "yes")
}

/// Print the result of applying changes.
fn print_result(ctx: &MontContext, result: &ApplyResult) {
    for id in &result.created {
        let file_path = ctx.tasks_dir().join(format!("{}.md", id));
        println!("created: {}", file_path.display().to_string().bright_green());
    }

    for (original_id, new_id, id_changed) in &result.updated {
        if *id_changed {
            println!(
                "renamed: {} -> {}",
                original_id.bright_yellow(),
                new_id.bright_green()
            );
        } else {
            let file_path = ctx.tasks_dir().join(format!("{}.md", new_id));
            println!("updated: {}", file_path.display().to_string().bright_blue());
        }
    }

    for id in &result.deleted {
        println!("deleted: {}", id.bright_red());
    }
}

/// Auto-commit changes if jj is enabled.
fn auto_commit(ctx: &MontContext, result: &ApplyResult) {
    // Skip if jj is disabled
    if !ctx.config().jj.enabled {
        return;
    }

    // Skip if no changes
    if result.created.is_empty() && result.updated.is_empty() && result.deleted.is_empty() {
        return;
    }

    // Build commit message
    let message = build_commit_message(result);

    // Run jj commit (only commit task files)
    match jj::commit(&message, &[ctx.tasks_dir()]) {
        Ok(result) if result.committed => {
            println!("{}", "committed".bright_green());
        }
        Ok(_) => {} // Nothing to commit (e.g., .tasks is gitignored)
        Err(e) => {
            eprintln!(
                "{}: failed to auto-commit: {}",
                "warning".yellow(),
                e
            );
        }
    }
}

/// Build a commit message from apply result.
fn build_commit_message(result: &ApplyResult) -> String {
    let mut parts = Vec::new();

    if !result.created.is_empty() {
        if result.created.len() == 1 {
            parts.push(format!("Create task {}", result.created[0]));
        } else {
            parts.push(format!("Create {} tasks", result.created.len()));
        }
    }

    if !result.updated.is_empty() {
        let renamed: Vec<_> = result.updated.iter()
            .filter(|(_, _, changed)| *changed)
            .collect();
        let updated: Vec<_> = result.updated.iter()
            .filter(|(_, _, changed)| !*changed)
            .collect();

        if !updated.is_empty() {
            if updated.len() == 1 {
                parts.push(format!("Update task {}", updated[0].1));
            } else {
                parts.push(format!("Update {} tasks", updated.len()));
            }
        }

        if !renamed.is_empty() {
            for (old, new, _) in renamed {
                parts.push(format!("Rename {} to {}", old, new));
            }
        }
    }

    if !result.deleted.is_empty() {
        if result.deleted.len() == 1 {
            parts.push(format!("Delete task {}", result.deleted[0]));
        } else {
            parts.push(format!("Delete {} tasks", result.deleted.len()));
        }
    }

    parts.join(", ")
}

/// Arguments for the jot command.
pub struct JotArgs {
    /// Optional title for the jot
    pub title: Option<String>,
    /// Skip editor and confirmation, create jot immediately
    pub quick: bool,
    /// Resume editing the most recent temp file
    pub resume: bool,
    /// Resume editing a specific temp file
    pub resume_path: Option<PathBuf>,
    /// Editor name override
    pub editor: Option<String>,
}

/// Quick jot command - creates a jot with optional pre-filled title.
pub fn jot(ctx: &MontContext, args: JotArgs) -> Result<(), AppError> {
    // Resume mode
    if args.resume || args.resume_path.is_some() {
        return resume_from_temp_file(ctx, "jot", args.resume_path, args.editor.as_deref(), "jot");
    }

    // Generate a unique ID for the jot
    let jot_id = ctx.generate_id(&ctx.graph())?;
    let jot_title = args.title.clone().unwrap_or_else(|| jot_id.clone());

    let jot = Task {
        id: jot_id,
        new_id: None,
        title: Some(jot_title),
        description: String::new(),
        before: vec![],
        after: vec![],
        gates: vec![],
        task_type: TaskType::Jot,
        status: None,
        deleted: false,
    };

    // Quick mode: skip editor and confirmation, create jot immediately
    if args.quick {
        let id = jot.id.clone();
        ctx.insert(jot)?;

        let file_path = ctx.tasks_dir().join(format!("{}.md", &id));
        println!("created: {}", file_path.display().to_string().bright_green());

        // Auto-commit
        if ctx.config().jj.enabled {
            let message = format!("Create jot {}", id);
            match crate::jj::commit(&message, &[ctx.tasks_dir()]) {
                Ok(result) if result.committed => println!("{}", "committed".bright_green()),
                Ok(_) => {} // Nothing to commit (e.g., .tasks is gitignored)
                Err(e) => eprintln!("{}: failed to auto-commit: {}", "warning".yellow(), e),
            }
        }

        return Ok(());
    }

    let comment = build_multiedit_comment(MultiEditMode::CreateWithType(TaskType::Jot));
    let temp_path = make_temp_file("jot", std::slice::from_ref(&jot), Some(&comment))?;

    // template = [] so exiting without changes still creates the jot
    // graph = [] since jot doesn't exist yet (all inserts)
    run_editor_workflow(ctx, &temp_path, &[], &[], args.editor.as_deref(), "jot")
}

/// Arguments for the distill command.
pub struct DistillArgs {
    /// Jot ID to distill
    pub jot_id: String,
    /// Resume editing the most recent temp file
    pub resume: bool,
    /// Resume editing a specific temp file
    pub resume_path: Option<PathBuf>,
    /// Read task definitions from stdin (LLM-friendly, skips editor)
    pub stdin: bool,
    /// Editor name override
    pub editor: Option<String>,
}


/// Distill a jot into concrete tasks.
///
/// Opens editor with the jot commented out and space for new tasks.
/// When saved, the jot is deleted and new tasks are created.
pub fn distill(ctx: &MontContext, args: DistillArgs) -> Result<(), AppError> {
    // Resume mode - recover original IDs from temp file header
    if args.resume || args.resume_path.is_some() {
        let temp_path = if let Some(path) = args.resume_path {
            path
        } else {
            find_most_recent_temp_file("distill")
                .ok_or_else(|| AppError::TempFileNotFound("No recent distill temp file found".to_string()))?
        };

        if !temp_path.exists() {
            return Err(AppError::TempFileNotFound(temp_path.display().to_string()));
        }

        // Recover original IDs (the jot being distilled) from the temp file
        let original_ids = parse_original_ids_from_file(&temp_path);
        let graph_tasks: Vec<Task> = {
            let graph = ctx.graph();
            original_ids
                .iter()
                .filter_map(|id| graph.get(id).cloned())
                .collect()
        };

        return run_editor_workflow(ctx, &temp_path, &[], &graph_tasks, args.editor.as_deref(), "distill");
    }

    // Stdin mode: read task definitions from stdin (LLM-friendly)
    if args.stdin {
        let content = read_stdin()?;
        return distill_stdin_mode(ctx, &args.jot_id, &content);
    }

    // Load the jot
    let jot = ctx
        .graph()
        .get(&args.jot_id)
        .ok_or_else(|| AppError::TaskNotFound {
            task_id: args.jot_id.clone(),
            tasks_dir: ctx.tasks_dir().display().to_string(),
        })?
        .clone();

    // Verify it's actually a jot
    if !jot.is_jot() {
        return Err(AppError::NotAJot(args.jot_id.clone()));
    }

    // Build comment with the original jot content for reference (no # prefix - make_temp_file adds it)
    let jot_markdown = jot.to_markdown();
    let jot_lines: String = jot_markdown
        .lines()
        .map(|line| format!("  {}", line))
        .collect::<Vec<_>>()
        .join("\n");

    let comment = format!(
        r#"ORIGINAL_IDS: {}
Original jot (for reference - will be deleted):

{}

Create replacement tasks below. The jot above will be deleted
when you save and confirm."#,
        args.jot_id, jot_lines
    );

    // Create a starter task template
    let starter = Task {
        id: "new-task".to_string(),
        new_id: None,
        title: Some("New Task".to_string()),
        description: "Description here.".to_string(),
        before: vec![],
        after: vec![],
        gates: vec![],
        task_type: TaskType::Task,
        status: None,
        deleted: false,
    };

    let temp_path = make_temp_file("distill", std::slice::from_ref(&starter), Some(&comment))?;

    // template = [starter] (for no-changes check)
    // graph = [jot] (jot exists in graph and will be deleted when user makes changes)
    run_editor_workflow(ctx, &temp_path, std::slice::from_ref(&starter), std::slice::from_ref(&jot), args.editor.as_deref(), "distill")
}

/// Distill a jot using stdin content (LLM-friendly, skips editor).
///
/// This parses task definitions from stdin, validates them, deletes the jot,
/// and creates the new tasks - all without user interaction.
fn distill_stdin_mode(ctx: &MontContext, jot_id: &str, content: &str) -> Result<(), AppError> {
    use std::path::Path;

    // Verify the jot exists and is actually a jot
    let jot = ctx
        .graph()
        .get(jot_id)
        .ok_or_else(|| AppError::TaskNotFound {
            task_id: jot_id.to_string(),
            tasks_dir: ctx.tasks_dir().display().to_string(),
        })?
        .clone();

    if !jot.is_jot() {
        return Err(AppError::NotAJot(jot_id.to_string()));
    }

    // Parse task definitions from stdin (use fake path for error messages)
    let edited = parse_multi_task_content(content, Path::new("<stdin>"))?;

    if edited.is_empty() {
        return Err(AppError::CommandFailed(
            "No tasks defined in stdin. Provide task definitions in frontmatter format.".to_string(),
        ));
    }

    // Compute diff: original = [jot], edited = new tasks
    // This will result in: delete jot, insert all new tasks
    let diff = compute_diff(std::slice::from_ref(&jot), &edited);

    // Show what will happen
    println!("{}", "Changes:".bold());
    for id in &diff.deletes {
        println!("  {} {}", "deleted:".red(), id.bright_yellow());
    }
    for task in &diff.inserts {
        println!("  {} {}", "created:".green(), task.id.cyan());
    }

    // Apply the diff (validates and commits atomically)
    let result = apply_diff(ctx, diff)?;

    // Show results
    for id in &result.deleted {
        println!("  {}: {}", "deleted".red(), id.bright_yellow());
    }
    for (old_id, new_id, id_changed) in &result.updated {
        if *id_changed {
            println!("  {}: {} -> {}", "renamed".blue(), old_id.bright_yellow(), new_id.cyan());
        } else {
            println!("  {}: {}", "updated".yellow(), new_id.cyan());
        }
    }
    for id in &result.created {
        println!("  {}: {}", "created".green(), id.cyan());
    }

    if result.created.is_empty() && result.updated.is_empty() && result.deleted.is_empty() {
        println!("No changes detected.");
    }

    // Auto-commit if jj is enabled
    if ctx.config().jj.enabled {
        let message = format!("Distill jot {} into tasks", jot_id);
        match jj::commit(&message, &[ctx.tasks_dir()]) {
            Ok(result) if result.committed => println!("{}", "committed".bright_green()),
            Ok(_) => {} // Nothing to commit (e.g., .tasks is gitignored)
            Err(e) => eprintln!("{}: failed to auto-commit: {}", "warning".yellow(), e),
        }
    }

    Ok(())
}
