---
id: add-jj-lib
title: Add jj-lib integration
complete: true
---

Add a module for running and parsing output from the jj command line tool

Start with functions for
1. 'jj commit'
2. Basic facilities for showing entire change history for a file.
  - For this, let's first confirm a good library for parsing the diffs from jj output? Or maybe there's a jj native way to do this?
