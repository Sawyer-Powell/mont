---
id: mont-new
title: Implement mont new command
parent: cli-commands
---

Implement the `mont new` CLI command to create a new task interactively.

# Behavior

1. Prompts for task details:
   - Title (required)
   - Parent task (optional)
   - Preconditions (optional, comma-separated)
   - Description (optional, opens editor or inline)
2. Generates kebab-case ID from title
3. Writes task file to `.tasks/<id>.md`
4. Prints created file path

# Implementation

- Use dialoguer or similar for interactive prompts
- Validate parent/precondition IDs exist
- Generate ID: lowercase, replace spaces with hyphens

# Acceptance Criteria

- Creates valid task file
- ID is unique (check before writing)
- Validates references exist
