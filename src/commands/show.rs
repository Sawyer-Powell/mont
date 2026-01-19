//! Show command - displays details for a single task.

use owo_colors::OwoColorize;

use super::shared::{make_temp_file, update_via_editor, UpdateResult};
use crate::error_fmt::AppError;
use crate::render::{print_gates_section, TaskDisplayView};
use crate::{MontContext, Task, TaskType};

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

    // Print task details using shared helpers
    print_task_details(ctx, &task, short);

    Ok(())
}

fn print_task_details(ctx: &MontContext, task: &Task, short: bool) {
    let graph = ctx.graph();
    let config = ctx.config();
    let view = TaskDisplayView::from_task(task, &graph, &config.default_gates);

    const LABEL_WIDTH: usize = 14;

    // Task ID
    println!("{:LABEL_WIDTH$} {}", "Id".bold(), view.id_colored());

    // Title
    if task.title.is_some() {
        println!(
            "{:LABEL_WIDTH$} {}",
            "Title".bold(),
            view.title_colored(usize::MAX)
        );
    }

    // Status
    println!("{:LABEL_WIDTH$} {}", "Status".bold(), view.status_colored());

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

    // Gates section using shared helper
    let all_gate_ids = ctx.all_gate_ids(task);
    print_gates_section(task, &all_gate_ids, "", LABEL_WIDTH);

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
