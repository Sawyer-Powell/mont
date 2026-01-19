---
id: mont-llm
title: Implement mont llm
status: complete
after:
  - mont-start
gates:
  - test: passed
---

This is the LLM/agentic coding namespace for mont.

Here's the core idea:

Mont is able to track the state of todos, and the state of in progress tasks.

The command that we need to implement is `mont llm prompt`. This reads the active state
of the mont task graph and generates a prompt for the LLM based on this state.

E.g.

State: No Task Active & jj revision has uncommitted changes

Generates stdout:
There is currently no task that has been marked as in progress, however it looks
like this revision has changes that have not been committed yet in JJ.

Ask the user if they would like to commit these changes with `jj commit`. Read the active
changes with <appropriate jj command for viewing changes> and suggest a commit message
and description for them.

And so on.

The idea is that we can write a new command alongside this called `mont llm start <task id>` (opens picker over ready tasks if no id specified)
which specifically asks the coding agent to run `mont llm prompt` and complete its requirements. Once it has completed its work
run `mont llm prompt` again to get the next prompt.

Here's a high level view of the states we should track.

enum InProgressState {
  NeedsImplementation,
  GatePendingUnlock,
  PendingMontDone,
}

enum TaskGraphState {
  NoInProgress,
  InProgress(InProgressState)
}

enum JJRevisionState {
  RevisionHasChanges,
  RevisionNoChanges
}

enum JJRevisionDescriptionState {
  RevisionHasDescription,
  RevisionDoesNotHaveDescription
}

I'm thinking the code looks like a match statement over the combination of all these states.
