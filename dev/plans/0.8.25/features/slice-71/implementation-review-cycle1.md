---
title: 0.8.25 Slice 71 — implementation review cycle 1
status: NEEDS_FIX
reviewed_commit: 8e586e66b343f3813c8b4be466bae13dd1fc4c34
date: 2026-09-08
---

# Slice 71 implementation review — cycle 1

Independent review passed the product search-path change and its focused
tests, but returned `NEEDS_FIX` on evidence handling:

- P1: the first manifest and observations were committed together, so that
  campaign was not immutable preregistration;
- P1: receipt validation trusted supplied digests/classification instead of
  recomputing them from manifest bytes and retained metrics;
- P2: AC-013 cells omitted SQLite/runtime and complete host-pressure evidence.

The correction retains the first campaign as exploratory, adds failing tests
at `a6a2ccf2`, derives digests and classifications, closes environment/runtime
fields, commits the revised protocol before measurement, and reruns all six
cells. A fresh independent review is required on the final candidate.
