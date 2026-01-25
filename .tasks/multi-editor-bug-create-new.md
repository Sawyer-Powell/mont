---
id: multi-editor-bug-create-new
title: Multi editor has bug detecting tasks which should be created
status: complete
gates:
  - user-qa: passed
  - architecture-validator: passed
  - test: passed
---

Replication:
Invoked mont task with an existing id of a task. I.e

mont task jot-rousing-ruff

In that multieditor, add another task under the existing task.
Don't accept changes.
Resume editing

mont task -r

Exit editing.

The CLI says that it detected the creation of two tasks:

- jot-rousing-ruff
- my-new-task

It should be able to see that jot-rousing-ruff already exists in the task graph.

# Hypothesis

After invoking mont resume -r, we lose the context of the ids specified in the input of 'mont task'

Maybe we solve this by including the ids of the input of mont task in the comments in the header?

We need an approach that works consistently across 'mont jot' 'mont distill' and 'mont task'
