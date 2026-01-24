---
id: mont-delete-multi-id
title: Support multiple IDs in mont delete
---

Update `mont delete` to support multiple task IDs with `?` picker support.

Current: `mont delete <id>` - single ID only
New: `mont delete id1,id2,?` - multiple IDs, `?` spawns picker

Implementation:
1. Change `id: Option<String>` to `ids: Vec<String>` in CLI args
2. Use `resolve_ids()` from shared.rs to expand `?` placeholders
3. Show confirmation listing all tasks to be deleted
4. Delete all in a single transaction for atomicity

Example usage:
```bash
mont delete old-task,stale-task
mont delete ?              # pick one via fzf
mont delete ?,?            # pick two via fzf
mont delete task-a,?       # delete task-a and one picked task
```
