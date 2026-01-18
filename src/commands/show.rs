//! Show command - displays details for a single task.

use owo_colors::OwoColorize;

use super::shared::{make_temp_file, update_via_editor, UpdateResult};
use crate::error_fmt::AppError;
use crate::render;
use crate::{MontContext, Task, TaskGraph, TaskType};

/// Show details for a single task.
pub fn show(
    ctx: &MontContext,
    id: &str,
    short: bool,
    editor: Option<Option<String>>,
) -> Result<(), AppError> {
    let task = ctx
        .graph()
        .get(id)
        .ok_or_else(|| AppError::TaskNotFound {
            task_id: id.to_string(),
            tasks_dir: ctx.tasks_dir().display().to_string(),
        })?
        .clone();

    // If editor flag is set, use edit workflow
    if let Some(editor_opt) = editor {
        let editor_name = editor_opt.as_deref();
        return edit_with_editor(ctx, id, &task, editor_name);
    }

    // Build a TaskGraph for rendering validators
    let graph = ctx.graph();
    print_task_details(&task, &graph, short);

    Ok(())
}

fn print_task_details(task: &Task, graph: &TaskGraph, short: bool) {
    let is_in_progress = task.is_in_progress();
    let is_jot = task.is_jot();
    let is_gate = task.is_gate();
    let is_complete = task.is_complete();

    const LABEL_WIDTH: usize = 14;

    // Task ID
    let id_display = if is_complete {
        task.id.bright_black().bold().to_string()
    } else if is_gate {
        task.id.purple().bold().to_string()
    } else if is_jot || is_in_progress {
        task.id.yellow().bold().to_string()
    } else {
        task.id.bright_green().bold().to_string()
    };
    println!("{:LABEL_WIDTH$} {}", "Id".bold(), id_display);

    // Title
    if let Some(ref title) = task.title {
        let title_display = if is_complete {
            title.bright_black().to_string()
        } else if is_gate {
            title.purple().to_string()
        } else if is_jot || is_in_progress {
            title.yellow().to_string()
        } else {
            title.bright_green().to_string()
        };
        println!("{:LABEL_WIDTH$} {}", "Title".bold(), title_display);
    }

    // Status
    let status_value = if is_complete {
        "complete".bright_black().to_string()
    } else if is_in_progress {
        "in progress".yellow().to_string()
    } else {
        "incomplete".white().to_string()
    };
    println!("{:LABEL_WIDTH$} {}", "Status".bold(), status_value);

    // Type
    let type_value = match task.task_type {
        TaskType::Task => "[task]".bright_green().to_string(),
        TaskType::Jot => "[jot]".yellow().to_string(),
        TaskType::Gate => "[gate]".purple().to_string(),
    };
    println!("{:LABEL_WIDTH$} {}", "Type".bold(), type_value);

    // Before
    if !task.before.is_empty() {
        println!(
            "{:LABEL_WIDTH$} {}",
            "Before".bold(),
            task.before.join(", ").cyan()
        );
    }

    // After
    if !task.after.is_empty() {
        println!(
            "{:LABEL_WIDTH$} {}",
            "After".bold(),
            task.after.join(", ").cyan()
        );
    }

    // Validations
    if !task.validations.is_empty() {
        for (i, val_item) in task.validations.iter().enumerate() {
            let label = if i == 0 { "Validations" } else { "" };
            if let Some(val_task) = graph.get(&val_item.id) {
                let marker = render::task_marker(val_task, graph);
                let line = render::format_task_line_short(val_task, graph);
                println!("{:LABEL_WIDTH$} {} {}", label.bold(), marker, line);
            } else {
                // Validator not found in graph, show ID only
                println!(
                    "{:LABEL_WIDTH$} {} {}",
                    label.bold(),
                    "?".bright_black(),
                    val_item.id.bright_black()
                );
            }
        }
    }

    // Description (unless short mode)
    if !short && !task.description.is_empty() {
        println!();
        let mut skin = termimad::MadSkin::default();
        skin.headers[0].align = termimad::Alignment::Left;
        skin.headers[1].align = termimad::Alignment::Left;
        skin.headers[2].align = termimad::Alignment::Left;
        skin.headers[3].align = termimad::Alignment::Left;
        skin.headers[4].align = termimad::Alignment::Left;
        skin.headers[5].align = termimad::Alignment::Left;
        skin.print_text(&task.description);
    }
}

fn edit_with_editor(
    ctx: &MontContext,
    original_id: &str,
    task: &Task,
    editor_name: Option<&str>,
) -> Result<(), AppError> {
    let suffix = format!("show_{}", original_id);
    let path = make_temp_file(&suffix, std::slice::from_ref(task), None)?;

    match update_via_editor(ctx, original_id, &path, editor_name)? {
        UpdateResult::Updated { new_id, id_changed } => {
            if id_changed {
                println!(
                    "renamed: {} -> {}",
                    original_id.bright_yellow(),
                    new_id.bright_green()
                );
            } else {
                let file_path = ctx.tasks_dir().join(format!("{}.md", new_id));
                println!("updated: {}", file_path.display().to_string().bright_green());
            }
        }
        UpdateResult::Aborted => {
            println!("No task defined, aborting edit.");
        }
    }

    Ok(())
}
