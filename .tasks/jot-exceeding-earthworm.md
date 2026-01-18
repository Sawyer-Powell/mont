---
id: jot-exceeding-earthworm
title: Implement search + edit flow
type: jot
---

1. User can do a fuzzy search over files in a tui format
2. User can then select which files they want to include in their bulk edit
3. All files are loaded into one file, with the same separation logic as mont distill
4. The new set of files generated from the process replaces the old set, same effective logic as delete old ones, create new ones.
5. The selection process for bulk edit should also support a simple command line argument for a list of ids, not the tui fzf version
