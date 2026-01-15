---
id: mont-new
title: Implement mont new command
parent: cli-commands
preconditions:
   - mont-check
validators:
    - test
---

Implement the `mont new` CLI command which accepts command line arguments
for creating a new `mont` task.

# Behavior

1. Arguments should be
   - id: Required, any string provided by user, must be unique
   - title: Optional, a friendly string for the title of the task
   - description: Optional, the markdown content of this task
   - parent: Optional, the id of the parent task for this task
   - precondition: Optional, can repeat this flag multiple times to reference multiple tasks that are preconditions for this one
   - validations: Optional, can repeat flag multiple times, validation tasks to be associated to the task
   - editor: Optional, if flag is set, immediately open md file in $EDITOR after creation

# Implementation

The first thing that we need to build

# Acceptance Criteria

- Creates valid task file
- ID is unique (check before writing)
- Validates references exist
