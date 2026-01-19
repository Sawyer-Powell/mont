---
id: priority-data-model
title: Add priority field to Task struct
---

Add Priority enum with levels: Low, Med, High, Urgent (default: Med).
Update YAML serde in task.rs for parsing and serialization.
Update to_markdown() to serialize priority field.

