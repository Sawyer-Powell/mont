---
id: disable-jj
title: Need a way to disable jj operations in global config
type: jot
---

Need a way to disable jj operations in global config.
My thought here is that we can set up the global config to parse a "jj" field in the yaml that can be enabled
or disabled (enabled by default).

This is then used by the jj module. If it's set, then we return the true/happy path for all the functions
to keep the rest of the system running.

This is for repoos where .tasks might be git excluded. I.e. an IC in a shared codebaase.
