---
id: jj-orchestration-foundation
title: Build the JJ orchestration foundation
type: jot
---

Establish the repository and workspace primitives required by Mont's orchestration model.

The design requires a typed JJ abstraction that operates with explicit repository and workspace context rather than ad hoc subprocesses rooted at the current directory. It must support workspace discovery and add/forget operations, revision and operation identifiers, snapshots, bookmarks, ancestry and descendant queries, range rebase and abandon operations, conflict inspection, speculative operations, and recovery guidance.

Separate local graph loading and mutation from read-only repository-wide analysis. Resolve local `.tasks` from the current JJ workspace root, including when commands run from nested directories. Define Mont-managed workspace storage under the platform data directory using a repository key derived from the canonical shared `.jj/repo` path. Physical workspace directories are not durable task state.

This jot also owns deciding the JJ-safe task-ID grammar and identifying every create, parse, edit, distill, rename, and lifecycle boundary that must enforce it. Active task identities must remain stable while owned by a worker or queue bookmark.
