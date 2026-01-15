---
id: mont-check
title: Implement mont check command, and internals
parent: cli-commands
validators:
    - test
    - interview-validator
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

Simple small collection of tests against these functions.
