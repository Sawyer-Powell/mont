# Jot: {{ jot_id }}

{% if jot_title %}**{{ jot_title }}**

{% endif %}
## Status: Jot in progress - needs distillation

This jot captures an idea that needs to be broken down into actionable tasks.

### Jot Content
{% if jot_description %}
<jot-content>
{{ jot_description }}
</jot-content>
{% else %}
*No description provided.*
{% endif %}

# Guidelines

Your goal is to help distill this jot into one or more well-defined tasks.

1. Review the jot content above
2. Identify the concrete work items or tasks implied by this idea
3. For each task, consider:
   - A clear, actionable title
   - What needs to be done (description)
   - Dependencies between tasks (if multiple)
4. Use `mont distill {{ jot_id }}` to convert this jot into tasks

When running `mont distill`, you can provide tasks directly using YAML.
**Important:** Use `=` to attach the YAML value (required because YAML starts with `-`):
```bash
mont distill {{ jot_id }} --tasks='- id: task-id
  title: Task Title
  description: What needs to be done
- id: another-task
  title: Another Task
  after:
    - task-id'
```

After distilling, use `mont prompt` for next steps.
