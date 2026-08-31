---
title: 0.8.24 Slice 70 — release readiness design
status: REVIEWED
target_release: 0.8.24
---

# Slice 70 release readiness design

The release candidate consumes immutable upstream records rather than rerunning
or recreating their workflows. The public-doc checker selects the greatest
semantic release with a validated canonical `published` record from Git's
tracked state files; release records are the source of the public version fact.
The candidate changelog has an explicit prepared/not-canonically-published
0.8.24 heading, while 0.8.23 accurately records its published state.

The only remaining transition is owner-controlled:

```text
locally verified candidate -> owner publication authorization -> public exact versions -> post-publish smokes -> complete
```

No tag, push, PR, main merge, workflow dispatch, environment approval, or
registry action is part of this slice.
