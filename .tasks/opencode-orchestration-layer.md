---
id: opencode-orchestration-layer
title: Compose Mont primitives into OpenCode orchestration
type: jot
---

After Mont's lifecycle and JSON contracts stabilize, design OpenCode skills and agents that launch and coordinate worker and validator processes using Mont's CLI primitives.

Mont itself must not launch, monitor, or own agent processes. The external layer should consume structured start, completion, integration, gate, provenance, and recovery metadata; support hierarchical orchestration through JJ ancestry and queue bookmarks; and preserve gates as per-task statuses rather than independent workspace-owning nodes.

Automatic agent launching and monitoring, gate-agent provenance enforcement, a daemon or ACID claim service, automatic squash policy, and automatic rollback remain outside the Mont MVP. This jot should be distilled only after the CLI contract is concrete enough to avoid coupling agents to unstable output.
