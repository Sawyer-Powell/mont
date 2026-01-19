---
id: replace-mont-new
title: Rename mont new to mont task and mont gate
status: complete
gates:
  - interview-validator: passed
  - test: passed
---

Mont new should no longer be used, instead we should completely remove it (breaking change)
and replace it with a mont task and mont gate command.

Both of these commands work just like mont new, but the first argument is the title (not the id),
and it just changes the default type that is assigned when creating the task.

I.e.
`mont task "Do this thing"` creates a new task with a random id with that title

Same for mont gate, but with type gate by default.
