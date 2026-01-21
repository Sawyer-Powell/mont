---
id: mont-prompt-no-jj
title: We need to fix behavior when jj is disabled for mont prompt
type: jot
---

Right now, jj state powers mont-prompt. We need to rethink this in light of jj integration possibly being disabled in
repos.

I think best for now is to change jj: disabled, to auto-commit: disabled?

That way we can still leverage jj revision status to power for mont-prompt.
