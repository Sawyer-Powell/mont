---
id: mont-show
title: Implement mont show command
parent: cli-commands
validations:
  - test
---

Implement the `mont show <task-id>` command to display a single task's details.

# Behavior

1. Takes a task ID as argument
2. Reads the corresponding `.tasks/<id>.md` file
3. Pretty prints the task to console with:
   - Title (highlighted)
   - Status (complete/incomplete)
   - Parent (if any)
   - Preconditions (if any)
   - Validations (if any)
   - Description (the markdown content)

# Example Output

```
mont-show
Implement mont show command

Status: incomplete
Parent: cli-commands
Validations: test

---

Implement the `mont show <task-id>` command to display a single task's details.
...
```

# Acceptance Criteria

- Shows task details in a readable format
- Errors clearly if task ID not found
- Uses owo-colors for styling
