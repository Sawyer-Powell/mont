---
id: mont-list
title: Implement mont list command
parent: cli-commands
---

Implement the `mont list` CLI command to show all tasks.

# Behavior

1. Reads all `.md` files from `.tasks/` directory
2. Parses each as a Task
3. Displays task list with: id, title, complete status

# Output Format

```
[ ] task-id-1: Task Title One
[x] task-id-2: Task Title Two (complete)
[ ] task-id-3: Task Title Three
```

# Implementation

- Use glob or std::fs to find task files
- Parse each with `task::parse`
- Format and print to stdout

# Acceptance Criteria

- Lists all tasks from `.tasks/`
- Shows completion status
- Handles parse errors gracefully (skip or warn)
