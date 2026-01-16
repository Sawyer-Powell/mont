---
id: in-progress-status
title: Add in-progress status to tasks
complete: true
---

Add support for marking tasks as "in progress" to track active work.

# Motivation

Currently tasks are either complete or not. We need an intermediate state to indicate a task is actively being worked on. This enables:
- Seeing what's currently in flight
- Built better tracing through jj revisions which revisions addressed which tasks

# Changes

## Task struct
- Add `in_progress: Option<u32>` field (default None) to Task in `task.rs`
- A non None value means the task is in progress
- Modeled as an int so a task that takes many jj revisions to complete can show up in the revision. I.e. the in progress counter is incremented each revision.

## Display
- Show in-progress tasks with a distinct marker (maybe `◐` or `◑`)
- In-progress tasks should be visually prominent, maybe an orange color (not dimmed like blocked)

# Example

```yaml
---
id: my-task
title: Some task
in_progress: 1
---

Currently working on this.
```

# Acceptance Criteria

- Can parse tasks with `in_progress: u32`
- Display shows in-progress marker distinctly
- In-progress tasks sorted/grouped appropriately in list output
