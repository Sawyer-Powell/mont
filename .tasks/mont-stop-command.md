---
id: mont-stop-command
title: Add mont stop command to clear in-progress status
gates:
  - user-qa
  - test
---

Add a new `mont stop` command that clears the in-progress status from a task, making it ready for work again.

## Implementation

1. **CLI in `main.rs`**: Add a `Stop` subcommand with an optional `id` argument
2. **Create `src/commands/stop.rs`**: Implement the `stop()` function
3. **Export in `src/commands/mod.rs`**: Add the module and public export

## Behavior

- If no task ID is provided, use the first in-progress task (similar to how `done` works)
- Validate the task exists and is currently in-progress
- Set `status` to `None` (clearing the in-progress state)
- This makes the task appear in `ready` again

## Notes

- This is NOT the same as setting `Status::Stopped` - we're clearing the status entirely
- Essentially the inverse of `mont start`
