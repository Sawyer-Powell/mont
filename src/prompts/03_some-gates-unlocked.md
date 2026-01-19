# Task: {{ task_id }}

{% if task_title %}**{{ task_title }}**

{% endif %}
## Status: Verification in progress

Gates passed: {{ gates_unlocked }}
Gates remaining: {{ gates_pending }}

{% if gate_id %}
### Next Step: Verify gate `{{ gate_id }}`

{% if gate_title %}**{{ gate_title }}**

{% endif %}
{% if gate_description %}
{{ gate_description }}
{% endif %}

Once verified, mark the gate as passed:
`mont unlock {{ task_id }} --passed {{ gate_id }}`

Once the gate is passed, run `mont prompt` again.
{% endif %}
