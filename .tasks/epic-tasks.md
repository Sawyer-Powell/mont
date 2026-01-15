---
id: epic-tasks
title: Allow tasks to be marked as epics
---

Add support for marking tasks as epics - high-level grouping tasks that organize related work.

# Motivation

Epics are tasks that exist purely to group other tasks. They:
- Cannot be worked on directly (they have no actionable work)
- Are complete when all their children are complete
- Provide organizational structure in the task graph

# Changes

## Task struct
- Add `epic: bool` field (default false) to Task in `task.rs`
- Add serde deserialization support

## Validation rules
- Epic tasks must have at least one child (task with this epic as parent)
- Epic tasks cannot have preconditions (they don't represent work)
- Epic tasks cannot be marked complete manually (derived from children)

## Display
- Show epics with a distinct marker (maybe `◆` or similar)
- Epic completion status derived from children, not from `complete` field

# Example

```yaml
---
id: auth-system
title: Authentication System
epic: true
---

High-level epic for all authentication work.
```

# Acceptance Criteria

- Can parse tasks with `epic: true`
- Graph validation rejects epics without children
- Graph validation rejects epics with preconditions
- Display shows epic marker
- Epic shown as complete only when all children complete
