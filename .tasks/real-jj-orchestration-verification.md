---
id: real-jj-orchestration-verification
title: Design a reusable real-JJ test harness
type: jot
---

Design reusable integration-test support around temporary real JJ repositories. This is a known prerequisite for repository-wide state and managed workspace implementation.

The harness must support isolated repositories and data roots, default and non-default workspaces, nested-directory CLI invocation, divergent and merged revisions, `.tasks` edits and conflicts, bookmarks, operation IDs, workspace registration and forgetting, ancestry assertions, and deterministic construction of partial lifecycle states. Tests must be parallel-safe and must not mutate process-global environment from in-process tests.

Definition of done: settle the harness API and ownership boundaries, prove uncertain JJ behavior with focused spikes where useful, and produce one or more implementation tasks capped at small/medium scope.
