---
title: Slice 135 search row-error follow-up audit
status: VECTOR_AND_EXPLANATION_SITES_REPAIRED_LEGACY_PROBES_PENDING
target_release: 0.8.27
---

# Search row-error follow-up audit — 2026-10-07

This is a source audit of `search.rs` at Git
`5f2f2cc29359fb190995de94a369b8544d274d94`, file SHA-256
`9cf58338b27f93295588c53d9aa89f44944d28bfe45bb3338872943406d4c24f`.
It extends the [first logic result](logic-first-result-2026-10-06.md) after
the node and edge FTS row-error repairs. The inventory below is the
pre-vector-repair snapshot; its subsequent dynamic result is recorded below.
It is not a complete exception-path audit.

The source snapshot has four `rows.flatten()` sites. A `rusqlite` row iterator
contains `Result<Row, Error>`; iterator flattening drops an `Err`. That is
hazardous when the operation promises a complete answer or truthful error.
The current node and edge FTS paths instead collect into
`rusqlite::Result<Vec<_>>` and propagate decoding failures.

| Site | Result that can be lost | Current classification | Next discriminator |
| --- | --- | --- | --- |
| `search.rs:1545` vector phase-1 candidates | A `rowid` or distance decode error can omit a nearest-neighbor candidate before hydration. | Confirmed incomplete-result defect on the active vector path; repaired after this snapshot. | The [RED/GREEN real-database receipt](results/2026-10-07-vector-row-repair/README.md) shows `Ok([])` before the fix and `EngineError::Storage` after it. |
| `search.rs:1987` pre-step-12 node fallback with `source_id` | A row error can remove a text hit. | Unsafe iterator shape, but unreachable through current public open: normal admission requires schema version 34 before search. Migration test hooks are a distinct route. | Retain as a test-hook-only audit lead; do not count it as a supported runtime path without a route witness. |
| `search.rs:2011` pre-step-8 node fallback | A row error can remove a text hit. | Same current public-open exclusion; a missing `source_id` column is a legacy-schema condition. | Retain as a test-hook-only audit lead; verify any future historical-data compatibility decision before deleting or repairing the branch. |
| `search.rs:2104` edge attribute explanation count | A failed cursor decode can undercount `dropped_edge_hits`; outer `if let Ok` also suppresses statement/query errors. | Confirmed on the active opt-in route by a test-only SQL decode fault; repaired after this snapshot. | The [RED/GREEN real-database receipt](results/2026-10-08-edge-explanation-repair/README.md) shows the previous false success and the repaired `EngineError::Storage` result. |

The source also has `rank_stream_candidates` ending in `.ok()` near line 1870.
That fallback to full sorting is intentional only if the fallback produces
the same complete answer. Existing rank-stream tests compare ordinary
result ordering, limits, and selected routes; fault-specific equivalence
remains unmeasured. The four sites above are an inventory, not four confirmed
bugs. The vector site is now confirmed by its separate real-database receipt.
The previous probes confirmed **two** FTS row errors and a distinct
missing-edge-index statement fallback; those receipts remain the evidence
for those repaired defects.

The public-open exclusion above follows `open.rs`: the normal
`open_with_embedder_and_subscriber_config` plan uses
`DatabaseAdmission::CurrentOnly`, and `open_with_migrations` calls
`admit_current_database` before constructing the engine. The admission check
rejects a noncurrent `user_version`; the current `SCHEMA_VERSION` is 34.
`DatabaseAdmission::TestMigrations` is feature-gated. This source-grounded
route proof does not establish that the legacy branches are harmless if a
future release again supports opening those historical schemas.

The edge explanation site has now been probed and repaired. Review test-hook
routes before considering a generic Semgrep prohibition on `.flatten()`:
most other uses in the crate
flatten `Option`, not iterator `Result`, and a broad rule would be noisy.
A focused semantic rule or compiler lint can later flag iterator flattening
of `Result` in database row loops, paired with reviewed exceptions. Existing
Clippy and typechecking did not flag the two confirmed FTS defects. Their
absence is not evidence that the remaining paths are safe.
