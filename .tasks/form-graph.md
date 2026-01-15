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

# Implementation Details

## `task.rs` changes
- Replace `subtasks: Vec<String>` with `parents: Vec<String>`
- Add `ValidatorWithPreconditions` error variant to `ParseError`
- Update `parse` function to reject validators that have preconditions
- Update doc examples to use `parents` instead of `subtasks`

## New `src/graph.rs`
- `TaskGraph` type alias: `type TaskGraph = HashMap<String, Task>`
- `GraphError` enum (thiserror) with variants:
  - `InvalidParent { task_id, parent_id }` - parent doesn't exist
  - `InvalidPrecondition { task_id, precondition_id }` - precondition doesn't exist
  - `InvalidValidation { task_id, validation_id }` - validation points to non-validator
  - `ValidationNotRootValidator { task_id, validation_id }` - validator has parents
  - `CycleDetected` - graph has cycles
- `form_graph(tasks: Vec<Task>) -> Result<TaskGraph, GraphError>` function
- DFS-based cycle detection (3-color algorithm)

## No new dependencies
Using simple `HashMap<String, Task>` instead of petgraph for easier diffing in future.

# Acceptance criteria

Unit tests that cover all rules above, both the happy and failure paths.

Do an interview with me once things are written to confirm all details.
