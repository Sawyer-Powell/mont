---
id: editor-resolution
title: Resolve which text editor the user wishes to use
status: complete
validators:
  - interview-validator
---

Many commands going forward will have options to open tasks directly in the user's editor.

We need a nice function for determining the user's configured editor and launching it.
- If a specific string is provided, we try to run that command line utility. I.e. the input to the function is "nano"
- Look for $EDITOR
- If no $EDITOR is found, we should default to an appropriate system text editor depending on OS. Nano for macos and linux users, notepad for windows users?

## Acceptance

A single tight function that we can validate through largely manual testing.
