---
id: status-show-stopped-count
title: Show count of stopped tasks in mont status info section
---

Add a line to the Info section of `mont status` that displays the number of stopped tasks. Stopped tasks are tasks that were started but then paused via `mont stop`.

## Implementation

1. Add a count of stopped tasks: `graph.values().filter(|t| t.is_stopped()).count()`
2. Add a `println!` line in the Info section displaying this count (similar to ready/jots/gates/completed)

## Acceptance Criteria

- The Info section of `mont status` shows the count of stopped tasks
- The count uses consistent formatting with other info lines (left-aligned number in 4-char field)
