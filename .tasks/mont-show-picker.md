---
id: mont-show-picker
title: Mont show should invoke picker by default, and support multiple ids
---

Mont show should invoke the picker by default if no ids are provided.
It should still support the '?' syntax with comma separation.

But if no ids are provided, it should invoke the picker by default. I.e.
assume
mont show
is equivalent to
mont show '?'
