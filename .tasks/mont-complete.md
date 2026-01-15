---
id: mont-complete
title: Implement mont complete command
parent: cli-commands
preconditions:
  - add-complete-field
  - add-jj-lib
---

Implement the `mont complete <task-id>` CLI command to mark a task as complete.

# Behavior

1. Takes a task ID as argument
2. Loads the task file
3. Sets `complete: true` in frontmatter
4. Writes the updated task file
5. Prints confirmation

# Implementation

- Parse task file
- Modify the complete field
- Serialize back to markdown with frontmatter
- Need a `task::serialize` or `task::write` function

# Challenges

- Preserving markdown description when rewriting
- Handling YAML serialization cleanly

# Acceptance Criteria

- `mont complete task-id` sets complete: true
- Original description preserved
- Idempotent (running twice is fine)
