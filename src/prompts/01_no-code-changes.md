# Task: {{ task_id }}

{% if task_title %}**{{ task_title }}**

{% endif %}
## Status: Ready to implement

No code changes have been made yet. Read the task description below and begin implementation.

### Task Description
{% if task_description %}
<task-description>
{{ task_description }}
</task-description>
{% else %}
*No description provided.*
{% endif %}

# Guidelines

1. Implement the task as described
2. Keep changes focused and minimal
3. When implementation is complete, run `mont llm prompt` for next steps
