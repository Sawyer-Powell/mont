---
id: canonical-filename-task-ids
title: Make filenames canonical task IDs
type: jot
---

Plan the breaking change that makes the `.tasks/<id>.md` filename stem the sole persisted task identity and removes the redundant `id` field from frontmatter.

IDs use strict lowercase kebab-case and must also be safe in JJ workspace names `mont/<id>` and bookmarks `mont/<id>`. The design must cover parsing and serialization, create/edit/rename/distill flows, generated IDs, collision handling, reference and `config.yml` rewrites, diagnostics, and migration of this repository's existing files and tests.

Definition of done: settle the file/identity contract and produce small implementation tasks that enforce it across every existing ID boundary. Managed workspace commands added later must consume the same validator.
