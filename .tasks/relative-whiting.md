---
id: relative-whiting
title: Design dependency semantics for jots
type: jot
---

Design how jots participate in Mont's dependency graph so broad design work can be scheduled before it is distilled into executable tasks.

The current graph validates jot dependencies, but `mont list` partitions tasks and jots before rendering and loses cross-type edges. The design must define mixed task/jot ordering in `list`, whether and how blocked jots appear in `ready`, and what happens to downstream dependencies when an upstream jot is distilled and deleted. In particular, decide whether dependencies transfer to all replacement tasks or use another explicit rule.

Definition of done: document the intended jot dependency and distillation semantics, identify affected graph/render/mutation paths, and produce bounded implementation tasks with tests for jot-to-jot and jot-to-task chains.
