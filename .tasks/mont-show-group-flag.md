---
id: mont-show-group-flag
title: Add -g/--group flag to mont show command
status: complete
gates:
  - user-qa: passed
  - test: passed
  - semver: passed
  - architecture-validator: passed
---

Add `--group` / `-g` flag to `mont show` that expands a task ID to include its full subgraph (all connected tasks via before/after dependencies), matching the existing behavior in `mont task`.

## Implementation

1. Add `--group` / `-g` bool flag to `Show` command in `src/main.rs`
2. Update `commands::show()` signature to accept the group flag
3. When `--group` is set:
   - Use `ctx.graph().subgraph()` to expand the ID (same pattern as `task_cmd.rs` line ~131)
   - Get topological order and filter to subgraph
   - Loop through and display each task
4. Add visual separator between tasks when showing multiple

## Example usage

```
mont show task-id      # Shows single task (current behavior)
mont show task-id -g   # Shows task-id AND all connected tasks
```

## Reference

See `src/commands/task_cmd.rs` lines 131-145 for the existing group expansion logic.
