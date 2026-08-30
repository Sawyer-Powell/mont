# JJ Workspace Orchestration Refactor

## Overview

Mont is becoming a JJ-native task tracker. JJ is mandatory, `.tasks` remains
versioned repository state, and Mont interprets that state across the whole JJ
repository rather than treating one working directory as the complete world.

The refactor adds managed worker workspaces without replacing the existing
in-place lifecycle:

- `mont start`, `mont done`, and `mont stop` continue to operate in place.
- `mont workspace start`, `mont workspace done`, and `mont workspace stop`
  operate on Mont-managed JJ workspaces.
- `mont workspace integrate` consumes completed managed work.
- `mont workspace cleanup` removes abandoned managed directories.
- `mont status` is global and reports task state across JJ workspaces.

Every invocation first inspects repository-wide task state. Read commands remain
available when that state is invalid, but report the ambiguity instead of
pretending to have a coherent answer. Mutation commands are blocked until the
state is repaired.

The first vertical slice is `mont workspace start`: prove repository-wide
validation, selected-revision readiness, managed workspace creation, and
idempotent recovery against temporary real JJ repositories. The next slice
makes cross-workspace status useful to humans. The final MVP slice completes,
queues, and integrates worker work.

## Status And Authority

This document refines the August 29, 2026 handoff at
`/tmp/opencode/mont-jj-workspace-orchestration-design.md`. The handoff remains
useful background, but this document is authoritative where the decisions
differ.

The codebase currently has a local-only operating model:

- `main.rs` loads `./.tasks` before dispatch.
- `MontContext` combines the local read model, validation, mutation, and disk
  persistence.
- `jj.rs` runs ambient-current-directory subprocesses and has no workspace,
  bookmark, ancestry, operation, rebase, or conflict primitives.
- Lifecycle commands mix validation, mutation, JJ execution, and rendering.
- Tests are unit tests; there is no reusable temporary real-JJ harness.

Useful existing seams are `GraphView`, transaction operations, per-command
modules, the centralized `jj.rs` module, and `TaskDisplayView`. They may evolve;
this design does not require preserving their present APIs.

## Decisions That Supersede The Handoff

### Managed lifecycle is namespaced

Managed workspace behavior is opt-in under `mont workspace` rather than the new
default meaning of `mont start`, `mont done`, and `mont stop`.

The command family is:

```text
mont workspace start <task-id> [--revision <revset>]
mont workspace done
mont workspace stop
mont workspace integrate <task-id>
mont workspace cleanup
```

The unnamespaced lifecycle remains the in-place workflow. Both modes use shared
task validation and repository-wide consistency checks.

### Repository-wide state gates mutation

The handoff made mutation decisions from the caller's local graph. The revised
model preflights every mutation against repository-wide state.

Mont synthesizes repository-wide state from exactly:

- every registered JJ workspace revision; and
- every Mont-owned queue bookmark named `mont/<task-id>`.

Arbitrary bookmarks and dormant historical heads are not part of current task
state.

Mont may mutate a task only when the current write workspace already contains
that task's repository-wide latest state. If another tracked branch contains a
newer task modification, Mont rejects the write rather than creating a second
modifying branch.

Multiple tracked branches modifying the same task concurrently is illegal even
if JJ can automatically merge identical file contents. A race may still create
this condition; the next Mont invocation detects and reports it.

Repository-wide state answers ownership, provenance, and consistency questions.
Readiness remains ancestry-sensitive: a dependency completed on another branch
does not make code available at the selected start revision.

### Human output is the agent contract

The JSON output proposed in the handoff is dropped. Commands produce clear
human-readable output, including paths, revisions, operation IDs, partial state,
and recovery instructions where relevant. Agents consume this output directly.

The existing Pi extension is deprecated and should not constrain the new output
or architecture.

### JJ is mandatory

Mont no longer supports a non-JJ lifecycle mode. The `jj.enabled: false` path
and initialization behavior that untracks `.tasks` are incompatible with this
model and should be removed. `.tasks` is versioned JJ repository state.

### Task IDs make a breaking change

Every task ID must be safe as all of:

- a `.tasks/<id>.md` filename;
- the suffix of a JJ workspace name `mont/<id>`; and
- the suffix of a JJ bookmark name `mont/<id>`.

Mont will fail loudly when loading a repository containing legacy IDs. It will
not provide an automatic migration command in this refactor. Existing invalid
IDs and references are repaired manually before using the new Mont version.

The exact accepted grammar still needs to be specified before implementation.
It must be enforced consistently at load, create, edit, rename, distill, and all
lifecycle boundaries.

## Repository-Wide Operating Model

### Discovery

Mont resolves the current JJ workspace root and shared repository before loading
`.tasks`. Commands work from any nested directory in a registered workspace.
JJ operations receive explicit repository or workspace context rather than
depending on the process working directory.

### Synthesis

Repository-wide analysis inspects all registered workspace revisions and all
`mont/*` queue bookmarks. A speculative JJ merge may be created with
`--no-integrate-operation` and inspected at the returned operation. It must not
change any physical working copy or integrate the speculative operation.

The synthesized model records, at minimum:

- each task version and its source workspace, bookmark, change ID, and commit ID;
- whether one task was modified concurrently on multiple tracked branches;
- conflicts in `.tasks`, including `config.yml`;
- malformed Mont workspace and queue bookmark names;
- workspace ownership and lifecycle status contradictions; and
- whether a task's latest repository-wide state is contained in the current
  write workspace.

Any `.tasks` conflict is invalid repository state. Ordinary source-file
conflicts are valid and do not prevent managed work from starting.

### Invalid state

Mont reports repository invalidity prominently on every invocation until it is
repaired.

- Mutation commands are blocked.
- Read commands remain callable.
- A read command that cannot produce an unambiguous normal answer reports the
  relevant errors or competing versions instead.
- Repair occurs through JJ or direct `.tasks` changes outside Mont; Mont does not
  mutate an ambiguous graph in an attempt to repair it.
- Filesystem cleanup is also blocked while repository state is invalid.

`mont check` and `mont status` are the primary diagnostics. Diagnostics should
name the involved workspaces, bookmarks, revisions, task IDs, and paths.

### Safe writes

A task mutation writes a focused delta into the current workspace. Before the
write, Mont verifies that the current workspace contains the repository-wide
latest version of every task the operation changes, including indirect changes
to references or configuration.

This rule applies to lifecycle commands and ordinary task commands such as
create, edit, rename, delete, distill, and gate changes. Validation and mutation
must account for all files affected by one logical operation, not only the task
named on the command line.

## Shared Start Semantics

Both in-place `mont start` and managed `mont workspace start` may start only a
task that is:

- an executable task, not a jot or gate;
- incomplete;
- not already in progress; and
- unblocked by dependencies at the revision where work will begin.

This corrects the current implementation, which does not reject jots, gates, or
dependency-blocked tasks.

Repository-wide preflight must also show that no tracked workspace or queue owns
the task and that the selected write base is safe.

## Managed Workspace Start

### Command behavior

`mont workspace start <id>` defaults to starting from the caller's `@` revision.
Unlike the in-place command, it accepts a non-empty working-copy revision. JJ
snapshots that revision, and the new worker begins with it in its ancestry. This
is required for a managed worker to start a subtask on top of its own in-progress
changes.

`--revision <revset>` accepts full JJ revset syntax. A revset may select one or
multiple parent revisions. The prospective worker's merged `.tasks` state must
be conflict-free and show the requested task as startable. Ordinary code
conflicts in a multi-parent worker are accepted and resolved as part of the
worker's task.

If dependencies are complete elsewhere but not at the selected revision, the
command fails and identifies the revisions containing those completions. The
user may supply an appropriate base revset. The selected base must itself
contain, or cleanly merge, all dependency completions required by the task.

### Ownership and storage

The active worker workspace is named `mont/<task-id>`. The unique JJ workspace
name is the managed ownership claim.

Generated workspace directories live under:

```text
$XDG_DATA_HOME/mont/workspaces/<repository-key>/<workspace-directory>
```

The repository key is derived from the canonical shared `.jj/repo` path. The
path resolver has a narrow injectable data-root boundary so internal tests can
use isolated temporary directories without mutating process-global environment
variables. Production obtains the root from the platform environment.

For the MVP, a managed-path collision without a matching workspace may be
treated as a loud error. Mont need not distinguish every possible source of an
unregistered directory. Contradictory states, such as the destination already
being registered under another workspace name, are invalid and reported.

### Start saga

Starting spans validation, JJ operations, task mutation, snapshotting, and
filesystem effects. It is implemented as an idempotent saga rather than an ACID
transaction.

Conceptually, the command:

1. Captures the pre-command JJ operation ID.
2. Synthesizes and validates repository-wide task state.
3. Resolves and validates the selected base revset.
4. Claims workspace name `mont/<task-id>`.
5. Creates the managed directory and worker working-copy revision.
6. Loads the worker's `.tasks` graph and marks the task `inprogress`.
7. Forces JJ to snapshot the worker revision.
8. Prints the workspace path, workspace name, change and commit IDs, selected
   parent revisions, and recovery information.

On retry, each step detects whether its intended effect already exists and skips
completed work. Recognizable partial state resumes from the first incomplete
step. Contradictory or ambiguous state stops with a precise error.

The MVP does not need exhaustive provenance for every orphan directory. It does
need faithful error reporting, idempotency, and tests for each meaningful partial
state. Mont does not automatically roll back a partial start. It reports the
pre-command operation ID and explains JJ recovery separately from ordinary
filesystem cleanup.

## Global Status

`mont status` is not namespaced. It becomes the primary human view of
repository-wide task state while retaining useful local JJ status information.

It should distinguish:

- task state at the current workspace revision;
- repository-wide effective task state;
- owning managed workspace or queue bookmark;
- source workspace and revision for the latest task update;
- whether that update is in the current workspace's ancestry;
- invalid or competing task versions; and
- dependency completions that exist elsewhere but not at the current revision.

`mont ready` may use repository-wide consistency checks, but a task is ready only
relative to its selected local/base revision. Repository-wide completion alone
does not make a dependent task locally ready.

## Managed Completion, Stop, And Integration

These commands retain the lifecycle and recovery principles from the handoff,
but live under `mont workspace`.

### `mont workspace done`

The command runs in the owning `mont/<task-id>` workspace. It verifies task
ownership, repository validity, in-progress state, gates, and absence of
descendant revisions. It marks the task complete, snapshots the completed tip,
creates queue bookmark `mont/<task-id>` at that tip, and forgets the JJ workspace
without deleting the physical directory.

It does not invoke an interactive commit, rebase the work, squash revisions, or
create an empty child. The command is an idempotent saga with operation-ID and
partial-state recovery reporting.

### `mont workspace stop`

The command runs in the owning workspace, rejects descendant revisions,
abandons worker-created revisions back to the branch point, forgets the
workspace, and leaves physical directory removal to cleanup. It does not need to
persist a `stopped` task status. It follows the same resumable saga contract.

### `mont workspace integrate <task-id>`

The command runs from the workspace that should receive queued work. It rebases
the complete worker-only revision range ending at bookmark `mont/<task-id>` onto
the caller's current revision without squashing, advances the caller to a fresh
empty child of the rebased tip, and removes the queue bookmark.

Code conflicts are valid results. `.tasks` conflicts are invalid. A bookmark
already contained in the caller's ancestry is stale rather than repository-wide
corruption and must be removed before another integration attempt. Integration
is resumable and reports its pre-command JJ operation ID.

### `mont workspace cleanup`

Cleanup removes only directories inside Mont's managed workspace root whose JJ
workspaces are no longer registered. It never removes a registered workspace and
does not treat physical directories as durable task state.

## Verification Strategy

Semantics are developed first against internal APIs backed by temporary real JJ
repositories. CLI tests then verify the end-to-end user contract. Mocked JJ text
output is not sufficient evidence for workspace, revset, merge, conflict,
bookmark, rebase, or operation-log behavior.

The test harness should provide:

- an isolated temporary JJ repository and configurable workspace topology;
- an injectable Mont data root;
- helpers to create tasks and task dependencies at selected revisions;
- helpers to create workspaces, bookmarks, divergent revisions, and conflicts;
- inspection of workspace registrations, ancestry, operation IDs, task files,
  and physical directories; and
- CLI invocation from repository roots and nested directories.

Tests should be parallel-safe without changing process-global environment from
internal library tests. CLI subprocess tests may set isolated environment values
per process.

## Suggested Task Breakdown

The following tasks are intentionally small to medium. Intermediate states do
not need to be release-ready, but every task must have a focused automated proof.

### Foundation

#### F1. Add a temporary real-JJ test harness

Create reusable test support for initializing temporary JJ repositories,
creating revisions and workspaces, writing `.tasks`, and inspecting resulting JJ
state. Include an injectable managed-data root.

Verification: two isolated harness instances can run concurrently; a smoke test
creates a second workspace and verifies its registered name, parent, and path.

Dependencies: none.

#### F2. Introduce explicit JJ repository and workspace context

Replace ambient-current-directory assumptions with repository discovery and a
typed command boundary that accepts explicit repository/workspace context.
Resolve the workspace root from nested directories. Make JJ mandatory and stop
untracking `.tasks` during initialization.

Limit this task to discovery, command execution, and identifiers needed by later
tasks; do not implement lifecycle behavior yet.

Verification: real-JJ tests discover the same shared repository and correct
workspace root from root and nested paths, including non-default workspaces.

Dependencies: F1.

#### F3. Define and enforce the JJ-safe task-ID grammar

Specify the grammar and enforce it at repository load plus every creation,
editing, rename, distill, and lifecycle boundary. Update this repository's own
legacy task IDs and affected references manually. Do not add a migration command.

Verification: table-driven grammar tests, load failure for legacy IDs, collision
tests, and round trips through every ID-changing command path.

Dependencies: none. Complete before managed workspace names are exposed.

#### F4. Read task trees at arbitrary JJ revisions

Add a read-only task/config loader for one revision or a selected merged revision
without treating it as the mutation target. Preserve enough source information
to report paths and revisions in diagnostics.

Verification: load distinct task states from divergent real-JJ revisions and
report `.tasks` or `config.yml` conflicts without changing a working copy.

Dependencies: F1, F2.

### Repository-wide model

#### R1. Inventory tracked task-state revisions

List every registered workspace revision and every valid `mont/*` queue bookmark
with typed workspace, bookmark, change, and commit IDs. Flag malformed Mont-owned
names without including arbitrary bookmarks.

Verification: real-JJ topology tests cover default, non-default, nested, stale,
and malformed workspace/bookmark entries.

Dependencies: F2.

#### R2. Synthesize repository-wide task provenance

Build the read-only repository model from the R1 inventory and F4 revision
loader. Track each task version's sources and whether its latest state is in the
current workspace ancestry. Use speculative JJ operations where merging is
required, without integrating the operation.

Verification: divergent non-conflicting task edits synthesize successfully and
leave the operation log/working copies unchanged; provenance points to the exact
workspace and revision.

Dependencies: F4, R1.

#### R3. Validate repository-wide invariants

Detect `.tasks` conflicts, concurrent modifications of one task including
identical content, malformed ownership names, multiple owners, contradictory
lifecycle state, and invalid local graphs. Represent invalidity independently of
normal graph loading so diagnostics remain available.

Verification: one real-JJ test per invalid-state class, including exact source
workspace, revision, task, and path diagnostics.

Dependencies: R2, F3.

#### R4. Gate command execution on repository validity

Change command startup so every invocation performs repository inspection before
normal execution. Read commands degrade to prompt diagnostics when ambiguous;
all mutations and cleanup are blocked. `check` remains usable even when ordinary
task parsing or graph validation fails.

Verification: CLI matrix across representative valid and invalid repositories,
showing reads remain callable and no mutation changes JJ or filesystem state.

Dependencies: R3.

#### R5. Enforce repository-safe task mutations

Before every existing mutation, verify that the write workspace contains the
repository-wide latest versions of all directly and indirectly affected task and
config records. Reject writes that would create a second modifying branch.

Verification: existing mutations succeed from an up-to-date workspace and fail
without changes from a stale sibling; include rename/reference/config rewrites
and a race that becomes detectable invalid state.

Dependencies: R4.

### First vertical slice: managed start

#### S1. Centralize startability validation

Create one startability rule used by both in-place and managed start. Reject
jots, gates, complete or in-progress tasks, and dependency-blocked tasks at the
selected revision.

Verification: focused graph tests plus CLI tests proving both start modes produce
the same validation result.

Dependencies: F3.

#### S2. Resolve managed workspace paths and start bases

Implement the narrow injectable data-root resolver, repository key, deterministic
workspace destination, and full-JJ-revset base resolution. Preflight the selected
single- or multi-parent task graph; accept ordinary code conflicts but reject
task conflicts and unavailable tasks.

Verification: real-JJ tests for `@`, a named revision, compound revsets,
multi-parent bases, dependency completion elsewhere, task conflicts, and accepted
code conflicts.

Dependencies: F2, F4, R3, S1.

#### S3. Implement the managed-start happy path

Add the internal operation that claims `mont/<id>`, creates the managed worker as
a child of the selected base, marks the task in progress there, snapshots it,
and returns human-renderable result metadata. Allow a non-empty caller.

Verification: a real-JJ test asserts workspace registration, managed path,
parentage, task status, snapshot, and unchanged caller contents.

Dependencies: R4, R5, S2.

#### S4. Make managed start resumable and idempotent

Model start as explicit detectable steps. Resume recognizable partial states,
reject contradictory states, capture pre-command operation IDs, and separate JJ
recovery guidance from filesystem leftovers. Do not add automatic rollback.

Verification: construct every meaningful partial state in temporary real-JJ
repositories, retry, and assert one correct final worker; verify contradictory
states fail without further mutation.

Dependencies: S3.

#### S5. Expose `mont workspace start`

Add CLI routing and concise human output for success, validation failure, partial
failure, and recovery. Support invocation from nested directories and
`--revision <revset>`.

Verification: end-to-end CLI tests exercise root, nested, default, non-default,
dirty caller, single-parent, multi-parent, retry, and concurrent claim cases.

Dependencies: S4.

### Second vertical slice: global observation

#### O1. Add a repository-wide display model

Represent local status, effective repository status, provenance, ownership,
ancestry presence, and ambiguity separately from ANSI rendering.

Verification: rendering-independent tests cover active workers, queued work,
stale local state, and competing versions.

Dependencies: R3.

#### O2. Make `mont status` repository-wide

Render the O1 model in a concise human view while retaining useful local JJ
status. Show dependency completions that exist elsewhere and the revisions that
contain them.

Verification: CLI snapshots for sibling and nested workers, queued completions,
invalid state, and a locally blocked task whose dependency is complete elsewhere.

Dependencies: O1, R4.

#### O3. Align other read commands with global validity

Update `list`, `show`, `ready`, and `check` to use shared repository diagnostics
while keeping readiness relative to the selected/current revision.

Verification: command matrix proves consistent diagnostics and no false local
unblocking from remote completion.

Dependencies: O1, R4.

### Third vertical slice: completion and integration

#### C1. Add queue bookmark and descendant primitives

Extend the typed JJ boundary with bookmark create/delete, descendant checks,
snapshots, workspace forget, and operation metadata required by done and stop.

Verification: real-JJ primitive tests, including descendant topology and
bookmark/workspace name reuse.

Dependencies: F2, F1.

#### C2. Implement resumable `mont workspace done`

Verify ownership and gates, reject descendants, complete and snapshot the task,
create the queue bookmark, and forget the workspace without deleting its
directory. Reconcile or supersede the existing `innocuous-groundhog` task.

Verification: happy path plus retry after every meaningful interruption and
contradictory-state tests.

Dependencies: C1, R5, S5.

#### C3. Implement resumable `mont workspace stop`

Verify ownership, reject descendants, abandon worker-created revisions, and
forget the workspace without persisting a stopped state or deleting its
directory.

Verification: happy path, nested worker blocker, and retry after every meaningful
interruption.

Dependencies: C1, R5, S5.

#### C4. Determine and rebase the worker-only revision range

Add typed ancestry/range queries and preserve the complete worker revision
structure while rebasing onto the caller. Do not squash by default.

Verification: linear, multi-revision, sibling, nested, and conflicted real-JJ
topologies produce the expected range and rebased ancestry.

Dependencies: C1.

#### C5. Implement resumable `mont workspace integrate`

Consume a queued bookmark, rebase C4's range onto the caller, advance the caller
to a fresh empty child, and remove the bookmark. Handle code conflicts, invalid
task conflicts, stale already-integrated bookmarks, partial operations, and
operation restore guidance.

Verification: end-to-end real-JJ and CLI tests for clean, conflicted, stale, and
interrupted integration.

Dependencies: C2, C4, O2.

#### C6. Implement `mont workspace cleanup`

Remove orphaned directories only within Mont's managed root and only when no JJ
workspace remains registered. Respect the global invalid-state mutation gate.

Verification: registered directories survive; forgotten-workspace directories
are removed; unrelated and out-of-root paths are untouched.

Dependencies: C2, C3, R4.

### Follow-up integration

#### I1. Remove deprecated agent launchers and non-JJ modes

Remove or deprecate the Pi extension, synchronous `mont claude` process launch,
`jj.enabled: false`, and untracked-task initialization paths. Retain prompt
facilities only where they are useful to external agents.

Verification: build/package tests and documentation contain no supported path
that launches an agent or treats `.tasks` as unversioned state.

Dependencies: S5, O2. It may be performed earlier if it does not delay the first
vertical proof.

#### I2. Update user documentation and release notes

Document mandatory JJ, ID breakage, namespaced managed lifecycle, global status,
human output, recovery, and manual migration expectations.

Verification: command examples run against a temporary repository and the
README no longer describes removed behavior.

Dependencies: C5, C6, I1.

## Dependency Summary

The critical path to the first end-to-end proof is:

```text
F1 -> F2 -> F4 -> R1 -> R2 -> R3 -> R4 -> R5 -> S3 -> S4 -> S5
              \                         /
               F3 -> S1 -> S2 ---------
```

Some work can proceed in parallel:

- F3 can proceed alongside F1/F2/F4.
- S1 can proceed while repository synthesis is built.
- R1 can begin once F2 exists.
- C1 and C4 can begin after the JJ boundary stabilizes, although their CLI
  surfaces should wait for the first slice.
- O1 can begin after R3 and does not block S3's internal happy-path proof.

The first release-quality checkpoint is S5 with all dependencies complete. O2
is the second user-visible checkpoint. C5 plus C6 completes the MVP lifecycle.

## Deferred Work

- Automatic legacy-ID migration.
- JSON or another machine-specific output mode.
- A daemon, lock service, or ACID transaction layer.
- Automatic rollback of partial lifecycle operations.
- Automatic agent launching or monitoring.
- Gate-agent provenance enforcement.
- Automatic revision squashing.
- A central named-repository configuration.
- Direct use of JJ's Rust libraries instead of the CLI.

## Open Questions

These decisions should be settled while distilling the relevant tasks, not
silently invented during implementation:

- What exact task-ID grammar is accepted?
- What human output fields and wording are stable enough for agent use?
- Which JJ versions and CLI output formats does Mont support?
- How should Mont identify the complete worker-only range when a worker has
  unusual merge ancestry?
- Which recognizable partial states belong to each lifecycle saga, and which
  states are contradictory rather than resumable?

## Rationale

Repository-wide validation prevents Mont from quietly creating contradictory
task histories while preserving JJ as the inspectable source of ownership and
topology. Ancestry-sensitive readiness prevents a task from starting against code
that does not include its completed dependencies. Namespacing managed lifecycle
allows the existing in-place workflow to remain useful without overloading one
command with two ownership models.

Real-JJ tests are the primary semantic proof because the design depends on JJ's
actual workspace, revset, merge, conflict, bookmark, rebase, snapshot, and
operation-log behavior. Small internal checkpoints make those semantics visible
early; CLI tests then validate the complete human workflow before release.
