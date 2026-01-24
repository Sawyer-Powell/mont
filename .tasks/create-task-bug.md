---
id: create-task-bug
title: Bug when creating a task with editor
status: complete
gates:
  - architecture-validator: passed
  - test: passed
---

Probably occurs for 'mont task' mont jot and mont gate

When creating one of these the yaml that appears in the editor
includes just

# ---
# id: 
# ---

If you add a title and then save, the id is an empty string and creates
a task ".md" in the .tasks folder. 

Better would be to not include the id in the default frontmatter when creating a new task, jot, gate
when running the corresponding commands. That way the user can add one manually, or we just
proceed with the default of automatically creating the id in the MontContext

We also need handling to forbid empty string ids.
