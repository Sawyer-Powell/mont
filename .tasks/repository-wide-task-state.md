---
id: repository-wide-task-state
title: Synthesize and validate repository-wide task state
type: jot
---

Add a read-only repository-wide model synthesized from every registered JJ workspace revision and every `mont/<task-id>` queue bookmark, while preserving the local `.tasks` graph as the authority for readiness and mutations.

Repository-wide synthesis must expose provenance for each task update and reject ambiguous state: `.tasks` conflicts including `config.yml`, differing concurrent edits to one task, multiple concurrent owners even when contents merge identically, malformed Mont workspace or bookmark names, contradictory workspace/task lifecycle state, and illegal descendants at lifecycle boundaries. `mont check` must remain usable in invalid states and report exact workspaces, bookmarks, revisions, paths, and task IDs with actionable repair guidance. Other commands should pause only for ambiguous or contradictory state, not recognizable resumable transitions or stale already-integrated bookmarks.

The initial implementation may inspect a speculative JJ merge created with `--no-integrate-operation`; it must not alter a physical working copy or integrate that operation.
