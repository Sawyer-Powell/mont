---
id: done-detect-from-graph
title: Detect in-progress task from graph instead of diff
status: inprogress
gates:
  - user-qa
  - test
---

Change `detect_in_progress_task()` in done.rs to query the task graph
directly instead of parsing jj diff output.

Current: Parse jj diff → find .tasks/*.md → look for "status: inprogress" text
New: Query graph.values().filter(|t| t.is_in_progress())

Benefits:
- Simpler, more direct approach
- Works even if task was started in a previous revision
- No dependency on jj diff parsing
