---
id: claude-not-showing 
title: Bug in mont claude when not using --ignore flag
---

There is a bug in running mont claude when not using ignore flag,
claude never appears in my terminal. My cursor just hanges below
my prompt line in the terminal.

I think this has something to do with how we're piping/routing stdin/stdout
for claude code.
