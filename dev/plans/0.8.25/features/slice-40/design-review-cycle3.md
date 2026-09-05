---
title: 0.8.25 Slice 40 independent design review — cycle 3
status: CHANGES_REQUIRED
review_cycle: 3
reviewed_on: 2026-09-05
reviewed_commit: 97bb4a77
---

# Slice 40 independent design review — cycle 3

Cycle 3 accepted the no-assignment model but found the physical-membership and
progress semantics incomplete. FIX-3 closes the findings before the final
allowed design-review cycle.

| ID | Severity | Finding | FIX-3 disposition |
|---|---|---|---|
| DR40-36 | P1 | Default-view eligibility did not match retained physical membership or relaxed reads. | Define physical membership independently for retained nodes; echo one effective instant for the edge rule; share predicates across projector paths. |
| DR40-37 | P1 | Incomplete and structurally torn states were conflated as processing. | Add a total state table; emit processing only for scheduler-reachable work and typed corruption for impossible partial states. |
| DR40-38 | P1 | Slice-25 pending receipts used terminal absence and could omit a no-runtime incomplete owner. | Build the pending list with the same completion classifier inside the actuation transaction. |
| DR40-39 | P1 | Persisted embedder identity could appear to change without an epoch transition. | Pin the profile before generation bootstrap, recompute the digest on open/read, and preserve the accepted identity-mismatch gate. |
| DR40-40 | P1 | A worker had no captured generation to compare at publication. | Add generation ID to each in-memory job; discard and rediscover a stale result at commit. |
| DR40-41 | P2 | Public responses and runtime enums were prose-only. | Define exact Rust records/enums, facade names, bindings, ordering, and decimal encoding. |
| DR40-42 | P2 | Legacy/redacted receipt behavior conflicted with new invariants. | Separate new and legacy NULL rules; preserve actuation erasure while mapping status redaction to non-disclosing not-tracked. |
| DR40-43 | P2 | Frozen digest named vector bytes absent from v1. | Append the exact v1 state-then-terminal bytes and retain visibility triggers as physical invalidation authority. |
| DR40-44 | P2 | Generation history allowed invalid mutation/boundary shapes. | Add boundary checks and immutable/retention triggers; define the limited cross-row claim and open-time corruption type. |
| DR40-45 | P2 | Storage measurement lacked fixed actuation shape. | Fix 10k operations/128-per-batch/79 receipts/100%-pending/checkpoint shape and report contributions separately. |
