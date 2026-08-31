---
id: mandatory-jj-mode
title: Remove optional JJ operation mode
type: jot
---

Plan removal of `.tasks/config.yml`'s `jj.enabled` setting and every no-JJ or happy-path branch in commands and JJ helpers. Mont's new operating model requires a JJ repository and versioned task state.

The design must inventory affected commands, define diagnostics outside a JJ repository, and separate this cleanup from the larger explicit repository-context and repository-wide state work.

Definition of done: document the supported mandatory-JJ behavior and produce bounded implementation tasks and verification for all removed configuration and command branches.
