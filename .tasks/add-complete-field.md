---
id: add-complete-field
title: Add complete field to Task struct
complete: true
---

Add an optional `complete` field to the Task struct to track whether a task is complete.

# Changes

- Add `complete: Option<bool>` to Task struct in `task.rs` (defaults to None/false)
- Update serde deserialization to handle the field

# Acceptance Criteria

- Existing tests still pass
- Can parse a task with `complete: true` in frontmatter
- Can parse a task without the complete field (defaults to None)
