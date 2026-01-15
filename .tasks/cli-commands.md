---
id: cli-commands
title: Mont CLI Commands
---

Parent task grouping all CLI command implementations.

# Commands

- `mont start <task-id>` - Create JJ revision for a task
- `mont new` - Create a new task interactively
- `mont list` - List all tasks
- `mont ready` - Show tasks ready to work on
- `mont complete <task-id>` - Mark a task as complete

# Dependencies

This requires:
- jj-lib integration for `mont start`
- complete field for `mont complete`
