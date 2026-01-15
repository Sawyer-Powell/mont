---
id: in-progress-status
title: Add in-progress status to tasks
---

Add support for marking tasks as "in progress" to track active work.

# Motivation

Currently tasks are either complete or not. We need an intermediate state to indicate a task is actively being worked on. This enables:
- Seeing what's currently in flight
- Preventing multiple agents from claiming the same task
- Better visibility into project status

# Changes

## Task struct
- Add `in_progress: bool` field (default false) to Task in `task.rs`
- A task cannot be both `in_progress: true` and `complete: true`

## Validation rules
- Reject tasks with both `in_progress: true` and `complete: true`

## Display
- Show in-progress tasks with a distinct marker (maybe `◐` or `◑`)
- In-progress tasks should be visually prominent (not dimmed like blocked)

# Example

```yaml
---
id: my-task
title: Some task
in_progress: true
---

Currently working on this.
```

# Acceptance Criteria

- Can parse tasks with `in_progress: true`
- Validation rejects tasks that are both in_progress and complete
- Display shows in-progress marker distinctly
- In-progress tasks sorted/grouped appropriately in list output
