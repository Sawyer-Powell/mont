# PROJECT GUIDELINES

This project is written in Rust, and uses JJ version control.

!!IMPORTANT!! Ask me lots of questions, you are a code monkey,
you do not make technical or architectural decisions.

!!IMPORTANT!! Keep your code simple. Opt for a simple functional style.
Your code should also avoid nesting where possible.

# PROJECT INFO

This project is called "mont" it's building a task management
and agent coordination framework. Here's an overview of the project

## Core Architecture

### Task Graph Foundation
The system is built on a **deterministically validated task graph** defined in `.tasks/*.md` files. Each task has frontmatter:
- `id`: Unique identifier
- `subtasks`: Child task IDs
- `preconditions`: Tasks that must complete first
- `validations`: Validator task IDs that must pass
- `title`, `explanation`: Optional metadata
- `validator`: Boolean marking persistent validation tasks

Task graph is a DAG validated deterministically before any work begins.

### Validator Tasks
Special tasks analogous to Claude Code skills. Define validation criteria:
- How to run unit tests, integration tests
- Automated validation (e.g., Playwright browser tests)
- Human validation (structured interviews/reviews)

Validators are reusable across multiple tasks in the graph.

### Agent Execution Model
1. Agent claims ready task from graph
2. Agent generates **agenda** - itself a task graph using same frontmatter format
3. Agenda references validators from main task graph for clear validation criteria
4. Agenda must pass deterministic validation before work begins
5. Agent executes in isolated JJ revision
6. Completion requires all agenda validators to pass

This ensures agents only execute well-defined tasks with explicit success criteria.

### Parallel Execution & Conflict Detection
- Tasks with no dependency edges execute in parallel by different agents
- Each agent works in isolated JJ revision
- Before modifying a file, agent notifies coordinator daemon
- Coordinator tracks file modification attempts across all active agents
- **Early conflict detection**: If Agent A modified file F and Agent B wants to modify F:
  1. Agents communicate directly to update plans
  2. If architectural conflict, escalate to human
  3. Human may update task graph, triggering work abandonment

### Coordination Hub
Human operates from trunk via integrated interface:
- **Ratatui TUI**: Graph visualization and agent request inbox
- **Neovim**: Task graph editing in `.tasks/`
- **Zellij**: Session orchestration and pane management
- Coordinator daemon tracks agent activity
- Each graph edit produces deterministic diff
- Diff drives mutations: drop work, modify JJ revisions, reassign tasks

### State Management
- Coordinator daemon runs on trunk
- Agents track file modification intent
- Distributed coordination via file modification notifications
- Task graph diffs compute required mutations deterministically

### Workflow Summary
1. Human defines/modifies tasks in `.tasks/*.md` via Neovim
2. Coordinator identifies ready tasks (preconditions met, parallel-safe)
3. Agents claim tasks, generate validated agendas, execute in JJ revisions
4. Coordinator detects conflicts via file modification tracking
5. Agents coordinate or escalate to human via TUI inbox
6. Human reviews in TUI, updates task graph, diff drives next steps
7. Completed work merges to trunk after human approval

## Technical Foundation
- **Language**: Rust with trait abstractions for modularity
- **VCS**: JJ (one task = one revision, cheap abandonment)
- **Initial Stack**: Zellij (orchestration), Neovim (editing), Ratatui (TUI)
- **Future-proof**: Generic traits support alternative editors, terminals, UIs
- **Agent**: Claude Code behind trait (swappable LLM execution)
- **Validation**: Deterministic graph validation at every step

## Design Philosophy
Clear task decomposition with explicit validation criteria. Parallel execution with early conflict detection. Human maintains architectural coherence through task graph management. Abandoned work is expected and valuable. System optimizes for human clarity, not agent throughput.

