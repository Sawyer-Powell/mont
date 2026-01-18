---
id: bug-list-error
title: Display is not correct when running mont-list
type: task
status: complete
---

# Replication steps

Create a task: "task-parent"
Create another task: "task-precondition" which has parent "task-parent"
Create another task: "task-a" which has parent "task-parent" and precondition "task-precondition"
Create another task: "task-b" which has parent "task-parent" and precondition "task-precondition"

## Expected output

A diamond shape in the graph output
