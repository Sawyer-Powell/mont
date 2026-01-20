# mont

A task tracker to help you and your agent write robust, reliable code.
Built on jj-vcs.

```bash
# Describe some gates, a task describing some form of quality control
# Gates should have a clear success state
% mont gate "Tests pass" \
    --description "Run 'make lint' then 'make test'. \
                   Mark as passed if no errors."
created: .tasks/festive-bengal.md

# If you want to use an agent, you can create a gate for doing a code review with you
% mont gate "Implementation interview" \
    --description "Scan your changes using 'jj diff', \
                   and then conduct an interview with the user \
                   to confirm implementation details"
created: .tasks/festive-bengal.md

# Jot down an unstructured idea for your codebase
% mont jot "Add user authentication"
created: .tasks/buff-bloodhound.md

# Use claude code to help you distill the jot then implement
% mont claude buff-bloodhound

# OR use mont manually
% mont distill buff-bloodhound --tasks='
    - id: auth-backend
      title: Add auth backend
      gate: [festive-bengal]
    - id: auth-ui
      title: Add login UI
      after: [auth-backend]
    - id: auth-tests
      title: Add auth tests
      after: [auth-backend]
  '
created: .tasks/auth-backend.md
created: .tasks/auth-ui.md
created: .tasks/auth-tests.md
deleted jot: buff-bloodhound
committed: Distilled jot 'buff-bloodhound' into tasks: auth-backend, auth-ui, auth-tests

# View dependency graph
% mont list
◉    [task] auth-backend Add auth backend
├─╮
○ │  [wait] auth-tests Add auth tests
  ○  [wait] auth-ui Add login UI

# Start work, this marks 'auth-backend' as in progress
% mont start auth-backend

# Unlock gate when tests pass
% mont unlock auth-backend -p festive-bengal
festive-bengal gate marked as passed

# Complete the task, this commits the changes and moves to a new revision
% mont done -m "Added auth backend"
```

## Tips and Tricks

**Claude integration.** Run `mont claude -i` anytime to get help tackling mont tasks. Mont generates prompts dynamically based on your tasks and repo state. Use `mont prompt` to inspect the generated prompt.

**Fuzzy finder.** Install [fzf](https://github.com/junegunn/fzf) to enable mont's picker functionality. Anywhere you'd normally type an id, you can omit it. Mont invokes an appropriate fzf picker to help you find your task.

**Editor support.** `mont task`, `mont jot`, `mont gate`, and `mont show` all accept `-e` to edit tasks in your preferred editor. Set `$EDITOR` or pass the binary explicitly with `-e`.
