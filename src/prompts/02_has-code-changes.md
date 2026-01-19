# Task: {{ task_id }}

{% if task_title %}**{{ task_title }}**

{% endif %}
## Status: Implementation of {{ task_id }} is in progress

Code changes have been made. Review the current implementation carefully using `jj diff` against the task requirements.

### Task Description

{% if task_description %}
<task-description>
{{ task_description }}
</task-description>
{% else %}
*No description provided.*
{% endif %}

{% if gate_id %}
## If the implementation is complete: Next step is to verify the code against gate: `{{ gate_id }}`

{% if gate_title %}**{{ gate_title }}**

{% endif %}
{% if gate_description %}
{{ gate_description }}
{% endif %}

Once all criteria are met for verification, mark the gate as passed:
`mont unlock {{ task_id }} --passed {{ gate_id }}`

Then, get the next step using
`mont llm prompt`
{% endif %}
