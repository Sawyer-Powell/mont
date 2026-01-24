---
id: mont-claude-updates
title: Updates to mont llm claude
status: complete
gates:
  - architecture-validator: passed
  - test: passed
---

Let's do a few updates here:
1. Remove llm namespace, claude and prompt should be just mont claude and mont prompt
-> Here you need to grep all instances of mont llm and systematically update them, especially in the default prompt templates
2. Update claude command to by default require a task id, dispatch the picker if not provided.

Here's how it should work:
- `mont claude <task id>` is used and there are changes in the current revision, and that task is not in progress,
fail and tell the user to commit their changes, or run `mont claude <task id>` against the task that is in progress.
- Allow the user to use a -i or --ignore flag that will run the mont prompt anyway and start claude up to take things from there.
