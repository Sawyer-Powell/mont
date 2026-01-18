---
id: actionable-errors
title: Make all error messages actionable
status: complete
---

Improve error messages throughout mont to include clear next action steps.

# Motivation

When a user encounters an error, they shouldn't have to guess what to do next.
Every error message should tell them exactly how to fix the problem.

# Examples

Bad:
```
error: task 'setup-db' references invalid parent 'database'
```

Good:
```
error: task 'setup-db' references invalid parent 'database'

  The parent task 'database' does not exist in .tasks/

  To fix this, either:
    1. Create the missing task: .tasks/database.md
    2. Remove the parent field from .tasks/setup-db.md
    3. Change the parent to an existing task
```

# Scope

Review and improve error messages in:
- `task.rs` - ParseError variants
- `graph.rs` - GraphError variants
- `main.rs` - CLI errors (file not found, directory missing, etc.)

# Acceptance Criteria

- Each error type has a helpful "To fix this" section
- Error messages reference specific file paths where relevant
- Suggestions are concrete and actionable
