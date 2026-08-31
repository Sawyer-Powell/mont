---
id: priority-cli
title: Add priority flags to CLI commands
after:
  - priority-data-model
---

Add `--priority <low|med|high|crit>` to `mont task` and `mont jot`. `mont task`
is also the existing edit command: with IDs, the flag updates every selected
task or jot; without IDs, it seeds the new-record template. Support quick jot
creation as well as the editor workflow. Reject priority for gate records.

Omitting the flag preserves an existing record's priority and creates new
records with priority unset. Keep `--patch` support from the data-model task.

Definition of done: CLI tests cover task and jot creation and editing, quick
jots, multiple selected IDs, omission, `crit`, invalid values, and gates.
