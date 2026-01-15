---
id: add-jj-lib
title: Add jj-lib integration
---

Add jj-lib as a dependency and create basic integration for working with JJ repositories.

# Changes

- Add `jj-lib` to Cargo.toml dependencies
- Create `src/jj.rs` module with basic JJ operations:
  - Open a workspace/repo
  - Create a new revision with a description
  - Get current revision info

# Notes

- jj-lib API may be unstable, check docs at https://docs.rs/jj-lib
- Focus on minimal viable operations needed for `mont start`

# Acceptance Criteria

- Can open the current JJ workspace
- Can create a new revision with a custom description
- Basic error handling with thiserror
