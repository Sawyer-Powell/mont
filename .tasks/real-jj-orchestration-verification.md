---
id: real-jj-orchestration-verification
title: Verify orchestration against real JJ repositories
type: jot
---

Build reusable integration-test coverage using temporary real JJ repositories rather than relying only on mocked command output.

The verification matrix must include nested-directory invocation; default and non-default workspaces; concurrent claims; sibling and nested workers; non-conflicting synthesis; conflicting and identical concurrent edits; local readiness with queued dependency completion; every interruption point and retry for start, done, stop, and integrate; descendant blockers; multi-revision integration without squashing; conflicted rebase recovery and `jj op restore` guidance; stale integrated bookmarks; safe orphan-directory cleanup; and stable ANSI-free JSON.

Distillation should identify the smallest reusable real-JJ test harness first, then assign scenario coverage alongside the implementation workstreams where practical rather than postponing all verification to the end.
