---
id: priority-effective
title: Compute effective priority from dependency graph
after:
  - priority-data-model
---

Add an `effective_priority()` graph API for both tasks and jots. A record's
effective priority is the maximum of its own priority and the priorities of all
incomplete tasks and jots it transitively blocks. Priority propagates across
task-to-task, task-to-jot, jot-to-task, and jot-to-jot edges represented by
either `before` or `after` frontmatter.

Use `crit > high > med > low > unset`. Gates and completed records do not
contribute inherited priority. Preserve the graph's existing cycle validation.

Definition of done: tests cover chains, branches, both relationship forms,
every mixed task/jot direction, unset priority, gates, and completed records.
