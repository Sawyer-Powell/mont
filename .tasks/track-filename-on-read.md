---
id: track-filename-on-read
title: Need to track task md file when reading
status: inprogress
---

When reading tasks from the disk, we don't record the file we read them from on the actual task struct.
This is a problem since right now we're assuming that task ids are the same as their file names.
This isn't always the case.
