---
id: mont-claude-fixes
title: Fix picker dispatching for mont claude
status: complete
gates:
  - test: passed
  - interview-validator: passed
---

Right now the picker dispatches before we know if we can launch mont claude.

Ensure that we do validation for mont claude *before* we launch the picker.

Also, --ignore should ignore this pre-validation
