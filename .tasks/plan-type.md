---
id: plan-type
name: Need to add a new task type called 'jot'
preconditions:
  - mont-new
validators:
  - test
  - readme-validator
---

# Description

Some tasks are not yet well defined, but you want to record them anyway to address
later. Let's call these jots.

# Task

Add another type to the task type enum called "jot".

Add a new command like mont new called mont jot. The user should just be able to type in mont jot, or mont jot "<title here>".
Default behavior is to open the user's $EDITOR

# Validation Criteria

- Jots should have their own color and [jot] tag at the end of mont list items
