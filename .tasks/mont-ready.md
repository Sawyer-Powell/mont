---
id: mont-ready
title: Implement mont ready command
parent: cli-commands
preconditions:
  - mont-list
  - in-progress-status
---

Implement the `mont ready` CLI command to show tasks ready to work on.

# Behavior

1. Loads all tasks and forms the task graph
2. Finds tasks where:
   - Not complete
   - All preconditions are complete
   - Not a validator task
3. Displays ready tasks

# Output Format

```
Ready tasks:
  task-id-1: Task Title One
  task-id-2: Task Title Two
```

# Implementation

- Use `form_graph` to validate and build graph
- Filter for ready tasks based on precondition completion
- Sort by some criteria (alphabetical or by parent hierarchy)

# Acceptance Criteria

- Shows only tasks with all preconditions met
- Excludes completed tasks
- Excludes validator tasks
