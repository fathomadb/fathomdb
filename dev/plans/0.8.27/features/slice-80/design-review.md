---
title: FathomDB 0.8.27 Slice 80 - design review
status: PASS
target_release: 0.8.27
---

# Slice 80 design review

Cycles 1 and 2 were performed by an independent, read-only, adversarial
subagent using Opus 5.5 at high effort. The later external code-grounded review
and cycle 3 were a separate independent review; this record does not infer a
model identity that was not captured with that review.

## Cycle 1 — FAIL at `e6601c67`

| # | Severity | Finding | Resolution |
| --- | --- | --- | --- |
| 1 | P1 | Moving the reader-pool data carriers would widen dozens of private fields, which PW27-4A forbids. They are built at root, in `graph_expand`, and in Slice 90's WAL seams. | **Historical cycle-1 resolution; superseded by the planning FIX-1 below.** Only the pool's functions move. The carriers stay at root, where a child module can read their private fields. They were then allocated to Slice 90. |
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
| 3 | P2 | The `search` rule was a closed list and hid two existing cycles (`search` ↔ `graph_expand` and `search` ↔ `reader_pool`). | Cycle 3 reworded the rule and initially accepted both seams. Implementation review later rejected the pool cycle: `begin_attributed_reader_tx` stays at the root, so only `search` ↔ `graph_expand` remains. |
| 4 | P2 | Test-only imports in `mod.rs` would be unused in the lib build and fail Clippy with warnings denied. | They are `#[cfg(test)] use` lines, `encode_graph_evidence_request` becomes `pub(super)`, and re-exported names are not imported again. |
| 5 | P2 | The existing `graph_expand.rs` `*_for_test` seams move with the file, but neither non-goal listed them. | Listed as exceptions in both files. |
| 6 | P3 | `slice50_evidence` does run the graph arm under `valid_as_of`. | The claim is reworded: no owner asserts window exclusion there. |
| 7 | P3 | The assertion-3 fixture needs the seed's window and the edge's `t_invalid` to cover the instant. The cycle-2 record of the advance-by-8 guard is stale. | The fixture is pinned in the design, and the record is marked superseded. |

All fixes are mechanical corrections of fact, so no further cycle was run.
The status stays PASS.

Implementation may start once Slice 80 is commissioned.

## Post-hoc adversarial design review (2026-09-27)

A read-only, adversarial review re-checked the recorded design claims against
the implementation. It covered range `9a31e979..a68e90b7` against baseline
production `bb077cfa`.

- **Cycle 1 verdict:** PASS-WITH-FIXES.
- **Behavior preservation:** confirmed mechanically. Of 12,413 moved
  non-blank lines, only 8 are not verbatim, and each is explained. The
  attribute sets are identical before and after, and no SQL, error mapping,
  or feature gate changed.
- **Where the defects lie:** in the recorded claims, not in behavior.

> **Correction to cycle 3 row 3.** The resolution text of row 3 was rewritten
> after implementation. It is marked "later", but it still said "only
> `search` ↔ `graph_expand` remains". That is false at module level; see
> finding 2 below. The row is left as written, and this section is the
> correction of record.

| # | Severity | Finding | Resolution |
| --- | --- | --- | --- |
| 1 | P1 | Four struct fields widened to `pub(crate)`, violating PW27-4A: `TelemetrySink.path` and the three `EvidenceCapture` fields. The records claimed no widening. | **Historical fix-1 resolution; its later-slice allocation is superseded below.** Fixed in `b8af4d86`: both struct definitions returned unchanged to crate root with private fields, following the carriers-stay-at-root rule. `tdd-chronology.md`, `design.md`, and the Slice 90 section of the master plan were corrected. |
| 2 | P2 | The dependency invariant is false at module level. Beyond `search` ↔ `graph_expand`, the design's facade placement closes three cycles: `read` ↔ `reader_pool`, `graph_expand` ↔ `reader_pool`, and `graph_expand` ↔ `search_api`. | **Historical resolution; superseded by the owner directive below.** Records were corrected without moving code: `design.md` restated the facade-vs-handler invariant and handed three accepted seams to Slice 90; `code-review.md` received a correction note. |
| 3 | P2 | The Slice 140 handoff for relocated test seams was missing from the master plan. | **Fixed.** Slice 140 now has a "Carried from Slice 80" block that names each relocated seam and its file. |
| 4 | P2 | Live AC-037 evidence was not recorded per the runbook: no pass lines, no grant, and no revert. The host is restricted today. | **Recorded as unevidenced.** No captured run output was found. `review-verification.md` and `status.md` now mark the live AC-037 layer at the candidate as UNEVIDENCED, which does not count as a pass. A re-run needs a HITL grant through the runbook. The runbook History records this. |
| 5 | P3 | The WAL guard retarget bundled lib.rs-only needles into a check on `READER_POOL_SOURCE`, and `assert_contains` checked only its first needle. | **Fixed** in `b8af4d86`. The needles are split, and `assert_contains` checks every needle. One latent vacuous needle in the PyO3 check is corrected. The fixture passes 314/314. |
| 6 | P3 | Items were wider than their users need (the finding says six but lists seven). | **Partly fixed** in `b8af4d86`: six items are now private. The seventh, `GraphExpansionErrorV1::new`, is **rebutted**: `graph_expand/codec.rs` calls it at lines 207, 214, 1000, and 1007, so `pub(super)` is required. |
| 7 | P3 | A `perf_gates.rs` comment pointed at `search.rs`. | **Fixed:** the comment now points at `filter.rs::build_vector_phase1_sql`. |
| 8 | P3 | The whole reader-release test file was gated on `debug_assertions`. | **Fixed.** Only the debug-only guards are gated, the assertions are unchanged, and the test passes in both debug and release. |
| 9 | P3 | Batch 1 moved 264 lines, below the AC27-80A floor of 300. | **Recorded** as a deviation in `tdd-chronology.md`. |
| 10 | P3 | A blanket `#[allow(unused_imports)]` sat on a re-export list. | **Fixed.** The allow is removed and the three unused `*ForTest` re-exports are trimmed. None was root-exported before the move. |
| 11 | P3 | A review record was rewritten after the fact (cycle 3 row 3). | **Folded into finding 2.** See the correction note above. |

### Post-hoc cycle 2 — PASS-WITH-FIXES at `2121690e`

All 11 cycle-1 resolutions were verified against code; the
`GraphExpansionErrorV1::new` rebuttal is correct (`codec.rs` calls it). The
FIX-1 diff changes no behavior, widens nothing, and leaves public and hidden
surfaces unchanged. A rebuilt module graph confirms the four recorded cycles
are complete among Slice 80's modules. Three P3 wording findings were closed
in FIX-2 without another cycle:

| # | Severity | Finding | Resolution |
| --- | --- | --- | --- |
| N1 | P3 | `tdd-chronology.md` still claimed live AC-037 passed. | Qualified as UNEVIDENCED. |
| N2 | P3 | The `STATUS-0.8.27.md` Slice 80 row was unqualified. | Qualified as UNEVIDENCED. |
| N3 | P3 | The cycle list read as global; three inherited cycles with earlier-slice modules were unrecorded. | **Historical resolution; allocation superseded below.** The list was scoped to Slice 80 modules, and the inherited cycles were recorded as Slice 90 context. |

The Slice 80 implementation design review is closed.

## Slice 85 planning FIX-1 (2026-09-27) — pending rereview

The independent review of the planning-only Slice 85 insertion found two
architectural defects. FIX-1 corrects the plan and all current allocations;
the original Slice 80 finding text above remains historical evidence.

| # | Severity | Finding | FIX-1 disposition |
| --- | --- | --- | --- |
| 1 | P1 | R27-85A allowed carriers to remain indefinitely at crate root and therefore did not establish semantic ownership. | Every root-kept reader carrier, `TelemetrySink`, `EvidenceCapture`, and `begin_attributed_reader_tx` must receive non-root semantic ownership with private fields and rooted contracts preserved. Root retention requires an item-specific reviewed exception proving it is durable and protects a stronger invariant. |
| 2 | P1 | The plan treated `search` ↔ `graph_expand` as accepted while requiring only the other three Slice 80 cycles to be removed. | Slice 85 must eliminate all four Slice 80 cycles. Only the three named inherited earlier-slice cycles are initially eligible for a narrow allowlist; any other retained cycle requires design review to prove it unavoidable. |

This FIX-1 does not rebind the authoritative Slice 80 SHA or claim a Slice 85
design-review pass. Independent rereview is required.
