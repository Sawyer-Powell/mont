---
id: fix-jot-status-display
title: Fix jot in-progress showing as incomplete in mont show
gates:
  - user-qa
  - test
---

When a jot is marked as in-progress, mont show displays "Status: incomplete" which is confusing.
Jots don't have gates, so they can't be "incomplete" - they're either ideas (not started) or 
being distilled (in-progress).

Fix mont show to display appropriate status for jots:
- If not in-progress: show "Status: jot (not started)" or similar
- If in-progress: show "Status: being distilled" or similar
