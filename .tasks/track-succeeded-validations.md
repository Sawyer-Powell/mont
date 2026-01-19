---
id: track-succeeding-validations
description: Add a mechanism in the yaml frontmatter to record when validations pass
status: complete
---

Force the LLM to mark validations as having passed inside the task markdown.
Need to find some good structure for this.

If you're a coding agent reading this, we need to brainstorm this first.

# Implementation Decisions (from interview)

- **Format**: Mixed list in YAML where strings are pending, objects track status
- **Example**:
  ```yaml
  gates:
    - val1          # pending (string format)
    - val2: passed  # passed
    - val3: failed  # failed
    - val4: skipped # skipped
  ```
- **Statuses**: `passed`, `failed`, `skipped`
- **Consistency checking**: Handled by `mont check` command, not at parse time
- **Revision tracking**: Not needed - jj revision history captures when changes were made
