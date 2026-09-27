---
title: FathomDB 0.8.27 Slice 80 - engine read, search, graph, and evidence domains
status: COMPLETE
target_release: 0.8.27
baseline_sha: bb077cfa
---

# Slice 80 plan

Design review passed after three cycles plus an external code-grounded review
(`design-review.md`). Slice 80 was commissioned by the repository owner on
2026-09-27. This plan supersedes the
master-plan draft (`dev/plans/plan-0.8.27.md`,
"Slice 80 — read, search, graph, and evidence") as execution authority once
design review passes. `design.md` is the authority for the exact inventory,
batches, gate retargets, and evidence. Where the two differ, `design.md`
wins. The review record is `design-review.md`.

## Outcome

This slice moves the root-owned read side of `fathomdb-engine/src/lib.rs`
(19,001 lines at `bb077cfa`) into private semantic modules. The read side
covers the reader pool, filters, ranking and fusion, search execution and
its result types, the search and read `Engine` facades, telemetry capture,
read verbs, and graph traversal. It also splits the 3,525-line
`graph_expand.rs` along its measured dependency boundaries.

The following do not change:

- behavior, schema (34), SQL text, and statement order;
- snapshot and transaction scope, and reader dispatch;
- error mapping and feature gates;
- public paths and wire encodings.

This is a behavior-preserving structural slice. It fixes no known defect. If
a characterization test exposes a semantic defect, the move stops for a
separate RED/GREEN correction.

Execution is direct, in the release worktree, with one writer. No Steward or
Orchestrator role is used.

## Reconciliation since the draft

The draft was written in prework at `a3e6cff6`. The baseline is now
`bb077cfa`. Material changes:

1. **Slices 40-70 moved most non-read domains out of `lib.rs`.** They took
   errors, identity, temporal, test hooks, lifecycle, provenance,
   dependency, erasure, write, ingest, consolidation, projection, vector,
   mean, embedding, and rerank. What remains in `lib.rs` is the read side,
   the open, runtime, and operator facade (Slice 90), the write/search index
   projectors, and the `*_for_test` seams (Slice 140).
2. **Slice 70 deferred items to this slice.** The deferred items are:
   - fusion (`fuse_rrf`, `fuse_three_arms`, `RRF_*`, `RECENCY_WEIGHT`, and
     the reweights);
   - the search-limit constants;
   - `vector_filter_*`;
   - `read_search_in_tx`;
   - `branch_str` and `append_jsonl`;
   - an owner decision for the four search-owned fields on
     `ProjectionRuntimeShared`.

   It also left a per-batch rule: run every source-scraping gate in each
   batch.
3. **Existing semantic destinations are named first (PW27-4A).**
   `graph_expand.rs`, `evidence.rs`, `frozen_read.rs`, `pagination.rs`, and
   `dependency_trace.rs` already own their domains. `evidence.rs` (2,988
   lines), `frozen_read.rs`, `pagination.rs`, and `dependency_trace.rs` stay
   as they are; only their imports change. Legacy traversal extends
   `graph_expand` instead of creating a parallel graph module.
4. **The `graph_expand.rs` split is supported, but not as the draft drew
   it.**
   - **Measured dependencies:** types → validation → execution, and types →
     codec. There are no cycles. The request and result wire codec (about
     1,550 lines) depends only on types and crate value types, never on
     `EngineError`, `Connection`, or `frozen_read`.
   - **Where validation goes:** validation returns `EngineError` and only
     execution uses it. It therefore goes with execution, not with types.
     The draft's "types/validation" pairing would give the pure types file
     an engine dependency.
   - **What is approved:** a directory module with `types`, `codec`,
     `execution` (including validation), and `traversal` (see item 5).
5. **Legacy traversal is graph code.** That covers `graph_neighbors`,
   `search_expand`, and the BFS SQL builders, which move to
   `graph_expand/traversal.rs`. The search graph arm is not graph code.
   `bfs_graph_arm_candidates` is generic over search's capture trait and
   uses search result types, not the BFS builders, so it moves to
   `search.rs`. Putting it under `graph_expand` would create a cycle.
6. **Reader-pool data carriers stay at root.**
   - **The rule:** PW27-4A forbids field widening.
   - **Why it matters here:** the pool's data carriers
     (`ReaderWorkerPool`, `SearchReaderWork`, the request structs and enum,
     and `FrozenQueryRuntime`) have private fields. Those fields are built
     at root, in `graph_expand`, and in Slice 90's WAL seams.
   - **What moves:** only the pool's functions move to `reader_pool.rs`.
     That includes the worker loop, whole, with its inline WAL and
     diagnostic arms.
   - **What Slice 90 gets:** the carriers and the extraction of the WAL
     arms, as an explicit hand-off.
7. **The draft's test obligations are already met by existing owners.**
   These include snapshot authority under concurrent mutation, eligibility
   before every bounded cap, deterministic tie ordering, codec round-trip
   and corruption properties (proptest round-trips in `slice60_wire` and
   `slice55_wire` cover the graph request and trace codecs; the graph result
   codec and corruption refusal are fixture-covered only — a pre-existing gap
   logged as `TC-aa4bea08-f281-47eb-8022-d63250d1daac`), and SQL/plan
   assertions. `design.md` maps each owner.
   Characterization is limited to gaps that the mapping proves.
   - **TC-38 (search visibility):** the hybrid, text-only, filtered, and
     explained paths are covered by `slice15b_search_validity` and
     `opp12_existence_axis`. Search honors only the validity axis
     (`valid_as_of`, `include_out_of_window`) and refuses existence
     relaxation (`include_superseded`, `include_inactive`) with
     `InvalidArgument`. No owner asserts validity exclusion or inclusion on
     the graph arm or on `search_projected_text` under a non-default view;
     every existing `search_projected_text` call uses
     `ReadView::default()`.
   - **The proven gaps:** reader transaction release after a refusal inside
     the transaction, and the two view paths above.
8. **Graph and evidence tests stay together.** The in-crate evidence tests
   call `graph_expand` execution internals directly. That confirms the
   draft's "keep graph and evidence tests together". No test is split.
9. **More source-scraping gates name moved items.** The design lists each
   retarget:
   - `tests/slice60_fix1_wire.rs` reads `graph_expand.rs` by path;
   - `scripts/check-c1-conformance.sh` reads `SearchHit` from `lib.rs`;
   - the ACTIVE `dev/plans/plan-0.8.20.md` cites `text_hit_passes_filter`
     and `edge_fts_hit_passes_filter` in `lib.rs`.

   Further retargets:
   - the Windows WAL guard reads a reader-completion needle and
     `WalConnectionInventory` from `lib.rs`, so it needs a
     `READER_POOL_SOURCE` retarget when the pool functions move;
   - the C1 self-test fixtures edit `SearchHit` in `lib.rs`;
   - `plan-0.8.20.md` also cites `pub fn search_filtered`.
10. **The remaining write/search index projectors are not read code.** These
    are `project_canonical_*_row`, `index_targets_for_row_kind`, the
    tokenizer reproject, and `canonical_node_rows`, all called from write,
    projection, and open. They stay at root and are allocated to Slice 90's
    facade closure.
11. **Frozen snapshot leases are out of scope.** They stay postponed to
    0.8.28 (D28-05).

## Decision: the four search-owned runtime fields

The fields are `search_limit_override`, `recency_reweight_enabled`,
`importance_reweight_enabled`, and `vector_stage_only_for_test`. They
**stay on `ProjectionRuntimeShared` with its shape unchanged.**

- **What they are:** per-engine atomics. The `*_for_test` seams set them,
  and search reads them into `SearchReaderWork` at dispatch.
- **Why they stay:**
  - Moving them to `Engine` state would change two struct shapes and touch
    the test seams, which are Slice 140 territory.
  - The move buys no behavior.
  - Slice 90 finalizes `Engine`'s state, and it may relocate them then, as
    part of the facade closure, with its own design review.
- **What records the decision:** `design.md` records it. Search reads the
  fields through the unchanged `projection_runtime.shared` path.

## Assigned inventory (summary)

`design.md` holds the by-name inventory and the batches. Destinations:

| Destination (private) | Owns |
| --- | --- |
| `fusion.rs` | `RRF_*`, `RECENCY_WEIGHT`, `fuse_rrf`, `fuse_three_arms`, the reweights, and the importance maps |
| `filter.rs` | `ScalarValue`, `ComparisonOp`, `Predicate`, `SearchFilter`, `FilterTerm`, `Filter`, filter validation, the SQL eligibility and rank builders, and the hit post-filters |
| `search_types.rs` | the public search result, explanation, trace, and fallback types, the BM25F plan types, and the search-limit constants and validation |
| `search.rs` | `read_search_in_tx` and its capture, statement, and rank-boundary helpers, the search graph arm, the `test-hooks` search witnesses, and BM25F execution |
| `search_api.rs` | the `impl Engine` search, frozen-search, and evidence facades |
| `telemetry.rs` | `TelemetrySink`, `append_jsonl`, `branch_str`, and search observability, telemetry, and feedback |
| `read.rs` | the read-verb, canonical-page, and operational-state in-transaction functions, and their `impl Engine` facades |
| `reader_pool.rs` | the pool's functions: its impl and `Drop` (including its two `*_for_test` methods), the worker loop, `finish_reader_request`, and the reader cache and lookaside probes (the data carriers, the shared `begin_attributed_reader_tx` primitive, and the open-path connection setup stay at root) |
| `graph_expand/` (split) | `mod.rs` re-exports and the in-file `graph_evidence_request_tests` (qualified names unchanged); `types.rs`; `codec.rs`; `execution.rs` (with validation and `test-hooks` seams); `traversal.rs` (legacy traversal, the BFS builders, and their `impl Engine` facades) |
| Stays at root | `Engine`; the open, runtime, WAL, and operator facade, the reader data carriers and shared private `begin_attributed_reader_tx` primitive, `usable_dense_runtime`, and the index projectors (Slice 90); every `*_for_test` Engine seam and `pub fn *_for_test` wrapper (Slice 140); `hex_encode`; `RowKind` |

Public items keep their exact root paths through `pub use`. Doc-hidden items
keep their hidden status.

## Requirements and acceptance

The release-local R27-04/R27-05 and AC27-07/08 remain authoritative. This
slice adds:

| ID | Requirement | Acceptance |
| --- | --- | --- |
| R27-80A | Exact structural scope. | AC27-80A: the private modules own exactly the approved inventory. Batches move 300-1,200 lines each. Existing modules are extended first. The `graph_expand` split follows the measured dependency map. There is no public, schema, wire, SQL, or feature-gate change. |
| R27-80B | Snapshot authority and transaction lifetime are unchanged. | AC27-80B: the mapped snapshot-race, linearization, and reader-pool owners pass unchanged. A reader transaction is released after both success and refusal (tested if the mapping finds it uncovered). |
| R27-80C | Eligibility precedes every bounded cap. | AC27-80C: the mapped pretruncation, frontier, page, and graph owners pass, including their SQL and query-plan assertions. |
| R27-80D | Ordering and fusion are unchanged. | AC27-80D: the RRF, three-arm, reweight, tie-ordering, and prefix-stability owners pass. Graph response bytes stay identical across insertion permutations. |
| R27-80E | Codecs are byte-stable. | AC27-80E: the graph request and result codecs, and the evidence, frozen, pagination, and trace codecs, round-trip and refuse corruption. The existing proptests and canonical fixtures pass unchanged. |
| R27-80F | Search keeps its shipped view contract and the filter grammar: it applies the validity axis (`valid_as_of`, `include_out_of_window`) and refuses existence relaxation (`include_superseded`, `include_inactive`) with a typed `InvalidArgument`. | AC27-80F: the filter-grammar, unification, `slice15b_search_validity`, and `opp12_existence_axis` owners pass. Two mandatory characterization tests, each proven non-vacuous with a recorded mutant, cover the paths without an owner: the graph arm and `search_projected_text` each hide an out-of-window node under the default view, return it under `include_out_of_window` and under a `valid_as_of` inside its window, and refuse existence relaxation with `InvalidArgument`. |
| R27-80G | The search runtime fields keep their owner. | AC27-80G: `ProjectionRuntimeShared` is unchanged in shape, and search reads the same fields. |
| R27-80H | Structural and feature evidence. | AC27-80H: the public surface equals the Slice 30 baseline, and the hidden surface is additive only. Every named source-scraping gate is retargeted path-only and passes in every batch. The focused feature routes pass. The final gates pass: the canonical gate, workspace Clippy with warnings denied, `cargo check --workspace --all-targets`, the Python receipt, strict security with live AC-037 (through `dev/release/ac-037-live-netns-hitl-runbook.md`), and `scripts/test-feature-complete.sh` on the RTX 3090 host (search calls the feature-gated reranker and embedder). |

## TDD implementation sequence

Use one writer in the release worktree. Read-only reviewers share it.

0. **Commission.** Record the `slice-80-execution` ruling in
   `release-state-0.8.27.json`, set Slice 80 to `IN_PROGRESS` with this
   plan, design, design review, and TDD chronology in `design_refs`, and
   regenerate the views.
1. **Design and review.** Iterate on `design.md` with an independent design
   reviewer until it passes. The completed three-cycle record, including the
   external code-grounded review, is in `design-review.md`.
2. **Map owners and characterize.** Map each existing owner to R27-80B-F in
   `tdd-chronology.md`. Characterization covers:
   - **The reader-release test:** add it, show that it passes on unmodified
     production, kill a recorded temporary mutant, and restore the code.
   - **The two view-path tests (R27-80F):** add them, show that they pass
     on unmodified production, kill a recorded temporary mutant for each
     (as `design.md` defines them: both neighbor-path validity sites on the
     graph arm, and the single site in projected text),
     and restore the code.

   These are characterization-first tests: GREEN on unmodified production,
   RED against the specified temporary mutant, then GREEN again after exact
   restoration. Record commands, observed failures, and restoration evidence
   in `tdd-chronology.md`. If a characterization test instead fails on
   unmodified production, treat it as a genuine behavioral RED and stop the
   structural move until the smallest production fix makes it GREEN.
3. **Pre-move receipt.**
   - Take the public and hidden captures at the characterization commit
     (Node `v25.9.0`, at least 100 GB free).
   - Record baseline route counts using the extended per-batch check script
     (`design.md`).
4. **Move in the design's batches**, one commit each, copied verbatim. Each
   batch retargets, path-only, every gate that names what it moved. Each
   batch then runs the per-batch check, which covers:
   - feature-route checks and Clippy;
   - focused owners, with counts equal to the baseline;
   - the slice35 manifest and audit, the C1 gate and its self-test, the
     Windows WAL guard, `slice60_fix1_wire`, and plan anchors.

   Heavy captures run after the batches that add root re-exports and at the
   final candidate.
5. **Refactor within the boundaries.** Imports and visibility only. Every
   behavioral correction follows RED → GREEN: first commit or otherwise
   preserve the failing test, then make the smallest production change, then
   refactor without weakening the oracle. Record the chronology.
6. **Code review.** An independent read-only code-review subagent using
   `gpt-5.6-sol` with `high` reasoning reviews the whole diff. Behavioral
   findings are closed with a recorded RED/GREEN cycle. The review and finding
   dispositions are recorded in `code-review.md`.
7. **Verify.** After implementation and code-review closure, a separate
   read-only test/verification subagent using `gpt-5.6-terra` runs the final
   gates in AC27-80H and records them in `review-verification.md`:
   - The live AC-037 layer follows the HITL runbook: grant, check, revert.
   - Anything unavailable is recorded as unavailable, never as a pass.
8. **Close.**
   - Write `status.md`.
   - Set Slice 80 to `COMPLETE_ON_RELEASE_BRANCH` with its SHAs, advance
     `next_slice` to 90, and regenerate the views.
   - Record any carry-overs in the master plan's Slice 90, 140, or 150
     sections.
   - Clean up scratch material and caches whose ownership is proven. No
     push, tag, or publication.

## Non-goals

- No change to reader dispatch, snapshot or lease semantics, fusion
  arithmetic, ranking, or codec bytes.
- No new snapshot leases (0.8.28 D28-05).
- No move of the open, runtime, WAL, or operator facade, or of the index
  projectors (Slice 90). No move of `*_for_test` `Engine` seams or root
  `pub fn *_for_test` wrappers (Slice 140). The exceptions move with their
  owners, as `design.md` lists: the reader pool's
  `wal_connection_inventory_for_test` and
  `wal_native_state_inventory_for_test`, the `Filter` impl's test methods,
  the `test-hooks` search witnesses in `search.rs`, and the seams already
  in `graph_expand.rs` (`measure_graph_expand_for_test`, the
  `seed_graph_expand_*_for_test` and `graph_expand_with_*_for_test`
  families, `explain_graph_expand_for_test`, and
  `graph_expansion_degradation_codes_for_test`), which move with the file
  into `graph_expand/execution.rs`.
- No split of `evidence.rs`, `frozen_read.rs`, `pagination.rs`, or
  `dependency_trace.rs`, and no split of graph and evidence tests. No
  existing test changes its qualified name.
- No change to `ProjectionRuntimeShared`'s shape.
- No binding or SDK change.
- No duplicate of existing owner tests.
