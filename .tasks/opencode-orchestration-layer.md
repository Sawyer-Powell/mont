---
id: opencode-orchestration-layer
title: Compose Mont primitives into OpenCode orchestration
type: jot
after:
  - workspace-cleanup-design
---

After repository-wide state and the managed `start`, `done`, and `cleanup` contracts stabilize, design OpenCode skills and agents that coordinate external workers using Mont's human-readable CLI primitives.

Mont must not launch, monitor, or own agent processes. The external layer may consume task descriptions, lifecycle output, queue bookmarks, gates, provenance, and recovery guidance. It should preserve JJ ancestry as orchestration structure and gates as per-task statuses.

Automatic agent launching inside Mont, a daemon or lock service, automatic integration or squashing, automatic rollback, and gate-agent provenance enforcement remain outside the Mont MVP.
