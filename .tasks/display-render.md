---
id: display-render
title: Wiring + final output
before:
  - display-refactor
after:
  - display-symbols
validations:
  - test
complete: true
---

Wire up the full pipeline and produce final output.

# Pipeline

1. Call layout functions to build grid with tasks
2. Call routing to add connection cells
3. Call symbol conversion for each connection cell
4. Render each row: prefix + marker + task info

# Task Line Formatting

Port from current display.rs:
- Marker selection (colored circles based on state)
- ID display with color
- Title truncation (MAX_TITLE_LEN = 60)
- Type suffix ([epic], [bug], etc.)

# Sectioning

Integrate with existing sectioning:
- Active tasks (incomplete, non-validators)
- Validator tasks
- Complete tasks

# Verification

All existing E2E display tests must pass.
Run `cargo test` and `cargo run list` to verify.
