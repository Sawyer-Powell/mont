---
id: versioned-task-initialization
title: Make mont init always version tasks
type: jot
---

Plan removal of `mont init` modes that add `.tasks` to ignore or exclude files and untrack it through Git or JJ. `.tasks` is mandatory versioned repository state in the workspace architecture.

The design must cover fresh initialization, repositories where `.tasks` is already ignored or untracked, user-facing diagnostics, and removal of obsolete Git tracking-preference code without broadening into repository-wide graph synthesis.

Definition of done: specify initialization and repair behavior and produce bounded implementation tasks with real repository verification.
