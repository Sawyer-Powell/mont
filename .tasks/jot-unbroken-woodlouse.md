---
id: jot-unbroken-woodlouse
title: Organize mont list & ready
type: jot
---

- Jots should show in a separate group in mont list.
- Jots should show organized together under features in mont ready
- Mont ready output should have titles more truncated
- Fix truncation algorithm to not leave trailing space. I.e. don't do "some text ...", instead should be "some text..."
- In mont ready, the color of the id should match the coloration of the task type

## Introduce shortcodes

Operating on the output of mont-list is difficult, requiring the user to enter the full id
of the task to move forward. It would be much better if we could be like jj and show a
minimally viable id based on the current task list.

This is something that can be calculated deterministically. I.e. we could count in base62
(alphanumeric characters) all the tasks.

Short codes should be available for both mont list and mont ready
