---
id: graph-with-pattern
title: Replace graph() guard with with_graph() callback pattern
---

Change the `MontContext::graph()` API to prevent deadlocks by design.

## Current Problem

`ctx.graph()` returns a `RwLockReadGuard` that can be held across calls to methods needing write access, causing deadlocks:

```rust
let graph = ctx.graph();  // Acquires read lock
// ... use graph ...
ctx.update(id, task)?;    // DEADLOCK - needs write lock
```

## Proposed Solution

Replace with a callback pattern that ensures the lock is always released:

```rust
// Before:
let task = ctx.graph().get(id).cloned();

// After:
let task = ctx.with_graph(|graph| graph.get(id).cloned());
```

## Benefits

1. **Compile-time safety**: Lock scope is limited to the callback
2. **No deadlocks**: Lock is always dropped after callback returns
3. **Explicit intent**: Makes lock boundaries visible in code

## Implementation

1. Add `with_graph<R>(&self, f: impl FnOnce(&TaskGraph) -> R) -> R`
2. Deprecate `graph()` method
3. Migrate all call sites
4. Remove `graph()` after migration
