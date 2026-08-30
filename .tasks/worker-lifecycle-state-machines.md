---
id: worker-lifecycle-state-machines
title: Implement resumable worker lifecycle state machines
type: jot
---

Redesign `mont start <task-id>`, `mont done`, and `mont stop` around JJ workspace ownership named `mont/<task-id>`.

`start` validates readiness against the caller's local graph, claims the unique workspace name, creates a descendant working-copy revision in Mont-managed storage, marks the task in progress there, forces a snapshot, and returns workspace, revision, parent, and recovery metadata. It must resume a recognizable partial start after workspace creation.

`done` runs in the owning worker, verifies gates and rejects descendant revisions, marks the task complete and snapshots it, leaves the completed worker revision intact, creates the matching queue bookmark, and forgets the workspace without deleting its directory. It must not invoke an interactive commit, rebase, or create an empty child.

`stop` rejects descendants, abandons worker-created revisions back to the branch-point state, forgets the workspace, and leaves directory cleanup separate. It does not need to persist a stopped task status.

All three commands must be idempotent across recognizable interruption points. They capture the pre-command JJ operation ID, continue unambiguous partial transitions, avoid automatic rollback, distinguish filesystem leftovers from JJ state, and explain `jj op restore` recovery. When this jot is distilled, reconcile or supersede the existing `innocuous-groundhog` transactional-done task rather than duplicating it.
