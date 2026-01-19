---
id: llm-prompt-templates
title: Extract LLM prompt templates to markdown files
status: complete
after:
  - mont-llm
gates:
  - test: passed
---

Extract the default LLM prompt templates from hardcoded strings in llm.rs to markdown files.

## Requirements

1. Create a `.tasks/prompts/` folder (or similar) for storing prompt templates
2. Use `include_str!` macro to embed the templates at compile time
3. Support a simple templating syntax for dynamic values like:
   - `{{task.id}}` - task ID
   - `{{task.title}}` - task title
   - `{{task.description}}` - task description
   - `{{gate.id}}` - current gate ID
   - `{{gate.title}}` - gate title
   - `{{gate.description}}` - gate description
   - `{{gates.unlocked}}` - comma-separated list of unlocked gates
   - `{{gates.pending}}` - comma-separated list of pending gates

4. Create template files for each state:
   - `no-task-in-progress.md`
   - `no-code-changes.md`
   - `has-code-changes.md`
   - `some-gates-unlocked.md`
   - `all-gates-unlocked.md`

5. Implement a simple template renderer that replaces `{{variable}}` placeholders

This allows users to customize prompts by editing the markdown files, and keeps the Rust code clean.
