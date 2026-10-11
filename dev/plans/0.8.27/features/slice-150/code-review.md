---
title: FathomDB 0.8.27 Slice 150 — code review
status: COLLECTOR_CHANGE_APPROVED
target_release: 0.8.27
---

# Slice 150 code review

## Rust SDK hidden-surface row — `gpt-6-sol` high, 2026-10-10

Independent read-only review of RED `d80f43bcc` and GREEN `c3239879e`
returned **APPROVE**, no code finding. The test first failed at the exact
missing `sdk-default` row. Adding the one `fathomdb-sdk` default row passed all
fast hidden-surface self-tests. The existing collector builds hidden-inclusive
rustdoc JSON and test inventory for every row; this addition uses those paths
without changing the Rust SDK or other row definitions.

The reviewer identified one expected comparison identity change. The reviewed
Slice 140 baseline has `tests-req-fathomdb-sdk--none` with 38 entries. Adding
the explicit `sdk-default` row replaces that derived inventory identity with
`tests-sdk-default`. Qualification must compare their entries directly and
record the rename and the additive SDK rustdoc row; the prior baseline entries
remain immutable. This review is not a final-candidate capture or a review of
the later oracle-retirement deletion.
