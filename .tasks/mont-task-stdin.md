---
id: mont-task-stdin
title: Add stdin support to mont task for LLM usage
status: complete
gates:
  - architecture-validator: passed
  - test: passed
---

The `--content` flag has issues when content starts with `---` (clap parsing confusion).

Add `--stdin` flag that reads task content from stdin:
```bash
echo '---
id: foo
title: My task
---
Description' | mont task --stdin
```

This is the most LLM-friendly approach:
- No shell escaping issues
- Works with any content (including `---` at start)
- Common CLI pattern

Implementation:
- Add `--stdin` flag to TaskArgs
- In task_cmd.rs, if stdin flag is set, read from stdin instead of using --content
- Conflicts with: --content, --resume, --patch, --append, editor mode
