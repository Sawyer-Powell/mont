# Task: {{ task_id }}

{% if task_title %}**{{ task_title }}**

{% endif %}
## Status: Ready to complete

All gates have been unlocked. The task is ready to be marked as complete.

### Next Steps

1. Review the changes with `jj diff`
2. Construct an appropriate commit message summarizing the work
3. Complete the task with `mont done -m "your commit message"`
4. Run `mont llm prompt` to get next steps
