---
title: Slice 135 Rust SDK candidate-only S02 bounded contention
status: AUDITED_LOCAL_RAW_PENDING_RETENTION
target_release: 0.8.27
---

# Rust SDK S02 bounded contention — 2026-10-08

The [functional protocol](../../rust-s02-contention-protocol.json) was
committed at `93e814f37` before the repeated campaign. Its SHA-256 is
`69c47776972e3ab1e756a8c00f10b1ad824897ce5ee8d8c09329599eecb04718`.
The external Cargo consumer compiled against clean product source
`3f29d649d0213e595c0dab251a449d92fd625792` and Rust crate tree
`c8eaaa19a0e0e876326b778733ace354cc7e21e8`. The external manifest,
lockfile, consumer source and release binary hashes matched the frozen
protocol. This is a **candidate-only source-bound Rust SDK** exercise; 0.8.26
has no Rust SDK peer, and the crate was not installed from a published
registry artifact.

Each fresh real database received 32 retained corpus nodes and a three-node,
one-edge provenance graph, then a searchable text/vector projection. Eight
writer calls and eight reader cycles used the same `Engine` concurrently.
Readers asserted exact anchor body, text and vector retrieval, and graph
neighbors during writes. The sequence drained projection work, asserted all
eight committed writer rows, resolved evidence, erased graph and writer
sources, then closed and reopened the engine. A separate SQLite connection
checked the retained anchor body, canonical row counts and
`PRAGMA integrity_check` after the product timer.

The initial feasibility run passed with eight overlapping reader cycles.
The frozen campaign then retained **ten of ten valid fresh-process runs**.
The independent auditor reopened every raw receipt and database: actual
writer/reader overlap ranged from seven to eight cycles per process; each
reopen had 32 corpus nodes, no erased graph or writer rows, the exact anchor
body and SQLite integrity `ok`. One process recorded a host-only paging
warning, and none of the measured children swapped. Whole-sequence median
was 5,863.764 ms, with observed range 5,824.410–5,899.372 ms. These are
descriptive candidate-only observations; no 0.8.26 delta, p95/p99 or
equivalence claim is supported by this functional campaign. The separate
[single-caller Rust S02 timing](../2026-10-07-rust-s02-candidate-timing/README.md)
has its own frozen workload and timing boundary.

Two retained negative controls changed actual campaign evidence and updated
the intermediate SHA-256 claims. The independent auditor rejected a false
overlap count against the call timeline and a changed retained anchor body
against the reopened SQLite oracle. Its parser also accepts relocated raw
archives while checking the recorded command and binary identity. The seven
focused campaign/audit tests, Rust formatting, Ruff and the scoped Markdown
validator passed. The local raw directory is
`/tmp/slice135-rust-s02-contention-campaign-93e814f37`; its verified
`SHA256SUMS` covers **104 files** and has SHA-256
`5289064b831f53dda66a90818f6052a631bc55b4fb2685924488904facf3d22e`.
Final raw-archive retention is deferred until the end of Phase 1.

This result supplies the Rust SDK bounded-contention route required by S02.
S03, the remaining Pareto/robustness/logic matrices, full functional
conditions and the exact-candidate Phase 1 checkpoint remain open.
