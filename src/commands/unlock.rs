//! Unlock command - marks gates as passed or skipped.

use std::collections::HashSet;

use owo_colors::OwoColorize;

use crate::error_fmt::AppError;
use crate::{MontContext, Task, GateItem, GateStatus};

/// Arguments for unlocking gates on a task.
pub struct UnlockArgs {
    pub id: String,
    pub passed: Vec<String>,
    pub skipped: Vec<String>,
}

/// Arguments for locking (resetting) gates on a task.
pub struct LockArgs {
    pub id: String,
    pub gates: Vec<String>,
}

/// Gate update specification: which gates to set to which status.
struct GateUpdates {
    passed: Vec<String>,
    skipped: Vec<String>,
    pending: Vec<String>,
}

impl GateUpdates {
    fn all_gates(&self) -> impl Iterator<Item = &String> {
        self.passed.iter().chain(&self.skipped).chain(&self.pending)
    }

    fn status_for(&self, gate_id: &str) -> Option<GateStatus> {
        if self.passed.iter().any(|g| g == gate_id) {
            Some(GateStatus::Passed)
        } else if self.skipped.iter().any(|g| g == gate_id) {
            Some(GateStatus::Skipped)
        } else if self.pending.iter().any(|g| g == gate_id) {
            Some(GateStatus::Pending)
        } else {
            None
        }
    }
}

/// Core gate update logic shared by lock and unlock.
fn update_gates(ctx: &MontContext, task_id: &str, updates: GateUpdates) -> Result<Task, AppError> {
    let graph = ctx.graph();

    // Get the task
    let task = graph
        .get(task_id)
        .ok_or_else(|| AppError::TaskNotFound {
            task_id: task_id.to_string(),
            tasks_dir: ctx.tasks_dir().display().to_string(),
        })?
        .clone();

    // Validate all specified gates exist
    let valid_gates = ctx.all_gate_ids(&task);
    for gate_id in updates.all_gates() {
        if !valid_gates.contains(gate_id) {
            return Err(AppError::GateNotValid {
                gate_id: gate_id.clone(),
                task_id: task_id.to_string(),
            });
        }
    }

    // Build updated gates list
    let mut new_gates: Vec<GateItem> = Vec::new();
    let mut seen_ids: HashSet<String> = HashSet::new();

    // Update existing gates
    for gate in &task.gates {
        let new_status = updates.status_for(&gate.id).unwrap_or(gate.status);
        new_gates.push(GateItem {
            id: gate.id.clone(),
            status: new_status,
        });
        seen_ids.insert(gate.id.clone());
    }

    // Add any gates that were specified but not already on the task
    for gate_id in updates.all_gates() {
        if !seen_ids.contains(gate_id)
            && let Some(status) = updates.status_for(gate_id)
        {
            new_gates.push(GateItem {
                id: gate_id.clone(),
                status,
            });
            seen_ids.insert(gate_id.clone());
        }
    }

    drop(graph);

    // Build and save updated task
    let updated = Task {
        gates: new_gates,
        ..task
    };
    ctx.update(task_id, updated.clone())?;

    Ok(updated)
}

/// Unlock gates on a task by marking them as passed or skipped.
pub fn unlock(ctx: &MontContext, args: UnlockArgs) -> Result<(), AppError> {
    let updates = GateUpdates {
        passed: args.passed.clone(),
        skipped: args.skipped.clone(),
        pending: vec![],
    };

    update_gates(ctx, &args.id, updates)?;

    // Print summary
    let total_updated = args.passed.len() + args.skipped.len();
    if !args.passed.is_empty() {
        println!(
            "{} {} marked as passed",
            args.passed.join(", ").bright_green(),
            if args.passed.len() == 1 { "gate" } else { "gates" }
        );
    }
    if !args.skipped.is_empty() {
        println!(
            "{} {} marked as skipped",
            args.skipped.join(", ").yellow(),
            if args.skipped.len() == 1 { "gate" } else { "gates" }
        );
    }

    if total_updated == 0 {
        println!("No gates updated");
    }

    Ok(())
}

/// Lock gates on a task by resetting them to pending.
pub fn lock(ctx: &MontContext, args: LockArgs) -> Result<(), AppError> {
    let updates = GateUpdates {
        passed: vec![],
        skipped: vec![],
        pending: args.gates.clone(),
    };

    update_gates(ctx, &args.id, updates)?;

    // Print summary
    if !args.gates.is_empty() {
        println!(
            "{} {} reset to pending",
            args.gates.join(", ").bright_black(),
            if args.gates.len() == 1 { "gate" } else { "gates" }
        );
    } else {
        println!("No gates updated");
    }

    Ok(())
}
