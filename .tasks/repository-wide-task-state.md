---
id: repository-wide-task-state
title: Design repository-wide task-state resolution
type: jot
---

Define the architecture that allows every Mont CLI invocation to resolve one coherent task graph from repository-wide JJ state. `docs/jj-workspace-orchestration.md` is the behavioral source, refined by the decisions recorded here.

Tracked tips are registered JJ workspace revisions and Mont-owned `mont/<task-id>` bookmarks. Successful resolution uses JJ's speculative merge of their `.tasks` trees without integrating the operation or changing a physical working copy. Ordinary source conflicts outside `.tasks` are irrelevant. A conflicted or invalid merged task tree fails loudly before command dispatch.

The resolver must also detect parallel edits to the same canonical task path even when JJ merges their contents cleanly. Parallel edits mean multiple incomparable latest modifications reachable from tracked tips. The design task owns selecting and validating an efficient algorithm, such as a single scan of divergent commit ancestry with parent IDs and changed paths; it must analyze cost rather than prescribing a command-per-task implementation. For MVP, parallel `config.yml` edits rely on JJ merge behavior and receive no equivalent ancestry check.

The coherent graph may fast-fail rather than preserve a rich partial graph. The architecture must still define actionable diagnostics, repository/workspace discovery, explicit JJ command context, supported JJ output contracts, local ancestry-sensitive readiness, mutation preflight boundaries, and operation-race behavior.

This jot also owns the first global `mont status` contract and an audit of every existing Mont command. For each command, decide whether it consumes repository-wide state, retains local semantics, changes behavior, or is removed. Human-readable output remains the agent-facing contract; JSON is not part of the MVP.

Definition of done: a reviewed architecture document with diagrams or data-flow descriptions where useful, focused real-JJ experiments for uncertain semantics, performance reasoning for parallel-edit detection, a command-by-command impact table, and a dependency-ordered set of autonomous small/medium implementation tasks. Each implementation task should target roughly 500 lines of change and remain below about 1000 lines.
