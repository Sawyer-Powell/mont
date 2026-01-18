---
id: jot-aspirant-dog
title: Rework task types
type: jot
before:
    - global-settings
---

We need to rework the structure of task types in mont

There are only two three core types of entries in mont:
- jot: For an unstructured idea that is not robust enough yet to drive an implementation
- task: A structured plan for how to implement something in the codebase, with an optional set of validators to ensure the quality of that task
- validator/gate: A task that describes how to quality control the codebase

The bug task-type and feature task type should no longer be supported in the core.

Instead of `mont new` we should change the commands to
- mont jot (we already have this)
- mont task
- mont gate

For each of these you just describe in the first argument the title of the task, and we support all the previous commands
`mont edit` works the same across all of them.

The key insight here is that `mont distill` should work across *all* task types

`mont distill` takes one task and converts into many

jots and tasks and gates are not different, they're all just markdown files, *mont* should focus on providing a consistent
core task management experience available across *all* task types.
