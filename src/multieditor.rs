//! Multieditor - unified diff-based task editing system.
//!
//! This module provides the core logic for the multieditor, which handles
//! creating, editing, and deleting multiple tasks in a single editor session.

use crate::error_fmt::AppError;
use crate::{MontContext, Task};

/// The result of comparing original tasks with edited tasks.
#[derive(Debug, Default)]
pub struct MultiEditDiff {
    /// New tasks that were added (beyond the original count)
    pub inserts: Vec<Task>,
    /// Tasks that were updated (original_id, updated_task)
    pub updates: Vec<(String, Task)>,
    /// IDs of tasks that were removed (missing from output)
    pub deletes: Vec<String>,
}

impl MultiEditDiff {
    /// Returns true if no changes were made.
    pub fn is_empty(&self) -> bool {
        self.inserts.is_empty() && self.updates.is_empty() && self.deletes.is_empty()
    }

    /// Returns the total number of changes.
    pub fn change_count(&self) -> usize {
        self.inserts.len() + self.updates.len() + self.deletes.len()
    }
}

/// Result of applying a diff.
pub struct ApplyResult {
    /// IDs of created tasks
    pub created: Vec<String>,
    /// (original_id, new_id, id_changed)
    pub updated: Vec<(String, String, bool)>,
    /// IDs of deleted tasks
    pub deleted: Vec<String>,
}

/// Compute the diff between original tasks and edited tasks.
///
/// Matching logic (ID-based):
/// - Tasks are matched by their `id` field against the graph
/// - If `new_id` is set, it's a rename: match by `id`, rename to `new_id`
/// - If an edited task's ID matches an existing task, it's an update
/// - If an edited task's ID doesn't match any existing task, it's an insert
/// - If an original task's ID isn't in the edited list (and not renamed), it's a delete
pub fn compute_diff(original: &[Task], edited: &[Task]) -> MultiEditDiff {
    use std::collections::{HashMap, HashSet};

    let mut diff = MultiEditDiff::default();

    // Build lookup of original tasks by ID
    let original_by_id: HashMap<&str, &Task> = original
        .iter()
        .map(|t| (t.id.as_str(), t))
        .collect();

    // Track which original IDs we've seen in edited (either by id match or rename)
    let mut seen_original_ids: HashSet<&str> = HashSet::new();

    // Process edited tasks
    for edited_task in edited {
        // Check for rename: new_id is set
        if let Some(ref new_id) = edited_task.new_id {
            // This is a rename: id is the original, new_id is the target
            if let Some(&original_task) = original_by_id.get(edited_task.id.as_str()) {
                seen_original_ids.insert(&edited_task.id);

                // Create the renamed task (use new_id as the actual id)
                let mut renamed_task = edited_task.clone();
                renamed_task.id = new_id.clone();
                renamed_task.new_id = None; // Clear the rename marker

                diff.updates.push((original_task.id.clone(), renamed_task));
            } else {
                // new_id set but original doesn't exist - treat as insert with warning
                let mut new_task = edited_task.clone();
                new_task.id = new_id.clone();
                new_task.new_id = None;
                diff.inserts.push(new_task);
            }
        } else if let Some(&original_task) = original_by_id.get(edited_task.id.as_str()) {
            // Same ID exists in original - check if content changed
            seen_original_ids.insert(&edited_task.id);

            if task_content_differs(original_task, edited_task) {
                diff.updates.push((original_task.id.clone(), edited_task.clone()));
            }
        } else {
            // New ID - this is an insert
            diff.inserts.push(edited_task.clone());
        }
    }

    // Original tasks not in edited are deletes
    for original_task in original {
        if !seen_original_ids.contains(original_task.id.as_str()) {
            diff.deletes.push(original_task.id.clone());
        }
    }

    diff
}

/// Check if task content differs (ignoring ID which is handled separately).
fn task_content_differs(a: &Task, b: &Task) -> bool {
    a.title != b.title
        || a.description != b.description
        || a.before != b.before
        || a.after != b.after
        || a.gates != b.gates
        || a.task_type != b.task_type
        || a.status != b.status
}

/// Fill in empty IDs in a diff before displaying to the user.
///
/// This generates IDs for tasks with empty IDs so the confirmation
/// prompt can show the actual IDs that will be created.
pub fn fill_empty_ids(ctx: &MontContext, diff: &mut MultiEditDiff) -> Result<(), AppError> {
    let graph = ctx.graph();

    // Fill empty IDs in inserts
    for task in &mut diff.inserts {
        if task.id.is_empty() {
            task.id = ctx.generate_id(&graph).map_err(AppError::from)?;
        }
    }

    // Fill empty IDs in updates (rare but possible)
    for (_original_id, task) in &mut diff.updates {
        if task.id.is_empty() {
            task.id = ctx.generate_id(&graph).map_err(AppError::from)?;
        }
    }

    Ok(())
}

/// Apply a diff to the task graph atomically.
///
/// Uses the transaction system for atomic validation:
/// 1. Builds a single Transaction with all inserts/updates/deletes
/// 2. ctx.commit(txn) creates a ValidationView overlaying all changes
/// 3. Full validation runs on the view (reference integrity, cycle detection)
/// 4. Only if validation passes -> all changes applied atomically
/// 5. If validation fails -> no changes applied
///
/// Note: Call `fill_empty_ids()` before this if you want to show the user
/// the actual IDs that will be created.
pub fn apply_diff(ctx: &MontContext, diff: MultiEditDiff) -> Result<ApplyResult, AppError> {
    let mut txn = ctx.begin();
    let graph = ctx.graph();

    let mut result = ApplyResult {
        created: Vec::new(),
        updated: Vec::new(),
        deleted: Vec::new(),
    };

    // Process deletes first - rewrite references for deleted tasks
    for id in &diff.deletes {
        txn.rewrite_references(&*graph, id, None);
        txn.delete(id);
        result.deleted.push(id.clone());
    }

    // Process updates - handle renames
    for (original_id, mut task) in diff.updates {
        let id_changed = task.id != original_id;

        // If ID changed, rewrite references to point to new ID
        if id_changed {
            txn.rewrite_references(&*graph, &original_id, Some(&task.id));
        }

        // Generate ID if empty
        if task.id.is_empty() {
            task.id = ctx.generate_id(&graph)
                .map_err(AppError::from)?;
        }

        result.updated.push((original_id.clone(), task.id.clone(), id_changed));
        txn.update(&original_id, task);
    }

    // Process inserts
    for mut task in diff.inserts {
        // Generate ID if empty
        if task.id.is_empty() {
            task.id = ctx.generate_id(&graph)
                .map_err(AppError::from)?;
        }

        result.created.push(task.id.clone());
        txn.insert(task);
    }

    // Drop the graph guard before committing
    drop(graph);

    // Commit atomically - validates all changes together
    ctx.commit(txn)?;

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TaskType;

    fn make_task(id: &str, title: &str) -> Task {
        Task {
            id: id.to_string(),
            new_id: None,
            title: Some(title.to_string()),
            description: String::new(),
            before: vec![],
            after: vec![],
            gates: vec![],
            task_type: TaskType::Task,
            status: None,
            deleted: false,
        }
    }

    #[test]
    fn test_compute_diff_no_changes() {
        let original = vec![make_task("task1", "Task 1")];
        let edited = vec![make_task("task1", "Task 1")];

        let diff = compute_diff(&original, &edited);
        assert!(diff.is_empty());
    }

    #[test]
    fn test_compute_diff_update() {
        let original = vec![make_task("task1", "Task 1")];
        let edited = vec![make_task("task1", "Updated Title")];

        let diff = compute_diff(&original, &edited);
        assert_eq!(diff.updates.len(), 1);
        assert_eq!(diff.updates[0].0, "task1");
        assert_eq!(diff.updates[0].1.title, Some("Updated Title".to_string()));
        assert!(diff.inserts.is_empty());
        assert!(diff.deletes.is_empty());
    }

    #[test]
    fn test_compute_diff_rename() {
        // With ID-based matching, changing an ID is a delete + insert, not an update
        let original = vec![make_task("old-id", "Task 1")];
        let edited = vec![make_task("new-id", "Task 1")];

        let diff = compute_diff(&original, &edited);
        // old-id is deleted, new-id is inserted
        assert!(diff.updates.is_empty());
        assert_eq!(diff.inserts.len(), 1);
        assert_eq!(diff.inserts[0].id, "new-id");
        assert_eq!(diff.deletes.len(), 1);
        assert_eq!(diff.deletes[0], "old-id");
    }

    #[test]
    fn test_compute_diff_insert() {
        let original = vec![make_task("task1", "Task 1")];
        let edited = vec![
            make_task("task1", "Task 1"),
            make_task("task2", "Task 2"),
        ];

        let diff = compute_diff(&original, &edited);
        assert!(diff.updates.is_empty());
        assert_eq!(diff.inserts.len(), 1);
        assert_eq!(diff.inserts[0].id, "task2");
        assert!(diff.deletes.is_empty());
    }

    #[test]
    fn test_compute_diff_delete() {
        let original = vec![
            make_task("task1", "Task 1"),
            make_task("task2", "Task 2"),
        ];
        let edited = vec![make_task("task1", "Task 1")];

        let diff = compute_diff(&original, &edited);
        assert!(diff.updates.is_empty());
        assert!(diff.inserts.is_empty());
        assert_eq!(diff.deletes.len(), 1);
        assert_eq!(diff.deletes[0], "task2");
    }

    #[test]
    fn test_compute_diff_complex() {
        // ID-based matching: tasks are matched by ID, not position
        let original = vec![
            make_task("task1", "Task 1"),
            make_task("task2", "Task 2"),
            make_task("task3", "Task 3"),
        ];
        let edited = vec![
            make_task("task1", "Updated Task 1"),  // ID matches: update
            make_task("renamed", "Task 2"),        // new ID: insert (task2 deleted)
            make_task("task4", "Task 4"),          // new ID: insert (task3 deleted)
        ];

        let diff = compute_diff(&original, &edited);

        // One update: task1 content changed
        assert_eq!(diff.updates.len(), 1);
        assert_eq!(diff.updates[0].0, "task1");

        // Two inserts: renamed and task4 (new IDs)
        assert_eq!(diff.inserts.len(), 2);

        // Two deletes: task2 and task3 (IDs not in edited)
        assert_eq!(diff.deletes.len(), 2);
    }

    #[test]
    fn test_compute_diff_with_delete_and_insert() {
        // ID-based matching: delete when ID removed, insert when new ID added
        let original = vec![
            make_task("task1", "Task 1"),
            make_task("task2", "Task 2"),
            make_task("task3", "Task 3"),
        ];
        // Keep task1 and task2, remove task3
        let edited = vec![
            make_task("task1", "Task 1"),          // ID matches: no change
            make_task("task2", "Task 2"),          // ID matches: no change
            // task3 ID not present: deleted
        ];

        let diff = compute_diff(&original, &edited);

        // No updates (task1 and task2 unchanged)
        assert!(diff.updates.is_empty());

        // No inserts
        assert!(diff.inserts.is_empty());

        // One delete: task3 (ID not in edited)
        assert_eq!(diff.deletes.len(), 1);
        assert_eq!(diff.deletes[0], "task3");
    }

    #[test]
    fn test_compute_diff_insert_only() {
        let original = vec![
            make_task("task1", "Task 1"),
        ];
        let edited = vec![
            make_task("task1", "Task 1"),          // ID matches: no change
            make_task("task2", "Task 2"),          // new ID: insert
        ];

        let diff = compute_diff(&original, &edited);

        assert!(diff.updates.is_empty());
        assert_eq!(diff.inserts.len(), 1);
        assert_eq!(diff.inserts[0].id, "task2");
        assert!(diff.deletes.is_empty());
    }

    #[test]
    fn test_compute_diff_empty_to_tasks() {
        let original: Vec<Task> = vec![];
        let edited = vec![
            make_task("task1", "Task 1"),
            make_task("task2", "Task 2"),
        ];

        let diff = compute_diff(&original, &edited);
        assert!(diff.updates.is_empty());
        assert_eq!(diff.inserts.len(), 2);
        assert!(diff.deletes.is_empty());
    }

    #[test]
    fn test_compute_diff_tasks_to_empty() {
        let original = vec![
            make_task("task1", "Task 1"),
            make_task("task2", "Task 2"),
        ];
        let edited: Vec<Task> = vec![];

        let diff = compute_diff(&original, &edited);
        assert!(diff.updates.is_empty());
        assert!(diff.inserts.is_empty());
        assert_eq!(diff.deletes.len(), 2);
    }
}
