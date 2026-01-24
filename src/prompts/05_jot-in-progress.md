# Jot: {{ jot_id }}

{% if jot_title %}**{{ jot_title }}**

{% endif %}
## Status: Jot in progress - REQUIRES DISTILLATION

> **IMPORTANT**: Jots are ideas, not tasks. You MUST convert this jot into concrete tasks BEFORE doing any implementation work. Do NOT write code or make changes to implement this jot directly.

### Jot Content
{% if jot_description %}
<jot-content>
{{ jot_description }}
</jot-content>
{% else %}
*No description provided.*
{% endif %}

# Required Workflow

## Step 1: Analyze and plan

1. Review the jot content above
2. Check existing tasks with `mont list` to understand context and potential dependencies
3. Break down the jot into concrete, actionable tasks
4. Present your task breakdown to the user for confirmation

## Step 2: Distill the jot (after user confirms)

Use `mont distill --stdin` to convert the jot into tasks:

```bash
mont distill {{ jot_id }} --stdin <<'EOF'
---
id: task-id-here
title: Clear actionable title
gates:
  - user-qa
  - test
---
Detailed description of what needs to be done.
Acceptance criteria go here.

---
id: second-task
title: Another task
after:
  - task-id-here
gates:
  - user-qa
---
Description of the second task.
EOF
```

This will:
- Validate the new task definitions
- Delete the jot automatically
- Create the new tasks
- Commit the changes

## Step 3: Continue with the new tasks

After distilling, run `mont prompt` to get instructions for working on the newly created tasks.

# Why this matters

Jots capture rough ideas. Tasks have:
- Clear acceptance criteria
- Gates for validation (tests, user QA, etc.)
- Dependencies that mont tracks

Implementing jots directly bypasses these safeguards and leads to incomplete or unvalidated work.
