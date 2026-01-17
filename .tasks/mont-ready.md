---
id: mont-ready
title: Implement mont ready command
after:
  - mont-list
  - in-progress-status
complete: true
---

Implement the `mont ready` CLI command to show tasks ready to work on.

# Behavior

Shows all tasks ready for work in a non-graph output

# Output Format

```
task-id-1: Task Title One
task-id-2: Task Title Two
```
# Acceptance Criteria

- Ready tasks mirror the ready tasks calculated from `mont list`
- Excludes completed tasks
- Excludes validator tasks
