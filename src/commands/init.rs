//! Init command - initialize mont in the current directory.

use std::io::Write;
use std::path::Path;
use std::process::Command;

use owo_colors::OwoColorize;

use crate::error_fmt::{AppError, IoResultExt};

/// Tracking preference for .tasks directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TrackingPreference {
    /// Include .tasks in source control (default)
    Tracked,
    /// Add .tasks to .gitignore (shared across clones)
    Gitignore,
    /// Add to .git/info/exclude (local only)
    GitExclude,
}

/// Current state of .tasks tracking configuration.
#[derive(Debug)]
struct TrackingState {
    /// Whether .tasks directory exists
    tasks_dir_exists: bool,
    /// Whether .tasks is in .gitignore
    in_gitignore: bool,
    /// Whether .tasks is in .git/info/exclude
    in_git_exclude: bool,
    /// Whether .tasks is in global git exclude
    in_global_exclude: bool,
    /// Path to global exclude file (if found)
    global_exclude_path: Option<String>,
    /// Whether we're in a git repository
    is_git_repo: bool,
}

impl TrackingState {
    /// Detect the current tracking state.
    fn detect() -> Self {
        let tasks_dir_exists = Path::new(".tasks").is_dir();
        let is_git_repo = Path::new(".git").is_dir();

        let in_gitignore = if Path::new(".gitignore").exists() {
            std::fs::read_to_string(".gitignore")
                .map(|content| contains_tasks_pattern(&content))
                .unwrap_or(false)
        } else {
            false
        };

        let in_git_exclude = if Path::new(".git/info/exclude").exists() {
            std::fs::read_to_string(".git/info/exclude")
                .map(|content| contains_tasks_pattern(&content))
                .unwrap_or(false)
        } else {
            false
        };

        let (in_global_exclude, global_exclude_path) = detect_global_exclude();

        Self {
            tasks_dir_exists,
            in_gitignore,
            in_git_exclude,
            in_global_exclude,
            global_exclude_path,
            is_git_repo,
        }
    }

    /// Get the current tracking preference based on detected state.
    fn current_preference(&self) -> TrackingPreference {
        if self.in_gitignore {
            TrackingPreference::Gitignore
        } else if self.in_git_exclude {
            TrackingPreference::GitExclude
        } else {
            TrackingPreference::Tracked
        }
    }

    /// Check if .tasks is currently tracked by git/jj.
    fn is_tracked_in_vcs(&self) -> bool {
        if !self.is_git_repo {
            return false;
        }

        // Check if .tasks is in git's index
        let output = Command::new("git")
            .args(["ls-files", ".tasks"])
            .output();

        match output {
            Ok(out) => !out.stdout.is_empty(),
            Err(_) => false,
        }
    }
}

/// Check if content contains a pattern that ignores .tasks.
fn contains_tasks_pattern(content: &str) -> bool {
    content.lines().any(|line| {
        let trimmed = line.trim();
        trimmed == ".tasks" || trimmed == ".tasks/" || trimmed == "/.tasks" || trimmed == "/.tasks/"
    })
}

/// Detect if .tasks is in global git exclude.
fn detect_global_exclude() -> (bool, Option<String>) {
    // Get global excludes file path
    let output = Command::new("git")
        .args(["config", "--global", "core.excludesFile"])
        .output();

    let global_path = match output {
        Ok(out) if out.status.success() => {
            let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if path.is_empty() {
                None
            } else {
                // Expand ~ to home directory
                Some(shellexpand::tilde(&path).to_string())
            }
        }
        _ => {
            // Default locations
            if let Some(home) = std::env::var_os("HOME") {
                let default_path = format!("{}/.config/git/ignore", home.to_string_lossy());
                if Path::new(&default_path).exists() {
                    Some(default_path)
                } else {
                    None
                }
            } else {
                None
            }
        }
    };

    if let Some(ref path) = global_path
        && let Ok(content) = std::fs::read_to_string(path)
        && contains_tasks_pattern(&content)
    {
        return (true, global_path);
    }

    (false, global_path)
}

/// Display the current state to the user.
fn display_state(state: &TrackingState) {
    println!("{}", "Current configuration:".bold());
    println!();

    // .tasks directory
    if state.tasks_dir_exists {
        println!(
            "  {} .tasks directory exists",
            "✓".green()
        );
    } else {
        println!(
            "  {} .tasks directory does not exist (will be created)",
            "○".dimmed()
        );
    }

    // Git repo check
    if !state.is_git_repo {
        println!(
            "  {} Not a git repository (git-related options will be skipped)",
            "!".yellow()
        );
        return;
    }

    // Tracking status
    let current = state.current_preference();
    match current {
        TrackingPreference::Tracked => {
            if state.is_tracked_in_vcs() {
                println!(
                    "  {} .tasks is tracked in version control",
                    "✓".green()
                );
            } else {
                println!(
                    "  {} .tasks is not excluded (will be tracked)",
                    "○".dimmed()
                );
            }
        }
        TrackingPreference::Gitignore => {
            println!(
                "  {} .tasks is in .gitignore (shared exclusion)",
                "✓".green()
            );
        }
        TrackingPreference::GitExclude => {
            println!(
                "  {} .tasks is in .git/info/exclude (local exclusion)",
                "✓".green()
            );
        }
    }

    // Global exclude warning
    if state.in_global_exclude {
        println!();
        println!(
            "  {} .tasks is in your global git exclude",
            "!".yellow()
        );
        if let Some(ref path) = state.global_exclude_path {
            println!(
                "    To change this, edit: {}",
                path.cyan()
            );
        }
    }
}

/// Prompt user for tracking preference.
fn prompt_preference(state: &TrackingState) -> Result<TrackingPreference, AppError> {
    if !state.is_git_repo {
        // No git, just create the directory
        return Ok(TrackingPreference::Tracked);
    }

    println!();
    println!("{}", "Select tracking preference:".bold());
    println!();
    println!(
        "  {} {} - Include in source control",
        "[1]".cyan(),
        "Tracked".bold()
    );
    println!(
        "  {} {} - Add to .gitignore (shared across clones)",
        "[2]".cyan(),
        "Gitignore".bold()
    );
    println!(
        "  {} {} - Add to .git/info/exclude (local only)",
        "[3]".cyan(),
        "Git exclude".bold()
    );
    println!();

    let current = state.current_preference();
    let default = match current {
        TrackingPreference::Tracked => 1,
        TrackingPreference::Gitignore => 2,
        TrackingPreference::GitExclude => 3,
    };

    print!("Enter choice [1-3] (default: {}): ", default);
    std::io::stdout().flush().with_context("failed to flush stdout")?;

    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .with_context("failed to read input")?;

    let choice = input.trim();
    if choice.is_empty() {
        return Ok(current);
    }

    match choice {
        "1" => Ok(TrackingPreference::Tracked),
        "2" => Ok(TrackingPreference::Gitignore),
        "3" => Ok(TrackingPreference::GitExclude),
        _ => {
            println!("{}: Invalid choice, using default", "warning".yellow());
            Ok(current)
        }
    }
}

/// Apply the tracking preference.
fn apply_preference(
    state: &TrackingState,
    preference: TrackingPreference,
) -> Result<(), AppError> {
    // Create .tasks directory if needed
    if !state.tasks_dir_exists {
        std::fs::create_dir(".tasks").with_context("failed to create .tasks directory")?;
        println!("  {} Created .tasks directory", "✓".green());
    }

    // Create default config.yml if it doesn't exist
    let config_path = Path::new(".tasks/config.yml");
    if !config_path.exists() {
        let default_config = "# Mont configuration\n# See https://github.com/Sawyer-Powell/mont for options\n\njj:\n  enabled: true\n\ndefault_gates: []\n";
        std::fs::write(config_path, default_config)
            .with_context("failed to create config.yml")?;
        println!("  {} Created .tasks/config.yml", "✓".green());
    }

    if !state.is_git_repo {
        return Ok(());
    }

    let current = state.current_preference();

    // Remove from old location if changing
    if current != preference {
        match current {
            TrackingPreference::Tracked => {} // Nothing to remove
            TrackingPreference::Gitignore => {
                remove_from_gitignore()?;
            }
            TrackingPreference::GitExclude => {
                remove_from_git_exclude()?;
            }
        }
    }

    // Add to new location (must happen before untracking for jj)
    match preference {
        TrackingPreference::Tracked => {
            println!("  {} .tasks will be tracked in version control", "✓".green());
            // Auto-commit .tasks changes
            commit_tasks_init()?;
        }
        TrackingPreference::Gitignore => {
            add_to_gitignore()?;
        }
        TrackingPreference::GitExclude => {
            add_to_git_exclude()?;
        }
    }

    // If switching to ignored mode, untrack the files AFTER adding to ignore
    // (jj requires files to be ignored before they can be untracked)
    // We always try this when preference is not Tracked, even if git says files
    // aren't tracked, because jj might still be tracking them
    if preference != TrackingPreference::Tracked {
        untrack_tasks()?;
    }

    Ok(())
}

/// Commit .tasks changes with "mont init" message.
fn commit_tasks_init() -> Result<(), AppError> {
    // Check if jj is available
    let jj_available = Command::new("jj")
        .args(["--version"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !jj_available {
        return Ok(());
    }

    // Check if there are .tasks changes to commit
    let status_output = Command::new("jj")
        .args(["diff", "--summary"])
        .output()
        .with_context("failed to run jj diff")?;

    let status = String::from_utf8_lossy(&status_output.stdout);
    let has_tasks_changes = status.lines().any(|line| line.contains(".tasks"));

    if !has_tasks_changes {
        return Ok(());
    }

    // Commit .tasks changes
    let commit_output = Command::new("jj")
        .args(["commit", "-m", "mont init", ".tasks"])
        .output()
        .with_context("failed to run jj commit")?;

    if commit_output.status.success() {
        println!("  {} Committed .tasks changes", "✓".green());
    }

    Ok(())
}

/// Untrack .tasks from git/jj while keeping files on disk.
fn untrack_tasks() -> Result<(), AppError> {
    println!("  {} Removing .tasks from version control tracking...", "→".cyan());

    // First, try git rm --cached (for git index)
    let git_output = Command::new("git")
        .args(["rm", "-r", "--cached", ".tasks"])
        .output()
        .with_context("failed to run git rm")?;

    if !git_output.status.success() {
        let stderr = String::from_utf8_lossy(&git_output.stderr);
        // Ignore "pathspec did not match" errors - means files weren't tracked
        if !stderr.contains("did not match") {
            return Err(AppError::CommandFailed(format!(
                "failed to untrack .tasks from git: {}",
                stderr
            )));
        }
    }

    // Then, try jj file untrack (for jj working copy)
    // This requires files to already be in gitignore/exclude
    let jj_output = Command::new("jj")
        .args(["file", "untrack", ".tasks"])
        .output();

    match jj_output {
        Ok(output) if output.status.success() => {
            println!("  {} Removed .tasks from tracking (files preserved)", "✓".green());
        }
        Ok(output) => {
            // jj file untrack failed - might not be a jj repo or files not ignored yet
            let stderr = String::from_utf8_lossy(&output.stderr);
            // If jj isn't available or this isn't a jj repo, that's fine
            if !stderr.contains("not ignored") {
                println!("  {} Removed .tasks from git index (files preserved)", "✓".green());
            } else {
                // Files not ignored - shouldn't happen since we add to ignore first
                eprintln!("  {} Warning: jj untrack failed: {}", "!".yellow(), stderr.trim());
            }
        }
        Err(_) => {
            // jj not available, that's fine - we already did git rm --cached
            println!("  {} Removed .tasks from git index (files preserved)", "✓".green());
        }
    }

    Ok(())
}

/// Add .tasks to .gitignore.
fn add_to_gitignore() -> Result<(), AppError> {
    let gitignore_path = Path::new(".gitignore");
    let mut content = if gitignore_path.exists() {
        std::fs::read_to_string(gitignore_path).with_context("failed to read .gitignore")?
    } else {
        String::new()
    };

    if !contains_tasks_pattern(&content) {
        if !content.is_empty() && !content.ends_with('\n') {
            content.push('\n');
        }
        content.push_str(".tasks/\n");
        std::fs::write(gitignore_path, content).with_context("failed to write .gitignore")?;
        println!("  {} Added .tasks/ to .gitignore", "✓".green());
    }

    Ok(())
}

/// Remove .tasks from .gitignore.
fn remove_from_gitignore() -> Result<(), AppError> {
    let gitignore_path = Path::new(".gitignore");
    if !gitignore_path.exists() {
        return Ok(());
    }

    let content = std::fs::read_to_string(gitignore_path).with_context("failed to read .gitignore")?;
    let new_content: String = content
        .lines()
        .filter(|line| !contains_tasks_pattern(&format!("{}\n", line)))
        .map(|line| format!("{}\n", line))
        .collect();

    std::fs::write(gitignore_path, new_content).with_context("failed to write .gitignore")?;
    println!("  {} Removed .tasks from .gitignore", "✓".green());
    Ok(())
}

/// Add .tasks to .git/info/exclude.
fn add_to_git_exclude() -> Result<(), AppError> {
    let exclude_path = Path::new(".git/info/exclude");

    // Ensure .git/info directory exists
    std::fs::create_dir_all(".git/info").with_context("failed to create .git/info")?;

    let mut content = if exclude_path.exists() {
        std::fs::read_to_string(exclude_path).with_context("failed to read .git/info/exclude")?
    } else {
        String::new()
    };

    if !contains_tasks_pattern(&content) {
        if !content.is_empty() && !content.ends_with('\n') {
            content.push('\n');
        }
        content.push_str(".tasks/\n");
        std::fs::write(exclude_path, content).with_context("failed to write .git/info/exclude")?;
        println!("  {} Added .tasks/ to .git/info/exclude", "✓".green());
    }

    Ok(())
}

/// Remove .tasks from .git/info/exclude.
fn remove_from_git_exclude() -> Result<(), AppError> {
    let exclude_path = Path::new(".git/info/exclude");
    if !exclude_path.exists() {
        return Ok(());
    }

    let content =
        std::fs::read_to_string(exclude_path).with_context("failed to read .git/info/exclude")?;
    let new_content: String = content
        .lines()
        .filter(|line| !contains_tasks_pattern(&format!("{}\n", line)))
        .map(|line| format!("{}\n", line))
        .collect();

    std::fs::write(exclude_path, new_content).with_context("failed to write .git/info/exclude")?;
    println!("  {} Removed .tasks from .git/info/exclude", "✓".green());
    Ok(())
}

/// Initialize mont in the current directory.
pub fn init() -> Result<(), AppError> {
    println!("{}", "Initializing mont...".bold());
    println!();

    // Detect current state
    let state = TrackingState::detect();

    // Display current configuration
    display_state(&state);

    // Prompt for preference
    let preference = prompt_preference(&state)?;

    println!();
    println!("{}", "Applying configuration...".bold());

    // Apply the preference
    apply_preference(&state, preference)?;

    println!();
    println!("{}", "Mont initialized successfully!".green().bold());

    if !state.tasks_dir_exists {
        println!();
        println!("Get started:");
        println!("  {} - Create a new jot", "mont jot".cyan());
        println!("  {} - Create a new task", "mont task".cyan());
        println!("  {} - List all tasks", "mont list".cyan());
    }

    Ok(())
}
