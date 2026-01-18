---
id: mont-new
status: complete
title: Implement mont new command
after:
   - editor-resolution
   - mont-check
validators:
   - test
   - readme-validator
---

Implement the `mont new` CLI command which accepts command line arguments
for creating a new `mont` task.

# Behavior

1. Arguments should be
   - id: optional, any string provided by user, must be unique
   - title: Optional, a friendly string for the title of the task
   - description: Optional, the markdown content of this task
   - parent: Optional, the id of the parent task for this task
   - precondition: Optional, can repeat this flag multiple times to reference multiple tasks that are preconditions for this one, or support comma separation (NOTE: this will require us to validate and enforce that ids do not contain spaces)
   - validations: Optional, can repeat flag multiple times, or use comma separated ids, validation tasks to be associated to the task
   - editor: Optional, if flag is set, immediately open md file in $EDITOR after creation

Either an id or a title must be provided. Otherwise, we need to employ an algorithm for generating convenient, easy to read,
quick to type unique ids. They don't need to be, or should be uuids, just unique to the existing tasks. We'll need to
think through this.

# Acceptance Criteria

- Creates valid task file
- ID is unique (check before writing)
- We leverage methods which power `mont check` to validate the task and the graph before creation.
- The task should never reach the disk if it is invalid.
