---
id: workspace-done-design
title: Design mont workspace done
type: jot
after:
  - workspace-start-design
---

Design `mont workspace done` for a worker created by `mont workspace start`.

The command runs in the owning workspace, verifies repository coherence, ownership, task state, gates, and descendant constraints, marks the task complete, snapshots the completed tip, creates bookmark `mont/<task-id>`, and forgets the JJ workspace without deleting its physical directory. It must not invoke an interactive commit, rebase or squash revisions, create an empty child, or integrate completed work. Integration belongs to ordinary JJ operations and is outside Mont's MVP.

The design must enumerate resumable and contradictory partial states, operation-ID recovery guidance, bookmark/workspace name reuse, and interaction with the existing in-place `mont done` behavior.

Definition of done: settle the state machine and human contract, reconcile or supersede `innocuous-groundhog`, and produce bounded dependency-ordered implementation tasks with real-JJ interruption and retry verification.
