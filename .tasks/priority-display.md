---
id: priority-display
title: Show computed priority in all task displays
after:
  - priority-effective
---

Update render.rs to show priority indicator in all task displays.
This includes: mont list, mont ready, mont show, mont status, fzf picker.
Always display the computed/effective priority, not the stored one.
Sort tasks by effective priority in mont list and mont ready.

