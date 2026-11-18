---
title: Slice 5 — Verification adequacy and test choices
status: COMPLETE
slice: 5
target_release: 0.8.30
date: 2026-11-18
---

# Slice 5 — Verification adequacy

Reviewed the `release/0.8.30` source at `18427b7a8`, selected assertion bodies,
feature/long-test gates, and recorded runtime and hosted evidence. This is a
bounded proposal review for [Slice 9](../plan.md#a-prework-effort). It records no
new suite execution or product verdict. [Requirements](../../../requirements.md),
[acceptance](../../../acceptance.md), [historical findings](../testing-robustness-findings.md).

## Need → requirement → acceptance → test

- **Correct ranked results and attribution, R30-COR/S50 (30), REQ-010/011 retrieval.** Accepted
  [stored-record identity](../../../adr/ADR-0.8.29-retrieval-record-identity.md).
  [`slice50_record_identity`](../../../../src/rust/crates/fathomdb-engine/tests/slice50_record_identity.rs)
  checks same-body revisions, independent expected score bits, ranks, provenance and exact evidence
  ownership.
  [`slice71_search_statement_trace`](../../../../src/rust/crates/fathomdb-engine/tests/slice71_search_statement_trace.rs)
  checks exact fixed-state optimized/full-sort result on a route-witnessed real DB. Retain both
  complementary oracles. A comparison can share the same faulty predicate. Treat .29 G-W
  sample860/query360/op9971 as unresolved, with both operations on the reference route. Inspect
  retained candidate authority first, then at most one bounded follow-up capturing candidates, ties,
  ranks and cursors. No general score tolerance.

- **Filter before candidate caps and retain committed snapshot authority, R30-COR/LAT (30/40).**
  [Eligibility ADR](../../../adr/ADR-0.8.25-eligibility-and-frozen-reads.md) →
  [`slice35_eligibility_pretruncation`](../../../../src/rust/crates/fathomdb-engine/tests/slice35_eligibility_pretruncation.rs).
  `body_fts_filters_before_the_ranked_limit` expects an eligible result after 150 excluded rows with
  limit one. Separate SQL/query-plan tests check placement and index shape. Retain the behavioral case.
  After-cap filtering on the fixture returns zero and must fail. Plan text alone cannot qualify a
  treatment, and direct text does not qualify the hybrid vector cap.

- **Feedback refers to the intended occurrence, R30-COR (30).**
  [`feedback`](../../../../src/rust/crates/fathomdb-mcp/tests/feedback.rs) exercises repeated refs from
  distinct cursors, typed refusal, no partial telemetry write and existing precedence, including
  compact output. Retain atomic and precedence cases. Richer occurrence labels depend on a selected
  contract. Add label lifetime/store/epoch and atomic mixed-input assertions only for that selected
  change.

- **Truthful coverage and UTF-8 spans, R30-COR (30).** In-crate
  [`stored_coverage_tests`](../../../../src/rust/crates/fathomdb-engine/src/stored_coverage_tests.rs)
  covers rollback/external changes, failed capture publication, reopen, and malformed-open error
  ordering. [`snippet_search`](../../../../src/rust/crates/fathomdb-engine/tests/snippet_search.rs)
  owns public search-sidecar cases. The in-crate suite already exercises pinned-reader scored extents.
  Add a publication composition only for a specific uncovered case. Assert exact known/unknown counts and
  UTF-8 ranges from authored inputs. A stale staged capture or treating unknown as zero must fail. Keep
  thin native refusal parity where changed.

- **Cursor and concurrent reader progress, REQ-019/055 and R30-ROB/QUAL (50/70).** AC-021 uses
  the [`lifecycle_reliability`](../../../../src/rust/crates/fathomdb-engine/tests/lifecycle_reliability.rs)
  60-second reader/DDL variant. AC-059b in [`cursors`](../../../../src/rust/crates/fathomdb-engine/tests/cursors.rs)
  requires the exact body and actual vector after the returned write cursor is satisfied. The long
  [`projection_cursor_bounds_observed_row_count`](../../../../src/rust/crates/fathomdb-engine/tests/cursor_read_after_write.rs)
  requires 1,000 searches, committed writes during the read phase, nonempty results and visible
  advancing hits within the snapshot cursor. Retain. Without `AGENT_LONG` the cursor check returns
  without exercising its oracle. Require observed execution and positive work. An empty reader stream
  or a dead writer fails the current oracle.

- **Typed refusal, owned capacity and clean close, R30-ROB (50).**
  [`slice90_foreground_dispatch`](../../../../src/rust/crates/fathomdb-engine/tests/slice90_foreground_dispatch.rs),
  [MCP admission](../../../../src/rust/crates/fathomdb-mcp/tests/admission.rs) and in-crate
  [`provider_deadline_starts_after_sqlite_quiescence_and_is_never_renewed`](../../../../src/rust/crates/fathomdb-engine/src/slice90_close_review_tests.rs).
  The last holds a real SQLite transaction and compares first/concurrent/repeated/Drop deadlines and
  retained-worker accounting. Retain the separate MCP permit and engine slot assertions. Probe a
  deterministic two-engine alias schedule with positive sibling work and reopen. A wrong owner, early
  deadline, refunded live slot or stranded waiter must fail. Avoid a total-close wall-clock assertion.

- **Bound native memory, R30-ROB/FORK (50/60).** .29 [Node D/E
  status](../../0.8.29/features/slice-130/status.md) records default FAIL/OFF_NOT_RUN and the separate
  semispace4 result. [`slice130_soak`](../../../../scripts/slice130_soak.py) and installed workers
  separate RSS, Node heap and private used/reserved counters. Retain the existing sampler and numeric
  checks. Select a bounded default Node pair with real work, exact output and ownership/exit evidence.
  Nil OFF private counters need actual allocator evidence. No forced GC or heap tuning to close the
  default claim.

- **Recovery after delivered interruption, R30-ROB (50).**
  [`slice100_interruption`](../../../../src/rust/crates/fathomdb-engine/tests/slice100_interruption.rs)
  and [operator](../../../experiments/0.8.29-slice100-checkpoint/README.md) separate source seams from
  the delivered Q1 event and fresh-process retry. Reuse valid .29 proof. Run a changed live checkpoint
  case only if selected and concretely authorized. Include actual delivery, durable state and survivor
  proof. Healthy child/parser success cannot satisfy the fault claim.

- **Useful latency and fidelity, REQ-010/011/017/018 and R30-LAT/QUAL (40/70).** AC-076/072 use the
  existing AC-012/013 names in
  [`perf_gates`](../../../../src/rust/crates/fathomdb-engine/tests/perf_gates.rs). AC-081 combines the
  absolute batch limits with positive reader progress. AC-073/075 real-corpus selectors are in [Slice
  70](../features/slice-70/plan.md). Preserve numeric criteria and actual long execution. 100k SDK
  latency, AC-083 tracked direct text, exact-neighbor fidelity and installed runtime behavior are
  separate populations. Attribute and compare using Slice 40's matched protocol before selecting a
  treatment.

- **Installed packages execute safely, REQ-032/052 and R30-QUAL/REL (70/80).** AC-036 requires the real
  no-listen syscall cycle with ptrace. AC-056 requires exact target-native installed software. Current
  [Rust registry smoke](../../../../scripts/release/smoke/smoke-crates-sdk.sh) pins the crate and
  opens/closes it. Keep the security/control layers. Extend released consumers through Slice 20 to
  verify a persisted record, missing record, separate-process reopen and clean exit. Slice 70 qualifies
  exact candidates and 80 verifies registry installs.

## Method and cost decisions for Slice 9

1. **Select R30-TEST gaps by plausible wrong behavior.** 20 owns first failure
   surviving later cancellation, published-state closure and consumer
   negatives. 30 owns rank/reference, committed coverage and typed-open oracles.
   50 owns positive progress, cleanup and recovery composition. 70 accounts for
   selected gaps and records actual applicable executions and owner dispositions.
2. **Keep one deep owning suite plus thin installed checks.** The Rust engine
   owns persistence/ranking state; MCP owns real request/epoch semantics. Keep
   target-native loader/ABI and SDK conversion checks where they detect issues
   an engine test cannot. Avoid copying the full path matrix into each binding.
3. **Retain fault strength and track actual scope.** Use real stores, definite
   rendezvous and human-authored assertions. Prefer existing controls with a
   demonstrated failing mutation or defect. Add a genuine RED for each behavior
   change. Treat already-passing characterization as evidence. Defer global
   coverage targets, exhaustive VFS/OOM and a new test framework.
4. **Use cost evidence before consolidating.** The Windows source-check hotspot
   was 856.43 s on one hosted run and 579.34 s on a later one. Keep meaningful
   structural mutations and the real Windows WAL tests while 20 reduces static
   work. There is no measured case here for deleting focused runtime tests.
   [Original CI](https://github.com/fathomadb/fathomdb/actions/runs/44215258383),
   [later fast job](https://github.com/fathomadb/fathomdb/actions/runs/44244789028/job/134601116052).
5. **Keep platform/access claims honest.** The planning worktree lacks its own
   venv, the Windows VM is off, and live GPU/fault execution needs its selected
   venue and authority. Retained proofs are reusable only for matched relevant
   source, features, fixture, artifact and protocol. Use scoped probes during
   implementation and one full developer gate per coherent source phase.
   [Environment inventory](slice-0.md), [quality roadmap](../../../notes/fathomdb-quality-testing-roadmap.md).

## Query scaffold reconciliation

[Slice2](slice-2.md) found the query `property_template.rs` compares a string with
its clone. It is not meaningful correctness coverage. Actual generated query
compiler expectations already exist in the in-crate
[query tests](../../../../src/rust/crates/fathomdb-query/src/lib.rs) and
[stop-word tests](../../../../src/rust/crates/fathomdb-query/src/stop_words_tests.rs).
The [scaffold checker](../../../../scripts/check-property-test-scaffolds.py)
does not currently register that query placeholder. Propose retirement or a
specific missing property in30 rather than duplicating the existing generated
cases or counting the placeholder as coverage. Keep the current gate scope
explicit; no test or checker changed here.
