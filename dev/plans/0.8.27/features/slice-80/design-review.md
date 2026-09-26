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
- The pool's `*_for_test` methods are recorded as moving with the impl.

Implementation may start once Slice 80 is commissioned.
