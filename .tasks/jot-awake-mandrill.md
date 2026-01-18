---
id: jot-awake-mandrill
title: Do an architecture review of the code
type: jot
---

A few things:

- We need to consolidate all of the methods for constructing an optimized task graph
    - Task graphs after being read from file system should always have transitive reduction run on them
- This should also consolidate the definition of a ready task

- We should have a commands module instead of dumping everything in main.rs
