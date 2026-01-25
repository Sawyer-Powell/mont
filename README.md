# mont

A task tracker to help you and your agent write robust, reliable code.
Built on jj-vcs.

## Notes

**ALERT - ALPHA SOFTWARE**:
- `mont` should work as described BUT
- Breaking changes will likely happen
- There are likely obvious bugs that haven't been found yet
- There are areas that are likely lacking ergonomic polish
- `mont` works well so far on my M4 Macbook Pro, but might be slow on your machine

**DON'T PANIC**:
- You can help improve `mont`!
- Report issues you find here on GitHub, I'll try and resolve them time-permitting

## Installation

Right now I'm only building/distributing binaries for MacOS. If you really want a different OS/linux distro supported,
create an issue here on GitHub.

Requires [jj-vcs](htps://github.com/artinvonz/jj): 
```bash
brew install jj
```

### Homebrew (macOS)
```bash
brew install sawyer-powell/tap/mont
```

### Cargo binstall
```bash
cargo binstall mont
```

### Cargo install
```bash
cargo install mont
```

## Quick Start

```bash
# Create a new task (opens your editor)
% mont
created: .tasks/my-task.md

# Or create a quick jot (unstructured idea)
% mont jot -q "Add user authentication"
created: .tasks/cheerful-otter.md

# View what's ready to work on
% mont ready

# Start working on a task
% mont start my-task

# When done, complete the task
% mont done -m "Added auth backend"
```

## Core Concepts

**Jots** are unstructured ideas that need to be refined into tasks. Use them to capture random ill-defined tasks from your stream of consciousness while working.

**Tasks** are concrete work items with clear completion criteria.

**Gates** are quality checkpoints (tests pass, code reviewed, etc.) that must be unlocked before completing a task.

## The Multieditor

`mont` opens your editor with a multi-document format. Create, edit, and link tasks in one session:

```yaml
---
id: auth-backend
title: Add authentication backend
gates: [code-review]
---
Implement JWT-based auth with refresh tokens.

---
id: auth-frontend
title: Add login UI
after: [auth-backend]
---
Build login/logout components.
```

Save to apply changes atomically. Use `mont <ids>` to edit specific tasks.

## Using Claude

```bash
# Let Claude tackle a task
% mont claude auth-backend

# Claude receives a prompt with task details, dependencies, and repo context
# It works through gates, asks for review, then completes the task
```

Use `mont prompt` to preview what Claude will receive.

## Commands

| Command | Description |
|---------|-------------|
| `mont` | Open editor to create/edit tasks |
| `mont <ids>` | Edit specific tasks |
| `mont status` | Show in-progress tasks |
| `mont list` | Show task dependency graph |
| `mont ready` | Show tasks ready for work |
| `mont jot [title]` | Create a quick jot |
| `mont distill <id>` | Convert jot to tasks |
| `mont start <id>` | Begin working on a task |
| `mont done [-m msg]` | Complete current task |
| `mont unlock <id> -p <gate>` | Mark gate as passed |
| `mont show <id>` | View task details |
| `mont delete <id>` | Delete a task |
| `mont claude <id>` | Launch Claude Code for a task |

## Tips

**Fuzzy finder.** Install [fzf](https://github.com/junegunn/fzf) to enable picker functionality. Instead of typing in a task id, many commands accept you entering `?` in their place. For each `?`, a picker is invoked to select the task id.

**Claude integration.** Use `mont claude <task-id>` to launch Claude Code with a dynamically generated prompt based on your task state. Use `mont prompt` to inspect what prompt would be generated.

**Shortcuts.** `mont st` is an alias for `mont status`.

# Notes for Contributors

This software is made by me, Sawyer, with a primary customer of me, Sawyer. While I would like this software to be useful
to as many people as possible, I'm not going to be fostering an open source community around `mont`, allowing others to submit PRs.

If you REALLY want to contribute, reach out to me directly and I'll see if you can be helpful. But don't expect your PRs to be
reviewed, much less approved. If you really don't like how `mont` currently works, and your github issue doesn't have a response
from me, fork it.

# Disclosure of LLM usage

This software is designed to make it easier for you to write reliable, well designed software using LLMs. As such, `mont` is developed
with extensive usage of LLMs (using `mont`!).

Some of the internals of the codebase may look weird/have duplicated logic, or just plain ol' bad code. If you find it, report
it, create a GitHub issue calling it out. The design goal of `mont` is to allow our usage of LLMs to produce *better* code than we could
without them. Bad code in `mont` means I'm not using `mont` correctly to build `mont`, or `mont` is designed poorly.
