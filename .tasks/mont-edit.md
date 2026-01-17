---
id: mont-edit
title: Add a mont edit command
---

# Description

Add a command, mont edit

- Should have the same flags as `mont new`, even better if we can enforce this in the type system in Rust
- If the id a task is updated through `mont edit`, we propagate those updates to all other tasks that depended on it
