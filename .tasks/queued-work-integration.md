---
id: queued-work-integration
title: Integrate queued worker revisions and clean workspace data
type: jot
---

Add queue consumption and filesystem cleanup around completed `mont/<task-id>` bookmarks.

`mont integrate <task-id>` runs from any receiving workspace, determines the complete worker-only revision range ending at the queue bookmark, rebases that full range onto the caller's current revision without squashing, advances the caller to a fresh empty child of the rebased tip, and removes the bookmark. It must report conflicts and recovery metadata using the pre-command JJ operation ID. A bookmark already contained in the caller's ancestry is stale rather than repository corruption and should block reintegration until explicitly removed.

`mont cleanup` removes only directories under Mont's managed workspace root whose JJ workspaces are no longer registered. It must never delete a registered workspace. Integration and cleanup must follow the shared resumable partial-operation contract and support sibling and nested worker topologies without recording parent-agent relationships outside JJ ancestry.
