---
title: Form graph out of tasks
---

Update the structure of tasks to not include "subtasks" but
a parent id, or a vec of parent task ids.

We also need to add a new parsing requirement to validator tasks.
Do this in a unit test on the parse function.
Validator tasks must not have preconditions, they can have parents.

Using a vec of Tasks, ensure a few rules:

- All parent tasks point to a valid task
- All preconditions point to a valid task
- All validations point to valid tasks that are marked as validators
- All validations point to a validator that does not have a parent, they must be root validators
- The graph of tasks forms a DAG

Ask me follow up questions if requirements are unclear, then update this markdown file with details.

# Acceptance criteria

Unit tests that cover all rules above, both the happy and failure paths.

Do an interview with me once things are written to confirm all details.
