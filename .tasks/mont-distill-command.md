---
id: mont-distill-command
title: Add mont distill command
status: complete
gates:
  - user-qa: passed
  - architecture-validator: passed
  - test: passed
---

Add `mont distill <jot-id>` command to convert jots to tasks:

1. Takes a jot ID as argument
2. Opens multieditor with:
   - Original jot commented out at top (will be deleted since not in parsed output)
   - Instructions explaining the flow
   - Empty space for user to create replacement tasks
3. When saved, the diff sees original=[jot], edited=[new-tasks]
4. Result: jot deleted, new tasks created

Example temp file:
```
# Original jot (will be deleted - do not edit below this line)
# ---
# id: my-jot
# title: Quick idea about caching
# type: jot
# ---
# Some rough notes here

# Create replacement tasks below:
---
id: implement-caching
title: Implement caching layer
---
Description...
```
