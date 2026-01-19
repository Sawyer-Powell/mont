---
id: introduce-shortcodes
title: Interactive task picker with fzf
status: complete
---

Operating on the output of mont-list is difficult, requiring the user to enter the full id
of the task to move forward.

## Solution

Instead of shortcodes (which have stability issues), implemented an interactive fzf picker
that launches when no task ID is provided to commands.

## Implementation

- Added `pick_task()` function in `commands/shared.rs` that shells out to fzf
- Made `id` argument optional for: `edit`, `delete`, `show`, `distill`
- When ID not provided, launches fzf with:
  - Aligned table format: `[type]  id  title`
  - Preview panel showing `mont show` output (60% width, right side)
- Errors if fzf not installed

## Usage

```bash
# Opens interactive picker
mont show
mont edit
mont delete
mont distill

# Still works with explicit ID
mont show my-task-id
```
