---
title: Slice 35 implementation review — FIX-7
status: RESOLVED_PENDING_FINAL_REVIEW
date: 2026-09-05
---

# Slice 35 implementation review — FIX-7

FIX-6 corrected the measurement boundary but its independent review found four
remaining defects. FIX-7 closes them:

- the mutation-source scanner distinguishes Rust lifetimes from character
  literals and inventories the production `prune_edge_projection_shadows`
  deletion path;
- measured-call counters are opt-in for the Slice 35 witness and do not alter
  the closed ordinary SCALE-02 repetition schema;
- blocked version-3 plans still validate witness-binding shape, uniqueness,
  references, artifact identity, and JSON pointers before returning a blocked
  classification; and
- the design names the actual `apply_batch_in_transaction` implementation seam.

The correction followed RED/GREEN discipline. Commit `380399e1` added failing
regressions for the lifetime-bearing mutation path, malformed blocked-plan
bindings, and opt-in call counts. Commit `386783e9` made those tests pass. The
focused classification, Slice 35, SCALE-02, and mutation-audit suite passed 46
tests after the correction.

The authoritative clean-tree measurement is
`scale-02-slice35-20260905T0404Z-659c38ea`. It binds the runner and invocation
blobs at `386783e9c27386552a6ecca6326a3c9dd59df7be`, identifies both arms as
`Engine.search_text_only`, and records exactly 5,500 measured calls per arm.
The candidate product commit remains
`0aff1cb08c61a8bb2a004813bbd5604b6ff1a403`. It passed the preregistered
three-percent non-regression boundary with 95-percent upper relative regression
of 1.623% at p50 and 2.121% at p95, with zero errors and zero timeouts.

Earlier Slice 35 receipts remain immutable historical evidence. Policy version
3 quarantines the four receipts whose operation label was not supported by
their execution path; no prior bytes were rewritten.
