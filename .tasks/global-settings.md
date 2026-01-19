---
id: global-settings
title: Enable a global settings yml file in .tasks file.
status: complete
---

We need a global settings file that can be configured to set up things
like gates that always must run.

The first thing to implement here is:
- A module in context.rs that defines how to parse the yaml file with serde
- The only item we need defined now is a list of default gates that must run
- In MontContext.rs we first load the taskgraph, then we load the global config and validate the config against the taskgraph
- This validation function should be on the impl GlobalConfig and accept a &TaskGraph as a parameter
