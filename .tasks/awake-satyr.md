---
id: awake-satyr
title: Add jj disabled mode via global config
status: complete
gates:
  - architecture-validator: passed
  - test: passed
---

Add a jj configuration section to GlobalConfig with an enabled field (default true).
When disabled, jj module functions return happy-path defaults:
- is_working_copy_empty() returns Ok(true)
- has_code_changes() returns Ok(false)
- status() returns Ok(String::new())
- commit() and commit_interactive() are no-ops returning success
- working_copy_diff() returns empty PatchSet

Pass enabled flag to each jj function from call sites that have access to config.
