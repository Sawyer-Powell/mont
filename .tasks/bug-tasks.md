---
id: bug-tasks
title: Allow tasks to be marked as bugs
---

Add support for marking tasks as bugs to distinguish defect fixes from feature work.

# Motivation

Bugs have different characteristics than feature tasks:
- Often higher priority
- May need different tracking/metrics
- Useful for filtering and reporting

# Changes

## Task struct
- Add `bug: bool` field (default false) to Task in `task.rs`

## Display
- Show bugs with a distinct marker (maybe `✗` or `⚠`)
- Consider showing bugs prominently (before regular tasks?)

# Example

```yaml
---
id: fix-login-crash
title: Fix crash on login with empty password
bug: true
---

App crashes when user submits login form with empty password field.
```

# Acceptance Criteria

- Can parse tasks with `bug: true`
- Display shows bug marker distinctly
- Bugs visually distinguishable from regular tasks
