---
id: skip-empty-jj-commits
title: Skip jj commits when working copy is empty
gates:
  - user-qa
  - test
---

Update `jj::commit()` to detect empty working copy and return early with success.

## Changes

In `src/jj.rs`, update the `commit()` function to:
1. Call `is_working_copy_empty()` before attempting commit
2. If empty, return `Ok(CommitResult { ... })` with a note that nothing was committed
3. Existing callers don't need changes - they'll see success either way

## Rationale

When `.tasks/` is gitignored/excluded:
- `jj diff` shows no changes for .tasks-only modifications
- `is_working_copy_empty()` returns true
- Attempting to commit would be a no-op anyway

This makes the behavior automatic - no new config flags needed. The existing `jj.enabled` flag retains its meaning (enable/disable jj entirely).

## Testing

- Test that commit returns success when working copy is empty
- Test that commit still works normally when there are changes
