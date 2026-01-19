---
id: priority-effective
title: Compute effective priority from dependency graph
after:
  - priority-data-model
---

Add effective_priority() function that walks the graph.
A task's effective priority = max(own priority, priority of all tasks it blocks).
This propagates through before/after relationships transitively.

