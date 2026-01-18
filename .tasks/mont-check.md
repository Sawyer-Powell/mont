---
id: mont-check
title: Implement mont check command, and internals
status: complete
validators:
    - test
    - readme-validator
---

We need a command that validates the integrity of the .tasks graph.
We have some set of validations that are run when executing `mont list`,
the internals of graph validation need to be moved to a `validations.rs` file.

The validator should be exposed by the `mont check` command which does a
full check of the graph.

If you pass in `mont check <id>`, the validator will just check that specific
task.

The CLI are thin layers over the internal functions that power these.
Individual task checks and full graph checks are two functions that will be
re-used a lot by other commands.

Conduct an interview with me using AskQuestion to confirm specifications, then modify this task
with changes to the requirements, then implement.

# Acceptance Criteria

A new file `validations.rs` with at least two functions, one for full graph validation
and another for single task validation.

Validation function for single tasks needs to also validate that the other tasks it references in the yaml also exist and are valid

Simple small collection of tests against these functions.

# Implementation Decisions (from interview)

- **Error handling**: Fail fast on first validation error
- **Single task validation**: Validates task + checks that referenced tasks (parent, preconditions, validations) exist and are valid
- **Output**: Simple pass/fail - success message on pass, error message on fail
- **Exit codes**: Use appropriate exit codes (0 for success, non-zero for errors)
- **Code organization**: Move existing validation logic from `graph.rs` to `validations.rs`, refactor `form_graph` to use new functions
