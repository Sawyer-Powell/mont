---
id: display-routing
title: Edge routing through grid cells
parent: display-refactor
preconditions:
  - display-layout
validations:
  - test
---

Route edges through the grid, updating Connection cells.

# Input

- Grid from display-layout
- Edges from `graph::transitive_reduction()`

# Routing Algorithm

For each edge (from_task, to_task):
1. Find the row/column of from_task and to_task
2. Update Connection cells along the path
3. Set up/down/left/right flags appropriately

# Edge Cases

- Forks: one task with multiple dependents
- Merges: multiple tasks pointing to one dependent
- Long vertical spans: edges crossing multiple levels
