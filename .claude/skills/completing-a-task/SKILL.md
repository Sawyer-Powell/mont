---
name: mont-task
description: Instructions for how to complete a task with mont
---

When given a task to work on by task id first:
!!Read the instructions below and create a todo list for yourself!!

- Grep inside of the .tasks folder to find the task with the appropriate id
- Mark the task as in_progress by setting the `in_progress` value to 1 in the yaml frontmatter, or incrementing it if it exists, and has not been modified in the current jj revision
- Read the task
- Conduct an interview with the user to confirm implementation details.
- Implement the task
- When implementation is complete, check the `validations` list in the yaml, if set, and find the corresponding task files (by id) in the .tasks folder that correspond to the validation tasks
- Run the validation tasks, resolve issues if validations fail
- Conduct an interview with the user to confirm implementation
- If user gives green light, use `jj commit -m ...` to record what you worked on, and move to a new jj revision
