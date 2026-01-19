---
id: jot-improvements
title: Across the board improvements for jots
status: complete
gates:
  - interview-validator: passed
  - test: passed
---

If a jot is marked in progress, our prompts should be different.

Right now we would use the same language for tasks as we do for jots, instead, we need a
new prompt template that encourages claude to distill the jot into task(s).

Also, jots are not designed to have gates, we should enforce this in our validation system
and across all places we render tasks, like mont show, edit, list, status etc. Jots should never
be shown to have gates.

The lifecycle of a jot is this:

`mont start <jot-id>`
jot is now in progress
`mont distill <jot id>` <- we need to make sure mont distill is LLM friendly, meaning completely scriptable with parameters 


`mont done` cannot be run on jots
