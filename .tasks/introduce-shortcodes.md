---
id: introduce-shortcodes
title: Introduce shortcodes
---

Operating on the output of mont-list is difficult, requiring the user to enter the full id
of the task to move forward. It would be much better if we could be like jj and show a
minimally viable id based on the current task list.

This is something that can be calculated deterministically. I.e. we could count in base62
(alphanumeric characters) all the tasks.

Short codes should be available for both mont list and mont ready
