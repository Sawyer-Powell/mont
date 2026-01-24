---
id: mont-jot-command
title: Add mont jot shortcut command
status: complete
gates:
  - architecture-validator: passed
  - test: passed
---

Add `mont jot` as a shortcut command:
- `mont jot` - opens multieditor with jot template
- `mont jot "Quick idea"` - opens multieditor with jot template, title pre-filled

Implementation: Can either be a separate CLI command that calls into task_cmd with appropriate args, or handled directly in main.rs routing.
