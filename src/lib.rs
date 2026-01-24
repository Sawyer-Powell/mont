pub mod commands;
pub mod context;
pub mod error_fmt;
pub mod jj;
pub mod multieditor;
pub mod render;

// Re-export commonly used types from context module for convenience
pub use context::{
    parse, GlobalConfig, GraphReadError, LoadError, MontContext, Op, ParseError, SettingsError,
    Status, Task, TaskGraph, TaskType, Transaction, TransactionError, ValidationError,
    GateItem, GateStatus,
};

// Re-export graph functions for binary
pub use context::graph::{available_tasks, form_graph};

// Re-export validation function for binary
pub use context::validations::validate_view;

use std::env;
use std::path::Path;
use std::process::Command;

use thiserror::Error;

/// Error type for editor resolution failures.
#[derive(Debug, Error)]
pub enum EditorError {
    #[error("no editor found: {0}")]
    NotFound(String),
}

/// Resolves the user's preferred text editor and returns a Command ready to execute.
///
/// Resolution order:
/// 1. If `editor` is provided, use that directly
/// 2. Check `$EDITOR` environment variable
/// 3. Fall back to OS-specific default (nano on macOS/Linux, notepad on Windows)
///
/// The returned Command has the file path already added as an argument.
pub fn resolve_editor(editor: Option<&str>, file: &Path) -> Result<Command, EditorError> {
    let editor_name = match editor {
        Some(e) => e.to_string(),
        None => env::var("EDITOR").unwrap_or_else(|_| default_editor().to_string()),
    };

    if editor_name.is_empty() {
        return Err(EditorError::NotFound(
            "editor name is empty; set $EDITOR or pass an editor explicitly".to_string(),
        ));
    }

    let mut cmd = Command::new(&editor_name);
    cmd.arg(file);
    Ok(cmd)
}

/// Returns the default editor for the current OS.
fn default_editor() -> &'static str {
    if cfg!(windows) {
        "notepad"
    } else {
        "nano"
    }
}
