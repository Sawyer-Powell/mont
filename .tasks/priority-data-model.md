---
id: priority-data-model
title: Add priority field to tasks and jots
status: complete
gates:
  - test: passed
  - user-qa: passed
---

Add an optional priority field for tasks and jots, backed by a `Priority` enum
ordered `crit > high > med > low`. YAML values use the lowercase names exactly.
An omitted priority remains unset and ranks below `low`; existing records must
continue to parse without migration. Gates do not support priority.

Update parsing and `Task::to_markdown()` in `src/context/task.rs`, serializing
the field only when set. Ensure priority-only multieditor changes are detected
and priority can be changed through `TaskPatch`.

Definition of done: all four values and omission parse and round-trip for both
tasks and jots, invalid values and priority on gates are rejected, and tests
cover priority-only editor and patch updates.
