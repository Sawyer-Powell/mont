---
id: boisterous-mudfish
title: Distill same-id bug causes task deletion
type: jot
---

When distilling a jot into a task with the same ID, the delete operation removes the newly inserted task. The insert happens before delete in the transaction, so Delete(id) removes what Insert(id) just created.

