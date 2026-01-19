---
id: mont-start
title: Implement mont start command
after:
  - add-jj-lib
  - in-progress-status
---

Implement the `mont start <task-id>` CLI command.

1. Takes a task ID as argument
2. Validates the task exists in MontContext
3. Ensures that we're on an empty JJ revision
4. If we're not on an empty JJ revision, and the diff for this revision contains a task with a status:complete change, we create a new revision
5. If we're not on an empty JJ revisions, and the diff does not contain the above, we error and tell the user to commit and move to a fresh change
6. Once we're confiremd on a fresh revision, In MontContext run an update on the task to be status inprogress

From here I would like to pass off to a claude code instance with instructions for how
to complete the task. Please conduct an interview with me to determine the best way to
get claude code to work on the task.

I want claude code to read the task, and then follow a precise list of instructions
for how to complete the task.

Later on we'll provide a 'mont done' command that marks the task as done and commits
