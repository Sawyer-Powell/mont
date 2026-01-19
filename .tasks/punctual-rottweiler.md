---
id: punctual-rottweiler
title: Maintain task continuity after mont done
type: jot
---

After mont done, we lose context of what we were working on.

Solution: Walk back in jj revision history to find the last completed task (identified by a change to status: complete in a .tasks/*.md file). Use that information to identify and annotate "next" tasks - the tasks that were blocked by the completed one and are now ready.

Clearly annotate these "next" tasks in mont list and mont ready output so the user (or Claude) knows what to pick up next.
