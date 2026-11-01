---
title: FathomDB 0.8.28 Slice 50 — requirements and acceptance criteria
status: design investigation; built-in-provider replacement proposed; no implementation authorized
target_release: 0.8.28
observed_on: 2026-11-01
---

# Slice 50 requirements and acceptance criteria

> **Current direction (2026-11-01):** owner scope `seq-355` limits production
> providers to the built-in implementation before 1.x. Read the revised
> [user needs, requirements and AC](built-in-provider-requirements.md),
> [design and extension inventory](built-in-provider-design.md), and
> [design work plan](built-in-provider-plan.md) before the original phase-1
> material below. Unit S is unchanged. Unit D / D50-1 / D50-2 are historical,
> unselected options, held without implementation; their recommendations are
> not the current proposal. The replacement contract and release placement
> require owner review before production changes.

Slice 50 (`DRAIN-SCAN-BOUND`) has two units:

- **Unit S** bounds the two pending-work scans, so drain cost is linear in
  the backlog.
- **Unit D** stops `Engine::drain` waiting for vector work that a session
  with a refused dense runtime can never do (todo 289).

[design.md](design.md) gives the changes and [test-plan.md](test-plan.md)
the runs. Line numbers refer to the slice base `544bf74f8` (`dev/0.8.28`).

## 1. Evidence

### 1.1 Unit S: the scans read the whole backlog

- `next_pending_projection_jobs` (`projection_worker.rs` 1017) is the
  dispatcher's scheduler scan. `pending_projection_candidate_page` (898)
  is the candidate-page query. The candidate-page query serves
  `connection_has_pending_projection_work` (1165, the probe in `drain`'s
  `wait_for_idle`), `pending_embedding_work` (1198, readiness counts),
  `derive_dense_readiness` and the open-time wake check.
- Each query is a `UNION ALL` of a node arm and an edge arm inside a
  subquery, under an outer `ORDER BY pending.write_cursor LIMIT 256`.
- `EXPLAIN QUERY PLAN` on this branch (SQLite 3.54.1, scratch probe
  2026-10-31, design § 1.3) shows that both arms already read their
  `write_cursor` index in order. The subquery runs as `CO-ROUTINE pending`
  and feeds a top-level `USE TEMP B-TREE FOR ORDER BY`. The sort consumes
  every pending row past the cursor before `LIMIT 256` applies.
- In the same probe, one page cost 38,110 VM steps at 1,000 pending nodes
  and 287,110 at 8,000 (7.5×). With a per-arm `ORDER BY write_cursor
  LIMIT 256` the same pages cost 20,825 and 28,394 VM steps. Every page,
  over 496 page positions, was identical between the two forms.
- Prototype on `exp/0.8.29-ac013-scale-drain` (`855d29155`, results
  `aa30085fc`). The dispatcher scan runs about once per 64-row batch, so
  a held drain costs O(N²). Under 64-row vec0 chunks (CS64, Slice 60's
  layout) the 100k held drain fell from 33.4 s to 5.7 s, and the store
  digests matched.
- **Correction to the master plan figure.** 33 s to 5.7 s is a CS64
  number. This slice lands on format 34 (1,024-row chunks, default 2 MB
  runtime cache): the prototype's `BASE` cell. Shipped `BASE` measured
  327, 347 and 468 µs per vector at 10k, 30k and 100k (47 s at 100k).
  `BASE` with `armlimit` was measured only with a 4 MB runtime cache
  (82, 81, 80 µs per vector). The format-34, default-cache cell with the
  fix has not been measured. AC50-08 measures it under a rule stated
  before the run.

### 1.2 Unit D: drain on a dense-disabled store (todo 289)

Todos ledger seq 289 on `dev/0.8.29`
(`TC-06cf0604-f36f-465d-8e82-5e88d10acac0`):

> runtime_lifecycle.rs drain() waits in wait_for_idle on
> connection_has_pending_projection_work, which never clears when the
> dense side is refused (nothing embeds the rows). fathomdb-mcp calls
> engine.drain(call_deadline_ms) on idle close, store switch and
> shutdown, so each close of such a store stalls the full deadline (10 s
> default) plus 5 s. Fix in the engine: treat pending vector rows on a
> dense-disabled store as not drainable.

The code on this branch confirms the defect:

- **Where it is.** The defect is in `Engine::drain`
  (`runtime_lifecycle.rs` 57–95), not in the two queries. The master
  plan's "which is in the same queries" is corrected here. The queries
  correctly report the rows as pending. The defect is that `drain` waits
  for them.
- **What dense-disabled means.** The open-time equivalence probe sets
  `dense_disabled` (`open.rs` 874; it never changes for the session).
  The scheduler is then built with no embedder (`open.rs` 927–928), so
  `next_pending_projection_jobs` returns no jobs (`!dense_arm_live`). The
  engine still holds the runtime (`runtime_embedder` is `Some`).
- **Why drain waits.** `drain` takes the typed early return only when
  `runtime_embedder.is_none()` (63). A refused runtime goes to
  `wait_for_idle`. There the probe returns `true` on every wake, so
  `drain` returns `Err(EngineError::Scheduler)` after the full
  `timeout_ms`.
- **Measured** (scratch probe, debug build, 2026-10-31). On a refused
  store with one pending body edge, `drain(3_000)` returned
  `Err(Scheduler)` after 3.000 s.
- **The same wait reaches internal callers.** `transition(...)`
  (`record_lifecycle.rs` 214), `configure_projections`
  (`projection_registry.rs` 1543) and lifecycle actuation
  (`actuation.rs` 1988) settle through `drain_for_non_embedding_mutation`
  (`runtime_lifecycle.rs` 102). Erasure settles through
  `freeze_projection_for_erasure` (`erasure.rs` 551), and the operator
  rebuild through `run_rebuild` (`projection_rebuild.rs` 70). Each calls
  `drain(30_000)`. On the same store, `transition("n1", Deleted)` waited
  30.000 s and returned `Err(Scheduler)`: the mutation failed. For an
  absent runtime the same callers map `EmbedderRequired` to "proceed".
  For a refused one they cannot proceed.
- **Readiness already handles the state.** `read_embedding_readiness`
  reports `Deferred` with the true pending count
  (`projection_generation.rs` 1849–1861). `derive_dense_readiness`
  reports `Unavailable` (1755). Status reports `Refused` or
  `VectorEquivalenceDisabled`. None of these change.
- **The shipped contract.** `dev/interfaces/rust.md` (lines 913–916 and
  1133–1136) says that for a refused runtime "`drain` retains ordinary
  bounded behavior" and "a bounded `drain` may return
  `EngineError::Scheduler`". `slice30_embedding_readiness.rs` line 207
  asserts `drain(0)` is an error that is not `EmbedderRequired`.

## 2. Requirements

### Unit S

- **REQ50-S1. Bounded page cost.** One call of either scan reads at most
  one page (256 rows) from each arm's index, plus the rows its predicates
  reject, whatever the size of the pending backlog.
- **REQ50-S2. Identical pages.** For every store state and every
  `(after_cursor, through_cursor, effective_at)`, each scan returns the
  same rows in the same order as the shipped SQL. Valid stores have
  unique `write_cursor` values across nodes and edges (design § 1.4).
- **REQ50-S3. Everything around the SQL unchanged.** The arm predicates,
  the joins, the generation `CROSS JOIN`, the 256-row page loop,
  in-flight skipping, `dense_member_kind_at`, the below-watermark check
  and the cursor rewind stay as they are. No schema change, no new
  index, no env gate.
- **REQ50-S4. Identical results.** A drained store has the same
  projected state as one drained by the shipped scan:
  `_fathomdb_vector_rows`, `vector_default` without `created_at`,
  `_fathomdb_projection_terminal` and the projection cursor. Readiness
  states and pending counts are unchanged. Search reads none of this
  code, so its results are unchanged.
- **REQ50-S5. Linear drain.** The held drain's cost per vector is flat
  from 10k to 100k vectors on this format (rule in test-plan § 4).
- **REQ50-S6. Plan requirement recorded.** A code comment at the SQL
  states why each arm carries its own `ORDER BY write_cursor LIMIT`. A
  plan test fails if an arm loses its bounded subquery, starts sorting
  on its own, or stops reading its `write_cursor` index.

### Unit D

- **REQ50-D1. Dense-disabled drain is prompt.** A session is
  dense-disabled when the engine holds a runtime that the equivalence
  guard refused (`runtime_embedder.is_some() && dense_disabled`). On
  such a session, with pending vector rows, `drain(timeout_ms)` returns
  once no worker is active. It does not wait for `timeout_ms`. The
  return value is owner ruling D50-1 (design § 2.3).
- **REQ50-D2. Other states unchanged.**
  - Absent runtime with pending rows: immediate `EmbedderRequired`.
  - Absent or refused runtime with no pending rows: `Ok(())` once
    workers are idle.
  - Usable runtime: the barrier, including the pinned invariant
    "`drain` `Ok` ⟹ `dense_readiness == Ready`".
  - A usable runtime that times out still returns `Err(Scheduler)` after
    `timeout_ms`.
- **REQ50-D3. Internal settles (conditional on ruling D50-2).** If the
  owner approves, the non-embedding mutations (lifecycle `transition`,
  `configure_projections`, lifecycle actuation) and erasure treat a
  refused runtime as they treat an absent one: settle the workers, then
  proceed. The operator rebuild still refuses (design § 2.4).
- **REQ50-D4. Readiness and status unchanged.** No change to
  `read_embedding_readiness`, `read_projection_status`,
  `read_projections`, generation status or the pending counts.

### Both units

- **REQ50-R1. Records.** In the same change: the interface doc's two
  refused-drain passages (D50-1), the CHANGELOG, this directory's
  status, and the release state.
- **REQ50-R2. Measurement tool in the repository.** The scaling run's
  probe, driver and analysis are committed and re-runnable (owner rule,
  2026-10-29).

## 3. Non-goals

- 64-row vec0 chunks and store format 35 (Slice 60). Vector eligibility
  in vec0 (Slice 70). This slice touches no table those slices change.
- Hoisting per-row work per batch: `actual_vector_attr_columns` and the
  commit-side `dense_member_kind_at`.
- Serializing writer and worker commits (the remaining gate-shape cost).
- The AC-013 latency gate itself. It is recorded, not claimed.
- `fathomdb-mcp`, which is not on this branch. Its close path gains from
  REQ50-D1 without a change.
- A new public error variant or binding change, unless D50-1 rules
  option C.
- `drain` with a usable runtime and a frozen dispatcher (`run_rebuild`
  freezes before it drains). This is pre-existing and not todo 289.

## 4. Acceptance criteria

| AC | Checks | How |
| --- | --- | --- |
| AC50-01 | REQ50-S6 | `slice50_scan_plans_bound_each_arm` fails at its RED sha (no arm is a bounded subquery `CO-ROUTINE`) and passes after GREEN |
| AC50-02 | REQ50-S1 | `slice50_scan_page_cost_is_independent_of_backlog`: VM steps of one page at 16,000 pending nodes and 4,000 edges, divided by the same at 2,000 and 500, is ≤ 1.5 for both scans. Fails at RED (expected ≈ 7–8) |
| AC50-03 | REQ50-S2 | Property test `slice50_bounded_scans_match_shipped_scan` (proptest, 64 cases by default): random interleavings of node and edge rows, terminals and eligibility states; every page position, with and without `through_cursor`, equals the shipped SQL kept verbatim in the test as the reference |
| AC50-04 | REQ50-S2 | Uncommitted mutations each make AC50-03 fail: the arm `LIMIT` set to 1; an arm ordered `DESC` |
| AC50-05 | REQ50-S2, S4 | Node and edge ordering cases through the engine: (a) 600 edges then 600 nodes, (b) 600 nodes then 600 edges, (c) interleaved with failed terminals, superseded and `t_invalid` rows. Pages equal the reference; after drain the vector rows equal the expected eligible set |
| AC50-06 | REQ50-S3, S4, D2, D4 | Drain, readiness and projection suites pass unchanged (test-plan § 3.4) |
| AC50-07 | REQ50-S4 | The scaling probe's store digest at the slice head equals the base's at every N and repeat |
| AC50-08 | REQ50-S5 | The scaling rule in test-plan § 4 holds |
| AC50-09 | REQ50-D1 | `slice50_refused_runtime_drain_does_not_wait_for_timeout`: `drain(5_000)` on a refused store with pending rows returns within 1 s with the D50-1 result. Fails at RED after about 5 s |
| AC50-10 | REQ50-D2 | The existing `slice30_embedding_readiness` tests pass. A refused store with no pending rows drains `Ok` (guard, design § 2.3) |
| AC50-11 | REQ50-D3 (if D50-2 approves) | One RED per caller family: `transition`, `configure_projections`, lifecycle actuation and erasure on a refused store with pending rows succeed within 1 s. Each fails at RED after 30 s with `Err(Scheduler)` |
| AC50-12 | all | On the final tree: `cargo clippy --workspace --all-targets -- -D warnings -A missing-docs`, `cargo check --workspace --all-targets`, `./scripts/agent-verify.sh` |
| AC50-13 | REQ50-R1 | `./scripts/agent-lint-md.sh` and `./scripts/check-release-state-views.sh` pass |
| AC50-14 | REQ50-R2 | The scaling tool runs from its committed files at the base and the head (test-plan § 4) |
