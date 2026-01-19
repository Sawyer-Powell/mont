# mont

```bash
# Describe a gate, a task describing some form of quality control
% mont gate "Tests pass" \
    --description "Run 'make lint' then 'make test'. Mark as passed if no errors."
created: .tasks/festive-bengal.md

# Jot down an unstructured idea for your codebase
% mont jot "Add user authentication"
created: .tasks/buff-bloodhound.md

# Use claude code to distill your thought and implement
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
