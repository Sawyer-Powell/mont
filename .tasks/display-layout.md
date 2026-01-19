---
id: display-layout
title: Types + level assignment + positioning + grid construction
before:
  - display-refactor
status: complete
gates:
  - test
---

Define core types and implement layout computation for the grid-based display.

# Types to Define

```rust
enum Cell {
    Task(TaskId),
    Empty,
    Connection { up: bool, down: bool, left: bool, right: bool },
}

struct Grid {
    rows: Vec<Vec<Cell>>,
}
```

# Level Assignment

Use BFS from source nodes (no predecessors). Level = longest path from any source.

# Positioning

Within each level, order tasks by priority:
1. In-progress tasks (highest)
2. Bug tasks
3. Regular tasks
4. Alphabetical by ID (tiebreaker)

# Grid Construction

Build initial grid with Task cells placed, Empty elsewhere.
Each task gets its own row.
