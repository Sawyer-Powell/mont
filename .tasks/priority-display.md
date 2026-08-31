---
id: priority-display
title: Show computed priority in all task displays
after:
  - priority-effective
---

Show effective priority for tasks and jots in `mont list`, `mont ready`,
`mont show`, `mont status`, and the fzf picker. This requires updating the
shared renderer plus command-specific output in `commands/show.rs`,
`commands/status.rs`, and `commands/shared.rs`. Gates have no priority
indicator; unset priority has no indicator.

Always display the computed priority rather than the stored value. Sort each
existing task or jot section in `mont list` and `mont ready` by effective
priority descending, then retain its existing state and ID tie-breakers.
Preserve dependency ordering in graph output and keep picker/extension parsing
compatible with the current marker, type, and ID positions.

Definition of done: display tests cover stored and inherited priority for both
tasks and jots, unset priority, all listed output paths, stable tie ordering,
and parseable picker/extension output.
