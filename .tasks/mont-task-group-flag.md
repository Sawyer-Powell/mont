---
id: mont-task-group-flag
title: Add --group flag to mont task
status: complete
gates:
  - architecture-validator: passed
  - test: passed
---

Add `-g` / `--group` flag to `mont task`:

When enabled, for each task ID specified, also include all tasks in that task's subgraph.

Example: `mont task a,b --group`
1. For task `a`, traverse `before` and `after` transitively to get full subgraph
2. For task `b`, do the same
3. Merge all subgraphs (deduplicate)
4. Topological sort the result
5. Open multieditor with all tasks in dependency order

This makes editing sequences of related tasks easier - you see the full context of dependencies.
