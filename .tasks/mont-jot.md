---
id: mont-jot
title: Need to add a new task type called 'jot' and 'distill'
complete: true
after:
  - add-jj-lib
  - mont-new
validators:
  - test
  - readme-validator
---

# Description

Some tasks are not yet well defined, but you want to record them anyway to address
later. Let's call these jots.

An idea for a new feature pops into your head

Use mont jot to quickly record the idea down
Use mont distill to transform jots into new tasks

Mont distill opens up a text file where you can write multiple
markdown files at once.


---
id: first-step
title: Here is the first step
---

Here is my description of the first step

---
id: second-step
title: Here is the second step
after:
    - first-step
---

Here is what needs to happen in the second step

After mont distill, the jot is deleted and these tasks are
inserted into the graph

Finally, the jj revision editor pops up for you to enter a description,
then mont moves you to the next revision.

# Task

Add another type to the task type enum called "jot".

Add a new command like mont new called mont jot. The user should just be able to type in mont jot, or mont jot "<title here>".
Default behavior is to open the user's $EDITOR

# Validation Criteria

- Jots should have their own color and [jot] tag at the end of mont list items
