---
id: support-for-binstall-and-brew
title: We need to support cargo binstall and brew
type: jot
---

I would like our repo to get prepared for continuous release cycle.
As a part of this, we need a way to compile binaries on github (macos only for now)
and distribute them using brew and cargo binstall.

We also need to consider how to best lock down the repo to prevent
releasing things like breaking changes, especially to task schema.

Also need to consider how deployment will work, alongside versioning.

Maybe cargo semver checks?
