---
title: 0.8.25 Slice 35 design reconciliation review
status: APPROVED
design_version: 6
review_fix: 4
date: 2026-09-04
---

# Slice 35 design reconciliation review

An independent reviewer approved design v6 after four bounded FIX cycles with
no unresolved P1 or P2 findings.

The review required the design to reuse the shipped `SearchFilter` vocabulary;
enumerate pre-truncation lowering for every search arm; apply caller eligibility
to emitted graph nodes rather than transport edges; replace unavailable Slice
40 generation IDs with current-state digests; cover in-place visibility changes
with a checked monotonic generation; define one-snapshot race semantics; keep
filter values out of the authenticated token; specify the byte codec and typed
failures; and make source, package, platform, CUDA, and performance gates
executable.

The resulting design is additive. It does not introduce a second filter DSL,
retained snapshot lease, semantic policy, or automatic profile routing.
