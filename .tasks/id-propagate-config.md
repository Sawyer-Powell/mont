---
id: id-propagate-config
title: Propagate ID changes to config.yml
---
When renaming a task ID, also update references in `config.yml`:

Currently, `ctx.update()` rewrites references in other task files (before/after/gates fields).
It should also check if the renamed task is referenced in `config.yml` `default_gates` list.

Example:
- User renames gate `code-review` to `pr-review`
- `config.yml` has `default_gates: [code-review, tests]`
- After rename, should become `default_gates: [pr-review, tests]`

Implementation location: `src/context/mod.rs` in the `update()` or `rewrite_references()` function.
