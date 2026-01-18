---
id: markdown-parser
title: Build a markdown parser that can parse task files
status: complete
---

# Description

Using serde_yaml, build a parser that can extract the
description and frontmatter from a Task markdown file.

This is the structure of the yaml frontmatter
* id: Unique identifier
* subtasks: Child task IDs
* preconditions: Tasks that must complete first
* validations: Validator task IDs that must pass
* title: String/optional
* validator: Boolean marking persistent validation tasks

The content of the markdown file has no structure apart
from the frontmatter. The content of the markdown file
is the task description.

# Outcome

A function for taking the content of a task md file 
(the actual content that would come out of reading from a path) 
and outputting a Task struct.

The output should be a Rust Result, the error system should use
thiserror, we should use a custom Error enum.

Later, we'll want to provide excellent user facing errors
on exactly what failed to parse.

# Acceptance Criteria

A test which can successfully parse a task
A test which cannot
