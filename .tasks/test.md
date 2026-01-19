---
id: test
title: Run tests
type: gate
---

Run
```bash
cargo build
cargo test
```

Then 
```bash
cargo clippy
```

If you see issues with clippy, you must immediately fix them.

Warnings are errors

# You can unlock gate when:

All commands yield no warnings
