---
id: semver
title: Determine version bump
type: gate
---
Review the changes made in this task and determine the appropriate version bump.

1. Run `jj diff` (or `git diff` if jj disabled) to see all changes
2. Read current version from `Cargo.toml`
3. Assess the changes:
   - **Major** (breaking): removed/renamed public CLI commands, changed behavior in incompatible ways
   - **Minor** (feature): new commands, new flags, new functionality
   - **Patch** (fix): bug fixes, internal refactors, docs, CI changes

4. Ask the user to confirm the version bump using AskUserQuestion:
   - Show current version and proposed new version
   - Briefly explain why (1 sentence)

5. Update `Cargo.toml` with the new version

YOU MUST get explicit confirmation from the user before updating the version.
