---
id: update-readme-for-release
title: Update README to match current CLI
---

The README has outdated examples that don't match current CLI:
- `mont gate "Tests pass"` syntax doesn't exist
- `mont distill --tasks='...'` flag doesn't exist  
- Shows `-e` for editor but editor is now default

Update to reflect current behavior:
- `mont` opens editor
- `mont <ids>` edits tasks
- `mont jot` for quick ideas
- `mont distill <jot-id>` opens editor
