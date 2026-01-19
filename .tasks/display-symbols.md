---
id: display-symbols
title: Cell-to-ASCII symbol conversion
before:
  - display-refactor
after:
  - display-routing
gates:
  - test
status: complete
---

Convert Connection cells to ASCII symbols based on their flags.

# Symbol Mapping

```
{up, down}           -> │
{left, right}        -> ─
{up, down, right}    -> ├
{up, down, left}     -> ┤
{down, right}        -> ╭ or ┌
{down, left}         -> ╮ or ┐
{up, right}          -> ╰ or └
{up, left}           -> ╯ or ┘
{up, down, left, right} -> ┼
```

# Implementation

Simple pattern match on the four boolean flags.
Return 2-char string (symbol + space) to maintain column alignment.
