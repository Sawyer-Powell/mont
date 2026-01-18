---
id: bug-tasks
title: Allow tasks to be marked as bugs, features, or epics
complete: true
---

Add support for marking tasks as bugs to distinguish defect fixes from feature work.

# Motivation

Bugs have different characteristics than feature tasks:
- Often higher priority
- May need different tracking/metrics
- Useful for filtering and reporting

# Changes

## Task struct
- Add `TaskType` enum with variants: `Bug`, `Epic`, `Feature`
- Add `type` field to Task (defaults to `Feature` if not specified)
- Use `#[serde(rename = "type")]` since `type` is a reserved word in Rust

## Display
- Show `[bug]` suffix after the title (similar to `[validator]`)
- `[bug]` is displayed in red when the task is available (not complete/blocked)
- `[bug]` is displayed in dimmed text when the task is complete or blocked
- `[epic]` is displayed in cyan when available, dimmed when not
- Bugs/epics are mixed with regular tasks (no special ordering)

# Example

```yaml
---
id: fix-login-crash
title: Fix crash on login with empty password
type: task
---

App crashes when user submits login form with empty password field.
```

# Acceptance Criteria

- Can parse tasks with `type: bug`, `type: epic`, or `type: feature`
- Tasks without a type field default to `feature`
- Display shows `[bug]` suffix in red for available bugs
- Display shows `[bug]` suffix in dimmed text for complete/blocked bugs
- Display shows `[epic]` suffix in cyan for available epics
- Bugs/epics visually distinguishable from regular feature tasks
