---
id: workspace-cleanup-design
title: Design mont workspace cleanup
type: jot
after:
  - workspace-done-design
---

Design `mont workspace cleanup` as a high-value convenience for removing physical directories left after managed workspaces are forgotten.

Cleanup may remove only directories inside Mont's managed workspace root whose corresponding JJ workspaces are no longer registered. It must never remove registered, unrelated, or out-of-root paths, must define canonicalization and symlink safety, and must obey repository-wide invalid-state mutation gating. Physical directories are not durable task or ownership state.

Definition of done: specify discovery, safety, output, retry, and failure semantics and produce bounded implementation tasks with temporary data-root and real-JJ verification.
