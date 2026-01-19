---
id: mont-edit-passed
title: Add --passed argument to mont edit
before:
  - mont-start
after:
  - global-settings
---

Using the --passed argument, you can specify a list of gates which have passed validations.
This updates the md to be like:

# ---
# gate:
#     - my_gate: passed
# ---

Having all gates (including default ones) marked as passed will be a requirement for `mont-done`
