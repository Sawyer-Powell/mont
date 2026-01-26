---
id: init-command
title: Add interactive mont init command
status: complete
gates:
  - user-qa: passed
  - test: passed
  - architecture-validator: passed
  - semver: passed
---

Add a `mont init` command that initializes the `.tasks` directory with user-chosen git tracking preferences.

## Detection (run first, report status)

On startup, detect and report existing configuration:
1. Check if `.tasks/` directory exists
2. Check if `.tasks` is in `.gitignore`
3. Check if `.tasks` is in `.git/info/exclude` (local exclude)
4. Check global git exclude via `git config --global core.excludesFile` (defaults to `~/.config/git/ignore`)

Display current state to user before prompting.

## Interactive prompts

Ask user for tracking preference:
1. **Tracked** (default) - Include in source control, no exclusions
2. **Gitignore** - Add `.tasks` to `.gitignore` (shared across clones)
3. **Git exclude** - Add to `.git/info/exclude` (local only, not shared)

If already configured, show current state and ask if they want to change it.

## Actions

Based on choice:
- Create `.tasks/` directory if missing
- Create default `config.yml` if missing
- Add/remove `.tasks` from appropriate exclude files based on choice
- If changing from one exclude method to another, clean up the old one
- **If switching from tracked to ignored**: Run `git rm -r --cached .tasks` (or jj equivalent) to untrack files while keeping them on disk

## Edge cases

- If `.tasks/` exists with tasks, don't delete anything - just update git tracking
- If global exclude has `.tasks`, inform user but don't modify global config (suggest they remove it manually)
- Handle non-git directories gracefully (skip git-related setup)
