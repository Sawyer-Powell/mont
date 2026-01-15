---
id: mont-start
title: Implement mont start command
parent: cli-commands
preconditions:
  - add-jj-lib
  - in-progress-status
---

Implement the `mont start <task-id>` CLI command.

# Behavior

1. Takes a task ID as argument
2. Validates the task exists in `.tasks/`
3. Creates a new JJ revision empty
4. Marks task as in progress in markdown file
5. Prints confirmation message

# Implementation

- Use clap for CLI argument parsing
- Load task from `.tasks/<task-id>.md`
- Use jj module to create revision

# Acceptance Criteria

- `mont start form-graph` creates a new JJ revision
- Error if task ID doesn't exist
- Revision description includes task title
