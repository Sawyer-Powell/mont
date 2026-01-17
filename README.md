# mont

A vcs friendly task management system for humans and LLMs.

Built with Rust on top of jj-vcs. Useful to track and plan work in your codebase in a simple,
version controlled way. Architected from the ground up to eventually serve as the data model
for coordinating parallel execution of coding agents. Today it helps you, your team, and your
agent plan and tackle work in your codebase.

## How to use

Tasks live in `.tasks/*.md` files. Each task declares:

```yaml
---
id: my-task
title: Human readable title
parent: parent-task        # optional
preconditions:             # must complete before this task
  - other-task
validations:               # validation tasks, maybe a script, maybe a prompt
  - cargo-tests
complete: false            # optional, set this to true if you want to track the file as complete

---

Task description in markdown.
```

## Usage

```
mont list                  # show task graph (hides completed)
mont list --show-completed # include completed tasks
mont ready                 # show tasks ready to work on
mont check                 # validate entire task graph
mont check <task-id>       # validate a single task and its references
mont new                   # create a new task
mont edit <task-id>        # edit an existing task
```

### Creating tasks

```
mont new --id my-task                           # create task with explicit id
mont new --title "My Task"                      # create task with generated id
mont new --id my-task --title "My Task"         # both id and title
mont new --id my-task --parent parent-task      # set parent
mont new --id my-task --precondition pre1,pre2  # set preconditions
mont new --id my-task --validation test         # set validations
mont new --id my-task --type bug                # set task type (feature, bug, epic)
mont new --id my-task --editor                  # open in $EDITOR after creation
mont new --id my-task --editor vim              # open in specific editor
```

### Editing tasks

```
mont edit my-task --title "New Title"           # update title
mont edit my-task --new-id new-id               # rename task (updates references)
mont edit my-task --parent new-parent           # change parent
mont edit my-task --precondition pre1,pre2      # replace preconditions
mont edit my-task --validation test             # replace validations
mont edit my-task --editor                      # open in $EDITOR
mont edit my-task --resume /path/to/temp        # resume failed edit
```

## Current output of `mont list` for this repo

Items with ◉ icon are ready for work

```
◉  mont-show Implement mont show command
│ ◉    add-jj-lib Add jj-lib integration
│ ├─╮
│ ○ │  mont-complete Implement mont complete command
├─╯ │
│   ○  mont-start Implement mont start command
├───╯
○  cli-commands Mont CLI Commands
◉  global-settings Enable a global settings yml file in .tasks file.
◉  llm-specific-commands Think through support for a set of llm specific commands
◉  mont-edit Add a mont edit command
◉  plan-type
◉  review-error-aesthetics Review error message aesthetics with Claude Code

◈  interview-validator Conduct interview to confirm changes [validator]
◈  readme-validator Ensure the readme is up to date with code [validator]
◈  test Run tests [validator]
```
## Status

Early development by one person on their macbook. Core task parsing and graph visualization implemented.
Beware linux and windows users, this might not be ready for you yet. Bug reports and contributions will be ignored 
for now, but feel free to ask questions.

### Working

- Task definition via markdown files with YAML frontmatter
- Graph validation (DAG enforcement, reference checking, cycle detection)
- CLI: `mont list` with JJ-style graph visualization
- CLI: `mont ready` to show tasks ready to work on
- CLI: `mont new` for creating new tasks with automatic ID generation
- CLI: `mont edit` for editing tasks with ID rename and reference propagation
- Task relationships: parent/child, preconditions, validations
- Validator tasks for defining reusable acceptance criteria


## Core ideas:

There's been an uptick in interest in version controlled task trackers for codebases as
coding agents have become better and better. One notable project is `beads` from Steve Yegge.
Which is powering his engine of vibe coding insanity known as `gas town`.

Using beads I found the task management useful, but the implementation overcomplicated and dense.
This prompted me to start building `mont`, a vcs compatible task tracker that is useful for humans
first, and optimized for agents later.

`mont` just asks you to write a bunch of markdown files in a `.tasks` folder in your repo. 

I don't want `mont` to have some convoluted database it has to maintain, `mont` state should be
entirely (or as much as possible) defined in the markdown files.

In the yaml frontmatter of those files, you can set up dependencies between tasks, 
parent-child relationships,
and even designate certain tasks as "validators". Validators are just tasks that describe how to run
some sort of validation on the codebase. Right now, for humans, they might be just helpful reminders, 
but this is extremely helpful for your coding agent. Instead of having to string together a 
hodgepodge of Claude skills to run your testing suite, go through a validation process with you, etc. 
Just write a task and include it in the validators. The `mont` cli will do its best to force your 
agent to always remember to run them when necessary.

`mont` becomes more interesting with the current plan for incorporating `jj-vcs`. `mont` models your
tasks as a DAG, which look suspiciously like the DAGs you get when running `jj log`. The goal when
completing programming tasks with `mont` is to try and reduce the scope of each task to something that
can be easily accomplished and verified inside a single `jj` revision. `mont` will give you tools
to create `jj` revisions from your tasks, and then allow you to understand your `jj` history
*in terms* of tasks. The design goal here is to augment the usefulness of both tools by finding interesting
ways to combine both systems to get a great paper trail.

Finally, similar to projects like `agentic-jujutsu`, `mont` aims to provide a coordination daemon
that can spin up and help you manage coding agents that try to tackle parallel tasks in your task graph.
This will be coming last, and will be built modularly on top of the core task management. The design goal here
is to enable `mont` to detect merge conflicts between agents as early as possible by tracking file modifications.
Sometimes merge conflicts can be resolved by simple inter-agent coordination, but sometimes they hint at bad
task parallelization, which means us, the humans, don't understand our code architecture yet.

My philosophy with `mont` is that this problem should be solved by fixing the tasks that caused the problem.
So, I the programmer should be able to write simple effective diffs on the task graph. These diffs would then drive
an update to the jj revision graph to get your code state to a point where parallel agents can be spun up again.

