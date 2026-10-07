---
title: Slice 135 search row-error follow-up audit
status: SOURCE_AUDIT_PROBES_PENDING
target_release: 0.8.27
---

# Search row-error follow-up audit — 2026-10-07

This is a source audit of `search.rs` at Git
`5f2f2cc29359fb190995de94a369b8544d274d94`, file SHA-256
`9cf58338b27f93295588c53d9aa89f44944d28bfe45bb3338872943406d4c24f`.
It extends the [first logic result](logic-first-result-2026-10-06.md) after
the node and edge FTS row-error repairs. It is not a dynamic defect finding
for the remaining sites or a complete exception-path audit.

The current file has four `rows.flatten()` sites. A `rusqlite` row iterator
contains `Result<Row, Error>`; iterator flattening drops an `Err`. That is
hazardous when the operation promises a complete answer or truthful error.
The current node and edge FTS paths instead collect into
`rusqlite::Result<Vec<_>>` and propagate decoding failures.

| Site | Result that can be lost | Current classification | Next discriminator |
| --- | --- | --- | --- |
| `search.rs:1545` vector phase-1 candidates | A `rowid` or distance decode error can omit a nearest-neighbor candidate before hydration. | Potential incomplete-result defect on an active vector path. No malformed vec0 row has yet been shown to reach this decoder. | Create a real-database fault fixture that makes the selected row fail conversion while the candidate remains physically queryable; require `Storage`, then repair only if the RED assertion is reachable. |
| `search.rs:1987` pre-step-12 node fallback with `source_id` | A row error can remove a text hit. | Unsafe iterator shape in a legacy-schema branch. Normal open migrates before search; reachability on a supported runtime database needs proof. | Exercise an explicitly supported historical database through open/migrate and record the selected route. If reachable, inject a row conversion failure and require a typed error. |
| `search.rs:2011` pre-step-8 node fallback | A row error can remove a text hit. | Same legacy-path question; the missing `source_id` column is the intended fallback trigger, not a license to discard row errors. | Use a pre-step-8 fixture and route witness, then a malformed matching FTS row if the branch remains reachable. |
| `search.rs:2104` edge attribute explanation count | A failed cursor decode can undercount `dropped_edge_hits`; outer `if let Ok` also suppresses statement/query errors. | Explanation-integrity lead. The default result set is unchanged, but an opt-in diagnostic can report a false count. | With explanation and an attribute filter active, inject a malformed matching edge cursor or a named SQL failure and assert whether the API returns a typed error or a documented unavailable explanation. |

The source also has `rank_stream_candidates` ending in `.ok()` near line 1870.
That fallback to full sorting is intentional only if the fallback produces
the same complete answer. Existing rank-stream tests compare ordinary
result ordering, limits, and selected routes; fault-specific equivalence
remains unmeasured. The four sites above are an inventory, not four confirmed
bugs. The previous real-database RED/GREEN probes confirmed **two** FTS row
errors and a distinct missing-edge-index statement fallback; those receipts
remain the evidence for repaired defects.

For this checkpoint, run the active vector-path probe before considering a
generic Semgrep prohibition on `.flatten()`: most other uses in the crate
flatten `Option`, not iterator `Result`, and a broad rule would be noisy.
A focused semantic rule or compiler lint can later flag iterator flattening
of `Result` in database row loops, paired with reviewed exceptions. Existing
Clippy and typechecking did not flag the two confirmed FTS defects. Their
absence is not evidence that these four paths are safe.
