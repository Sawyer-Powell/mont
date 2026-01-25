---
id: simplify-mont-commands
title: Make mont base command be a shortcut for mont task
---

Right now, if I want to create a new task, I can run mont task, and it opens my editor,
but if I want to edit something, I need to do 'mont task <ids>'

Going forward if I want to edit something, I just want to be able to do:

'mont <ids>'

To create a new task, I want to do:

'mont'

Basically, mont task gets aliased by the base 'mont' command
'mont status' is no longer aliased to the base command. Instead mont status is invoked as mont status, or mont st
