---
id: allow-empty-id-in-parse
title: Allow empty ID in task parsing for auto-generation
status: complete
gates:
  - user-qa: passed
  - test: passed
  - architecture-validator: passed
---

Add #[serde(default)] to the `id` field in Task struct so tasks can be
parsed without an ID. The multieditor already has code to generate IDs
for empty ones - this just allows that code path to be reached.

The EmptyId check in context/mod.rs still prevents loading ID-less tasks
from disk, so only the multieditor flow allows auto-generation.
