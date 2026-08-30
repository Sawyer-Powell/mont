---
id: orchestration-cli-contract
title: Define orchestration CLI and display contracts
type: jot
---

Stabilize the human and agent-facing command contract for orchestration after the underlying state model is known.

Human-readable output remains the default. Agent-facing lifecycle and inspection commands share `--format json` with stable fields for success, partial success, invalid state, ownership, revisions, filesystem effects, and recovery. JSON output must contain no ANSI escapes.

Update `mont list` and `mont status` to distinguish local status from repository-wide effective progress and show owning workspace or queue bookmark, change and commit IDs, and whether completion is in the caller's ancestry. Keep `mont ready` strictly local so completion on an unintegrated queued branch does not unblock dependents.

Deprecate `mont claude` and its synchronous process launcher while retaining only prompt or task-description facilities useful to external orchestration. When distilled, reconcile this direction with existing `change-mont-claude` work rather than extending a surface intended for removal.
