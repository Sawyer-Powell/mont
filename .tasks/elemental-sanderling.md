---
id: elemental-sanderling
title: Mont done needs better semantics
type: jot
---

Right now mont done reads from the current diff to find the active task. This is kinda ridiculous
since tasks are marked as in progress in their frontmatter. We should just query the taskgraph
for the in progress tasks directly instead of the revision's diff.
