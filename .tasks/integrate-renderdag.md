---
id: integrate-renderdag
title: Integrate sapling-renderdag for DAG visualization
before:
  - display-refactor
after:
  - display-render
validations:
  - test
status: complete
---

Replace our custom layout/routing code with Facebook's sapling-renderdag crate, which is battle-tested (used by jj and Sapling).

# Background

Our current layout algorithm has issues with edges visually passing through unrelated nodes. The git/jj approach uses column reservation and forbidden column checking to avoid this, which renderdag implements.

# Implementation

1. Add dependency: `sapling-renderdag = "0.1"`

2. Create adapter (~50 lines):
   - Wrapper struct similar to jj's `SaplingGraphLog`
   - Convert our task edges to renderdag `Ancestor` types:
     - Direct edges (parent/precondition) → `Ancestor::Parent`
     - Indirect edges → `Ancestor::Ancestor`
   - Iterate tasks in topological order
   - Call `renderer.next_row()` for each task

3. Update render pipeline:
   - Replace `compute_layout` + `build_grid` + `route_edges` with renderdag calls
   - Keep our task line formatting (markers, colors, titles)
   - Keep sectioning logic (active, validators, complete)

4. Choose rendering style:
   - `BoxDrawingRenderer` for curved/square Unicode
   - `AsciiRenderer` for ASCII-only terminals

# References

- jj integration: https://github.com/jj-vcs/jj/blob/main/cli/src/graphlog.rs
- renderdag docs: https://docs.rs/sapling-renderdag
- Sapling source: https://github.com/facebook/sapling/tree/main/eden/scm/lib/renderdag

# Verification

- All existing display tests pass
- `cargo run list` shows correct graph with no false connections
- Edge cases: diamonds, wide merges, long chains, independent components
