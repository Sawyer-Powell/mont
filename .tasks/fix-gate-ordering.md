---
id: fix-gate-ordering
title: Fix gate ordering issue when using mont prompt
status: complete
gates:
  - interview-validator: passed
  - test: passed
---

Right now, we don't enforce ordering well on gates when using mont prompt.

Going forward we need to ensure the ordering of gates by referencing:
- The order the default ones appear in the config.yml
- The order they appear in the task

Default gates should be ordered first.
