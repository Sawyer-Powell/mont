There is currently no task marked as in progress.
{% if has_uncommitted_changes %}
However, this revision has uncommitted changes.

Please review the current changes with `jj diff`.
Identify the work done and ask the user if they'd like to commit their changes, suggest a commit message.

If they say yes:
1. Commit the changes with `jj commit -m "message"`
2. Use `mont llm prompt` to see next steps
{% else %}
1. Use `mont ready` to see a list of available tasks for work.
2. Identify ones that look well defined and easy
3. Use `mont show <id>` to see more details about each task.
4. Ask the user if they'd like to start this task (or another one)
5. If they approve or choose a task, use `mont start <task id>` to begin work
6. Use `mont llm prompt` for next steps
{% endif %}
