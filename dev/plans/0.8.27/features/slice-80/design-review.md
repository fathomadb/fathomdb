---
title: FathomDB 0.8.27 Slice 80 - design review
status: PASS
target_release: 0.8.27
---

# Slice 80 design review

The reviewer is an independent, read-only, adversarial subagent (Opus 5.5,
high effort).

## Cycle 1 — FAIL at `e6601c67`

| # | Severity | Finding | Resolution |
| --- | --- | --- | --- |
| 1 | P1 | Moving the reader-pool data carriers would widen dozens of private fields, which PW27-4A forbids. They are built at root, in `graph_expand`, and in Slice 90's WAL seams. | Only the pool's functions move. The carriers stay at root, where a child module can read their private fields. The carriers go to Slice 90. |
| 2 | P2 | The Windows WAL guard reads `WalConnectionInventory` and the reader-completion pause needle, which move. | Retarget through an injectable `READER_POOL_SOURCE`, following the `36fc2352` precedent, with fixtures injecting it. |
| 3 | P2 | `plan-0.8.20.md` also cites `pub fn search_filtered` in `lib.rs`. | Retargeted in batch 9. |
| 4 | P2 | C1 self-test arms 12p and 12w edit `SearchHit` in `lib.rs`. | Pointed at `search_types.rs` in batch 4. |
| 5 | P2 | `bfs_graph_arm_candidates` is search code; placing it under `graph_expand` makes a cycle. | Moved to `search.rs` (batch 6), and plan item 5 was corrected. |
| 6 | P2 | The WAL and diagnostic arms are inline in the worker loop, not root executors. | Stated. The loop moves whole and verbatim, and the seam is handed to Slice 90. |
| 7 | P2 | With three passes over the pool, a leaked transaction fails at step 3, not at erasure. | Exactly one refusal per worker (8, round-robin), guarded by a debug worker-count assert. |
| 8 | P3 | Plan and design disagreed on constant placement; `append_jsonl` and `branch_str` are used only by telemetry. | Limits go to `search_types.rs`; `append_jsonl` and `branch_str` go to `telemetry.rs`. |
| 9 | P3 | The `Reader*Pause` aliases and `Engine::usable_dense_runtime` are root and runtime helpers. | Both stay at root. |
| 10 | P3 | Graph split line ranges; a `concat!` can list only existing files. | Types take lines 24-25, and the codec takes `is_false` (26-28). Each new module is appended to the `concat!` gates in its own batch. |
| 11 | P3 | Sibling import strategy was unstated. | Root re-imports keep sibling paths, so sibling files are untouched. |
| 12 | P3 | Coverage gaps: the `tc5-benchmark` route, error precedence, TC-38 view paths, and the `perf_gates` comment anchors. | Each is added to the design. |

## Cycle 2 — PASS-WITH-FIXES at `810ff0f2`

All 12 cycle-1 resolutions were verified against the code:

- **Reader pool:** the functions-only layout compiles with no field
  widening. The required `pub(crate)` methods are listed.
- **WAL guard retarget:** the `READER_POOL_SOURCE` retarget is sufficient.
- **Graph arm and batches:** `bfs_graph_arm_candidates` in `search.rs` and
  the 15-batch order are compile-feasible, and all batches are within
  300-1,200 lines.
- **Characterization test:** it is sound. Dispatch is strict per-request
  round-robin, the refusal happens inside the reader transaction, and the
  mutant fails at erasure with bounded retries.

Five P3 items were closed without another cycle:

- `CacheStatusReply` was removed from the moved hidden items.
- The `concat!` gate scope is now precise: the slice35 manifest takes only
  modules extracted from `lib.rs`.
- The two open-path reader helpers stay at root, so they need no
  `pub(crate)`.
- A `next_reader_worker_index_for_test` advance-by-8 guard was added.
  *Superseded in cycle 3:* the index is modulo 8, so an endpoint check is
  vacuous; the guard is now a per-dispatch progression.
- The pool's `*_for_test` methods are recorded as moving with the impl.

## External review — FAIL at `06e748c2`

An external review found four issues. All were checked against the code and
accepted:

| # | Severity | Finding | Resolution |
| --- | --- | --- | --- |
| E1 | P1 | Leaving `graph_evidence_request_tests` in `execution.rs` renames both tests, which the hidden baseline forbids. | The test module goes to `graph_expand/mod.rs`; names are checked in batch 12. |
| E2 | P1 | R27-80F required every `ReadView` axis, but search refuses existence relaxation by design. Graph-arm and projected-text view coverage was claimed but absent. | R27-80F restated as validity axis plus typed refusal. Two mandatory characterization tests with mutants. |
| E3 | P2 | `next_reader_worker_index_for_test` is modulo 8, so "advances by 8" is vacuous. | Per-dispatch progression `(start + i) % 8`. |
| E4 | P2 | The `*_for_test` non-goal contradicted the moving inventory. | Scoped to `Engine` seams and root wrappers, with named exceptions. |

A dependency-direction invariant was also added to the design.

## Cycle 3 — PASS-WITH-FIXES on the external-review revision

Confirmed:

- both search entry points refuse existence relaxation;
- projected text has one validity site;
- the graph-arm fixture is feasible;
- dispatch is the only increment of the routing counter;
- the `mod.rs` placement keeps the baseline test names.

| # | Severity | Finding | Resolution |
| --- | --- | --- | --- |
| 1 | P1 | The graph arm applies neighbor validity twice (edge-query `target_node` and hydration `body_validity`), so a one-site mutant survives. | The mutant removes both sites and keeps bound parameters referenced. The seed and resolve sites are out of scope. |
| 2 | P1 | `TraversalDirection`, assigned to `traversal.rs`, is used by `types.rs` and `codec.rs`. The codec also uses filter, search-type, and evidence carriers. | `TraversalDirection` moves to `graph_expand/types.rs`. The codec rule is restated: types plus crate value and carrier types, never `execution` or `traversal` functions. |
| 3 | P2 | The `search` rule was a closed list and hid two existing cycles (`search` ↔ `graph_expand` and `search` ↔ `reader_pool`). | Reworded to exclude only `search_api` and `telemetry`. Both cycles are kept verbatim as Slice 90 seams, and no new cycle may be added. |
| 4 | P2 | Test-only imports in `mod.rs` would be unused in the lib build and fail Clippy with warnings denied. | They are `#[cfg(test)] use` lines, `encode_graph_evidence_request` becomes `pub(super)`, and re-exported names are not imported again. |
| 5 | P2 | The existing `graph_expand.rs` `*_for_test` seams move with the file, but neither non-goal listed them. | Listed as exceptions in both files. |
| 6 | P3 | `slice50_evidence` does run the graph arm under `valid_as_of`. | The claim is reworded: no owner asserts window exclusion there. |
| 7 | P3 | The assertion-3 fixture needs the seed's window and the edge's `t_invalid` to cover the instant. The cycle-2 record of the advance-by-8 guard is stale. | The fixture is pinned in the design, and the record is marked superseded. |

All fixes are mechanical corrections of fact, so no further cycle was run.
The status stays PASS.

Implementation may start once Slice 80 is commissioned.
