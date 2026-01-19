---
id: mont-done
title: Implement mont done command
status: complete
after:
  - mont-start
gates:
  - test: passed
---

Only works if the revision has an in-progress task in the change list,
otherwise warnings are thrown or the user must provide a task id
explicitly.

Will fail if a tasks validators have not been marked as successful, this resolves
against default global validators too.

Once the task is marked as complete, mont will open an editor
for you to modify your revision's commit message, then move you
to a new revision

Use AskQuestion to nail down implementation details with me, then update this
file with that info.
