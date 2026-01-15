---
id: mont-list
title: Implement mont list command
parent: cli-commands
complete: true
validations:
  - test
---

Implement the `mont list` CLI command to show all tasks.

# Behavior

1. Reads all `.md` files from `.tasks/` directory
2. Parses each as a Task
3. Displays task list with: id, title, complete status
4. Display all tasks in a pretty graph format, kind of like jj log.

Work with me first to determine a good library for outputting pretty
console output. Before we write the cli, we need a test harness
set up to iterate on the design of the output of the mont list output.

The test harness should include a simple comprehensive variety of tasks.

# Acceptance Criteria

- Lists all tasks from `.tasks/`
- Shows completion status
- Display whether task is a validator task or not
- Indicate clearly whether a task is a child or not
- Display output in a graph format, like jj log
- Stop the world if parse errors are detected, these must be corrected immediately
