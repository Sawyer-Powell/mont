---
id: display-refactor
title: Refactor display.rs with level-based grid rendering
type: task
status: complete
---

Refactor the task graph display from a stateful "active lines" approach to a clean
level-based grid rendering system.

# Problem

Current `display.rs` (1335 lines) has:
- Complex `active_lines: Vec<Option<&str>>` state management
- 7-level nested conditionals for fork/merge symbol selection
- Same decision tree repeated 3x for color/marker/title

# Solution

Pipeline: Tasks -> Level Assignment -> Positioning -> Grid Construction -> Edge Routing -> Symbols -> Render

Core data structures:
- `Cell` enum: `Task(TaskId)`, `Empty`, `Connection { up, down, left, right }`
- `Grid` struct with `rows: Vec<Vec<Cell>>`

# Subtasks

1. display-layout - Types + level assignment + positioning + grid construction
2. display-routing - Edge routing through grid cells
3. display-symbols - Cell-to-ASCII symbol conversion
4. display-render - Wiring + final output
