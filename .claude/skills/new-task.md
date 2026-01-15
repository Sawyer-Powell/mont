# New Task Skill

Create a new mont task file in `.tasks/`.

## Task File Format

```yaml
---
id: <kebab-case-id>
title: <short title>
parent: <parent-task-id>  # optional, hierarchical grouping
preconditions:
  - <task-id>  # optional, must complete before this task starts
validations:
  - <validator-id>  # optional, validators that must pass
validator: false  # true if this is a reusable validator task
complete: false  # optional, tracks completion status
---

<description in markdown>
```

## Rules

1. **id**: Required, unique, kebab-case (e.g., `add-jj-integration`)
2. **parent**: Optional single parent task this belongs under
3. **preconditions**: Tasks that must complete before this can start
4. **validations**: Validator tasks that verify this task's work
5. **validator**: If true, task cannot have preconditions
6. **complete**: Optional, defaults to false

## Process

1. Ask user for task details if not provided:
   - What is the task?
   - Does it depend on other tasks (preconditions)?
   - Does it belong under a parent task?
   - What are the acceptance criteria?

2. Generate an appropriate kebab-case id from the title

3. Write the task file to `.tasks/<id>.md`

4. Show the user the created file path
