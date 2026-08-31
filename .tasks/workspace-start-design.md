---
id: workspace-start-design
title: Design mont workspace start
type: jot
after:
  - repository-wide-task-state
---

Design `mont workspace start <task-id> [--revision <revset>]` on top of the repository-wide state architecture.

The design must define shared startability with in-place `mont start`, selected-revision and multi-parent semantics, ancestry-sensitive dependencies, workspace naming and managed data paths, non-empty caller behavior, workspace creation and worker-local task mutation, snapshots, output and recovery metadata, concurrent claims, and the complete table of resumable versus contradictory partial states. It must preserve the namespaced managed lifecycle rather than changing unnamespaced `mont start`.

Definition of done: update the architecture documentation as needed and produce dependency-ordered implementation tasks, each independently verifiable against temporary real JJ repositories and sized for autonomous execution.
