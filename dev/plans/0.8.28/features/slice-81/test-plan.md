---
title: FathomDB 0.8.28 Slice 81 — test plan
status: COMPLETE
target_release: 0.8.28
---

# Slice 81 test plan

Traceability: [requirements](requirements.md). Hypotheses and sampling controls:
[experiment design](experiment-design.md). Native authority and reused tools:
[references](README.md#references). The table preserves original planned witness names; executed coverage and explicit
unqualified/deferred obligations are mapped in [results](../../../../experiments/erasure-availability/RESULTS.md).
All executed database tests use real files/connections.

## Baseline and behavioral matrix

| Planned test/witness | Requirements/hypothesis | Required assertion |
| --- | --- | --- |
| `shipping_ghost_positive_control` | P81-R01/05, H81-1 | Existing ordinary Engine witness still fails physical removal after positive prepresence; preserve known failure separately from passing candidate tests. |
| `native_pending_closure_blocks_unrelated_writer` | P81-R01/07, H81-5 | Current real global gate refuses unrelated writes; do not hide baseline with a wrapper. |
| `family_inventory_has_positive_witnesses` | P81-R01/05 | Each claimed family/mode populated and marker visible before deletion; populated v2, empty families and unsupported modes explicitly labeled. |
| `acceptance_and_progress_commit_together` | P81-R02/06 | Failure before commit produces no accepted job; committed intent and work mutation/progress survive reopen atomically. |
| `denial_linearization_and_pinned_reader` | P81-R03, H81-5 | New requests respect the committed fence; pre-admission/pinned behavior follows declared contract; no retrospective result revocation claim. |
| `denied_rows_do_not_affect_survivor_statistics` | P81-R04, H81-5 | At equal committed scope compare exact IDs/order/score bits with independently constructed survivor-only corpus; include nonmatching target terms and changed lengths. |
| `denial_precedes_caps_seeds_and_evidence` | P81-R03/04 | Duplicate/high-fanout targets cannot consume candidate/seed caps; relaxed/frozen/history/evidence either honor scope or explicitly refuse/mark unqualified. Test public routes before claiming integration. |
| `fresh_and_legacy_raw_erasure` | P81-R05, H81-1 | MATCH/content/live-shadow/main-file and applicable freelist/WAL/sink checks; unique marker absent only after complete, survivors and exact source scope preserved. |
| `bounded_units_release_writer` | P81-R07, H81-2 | Rendezvous after commit/before next unit permits real unrelated write and reader progress; a held transaction or disabled native gate is not a passing witness. |
| `merge_converges_without_traffic_and_with_updates` | P81-R06/08, H81-3 | No future user writes needed; bounded concurrent unrelated updates do not restart sanitation forever or resurrect target generations. |
| `reservation_saturation_and_original_deadline` | P81-R02/08, H81-4 | Reject an unguaranteed acceptance; retries/reopen never reset deadline; overdue remains durable breach plus owed work. |
| `source_reingestion_and_projection_race` | P81-R06/07 | Reject same-source reingestion while pending; stale queued/in-flight projection and rebuild cannot republish target content. |
| `reader_lease_and_unrelated_checkpoint_pin` | P81-R03/08, H81-4 | Test old and continually arriving unrelated readers; failed finalization stays incomplete; declared lease/admission policy permits eventual completion. |
| `telemetry_unavailable_and_retry` | P81-R05/08 | Wrong/missing original sink cannot clear obligation; reattach/retry is idempotent and proves sink/WAL completion. |
| `expiry_preserves_authorized_history` | P81-R09, H81-6 | 10k current expiry changes membership/certificate correctly without interpreting history retention as erase authority. |
| `runtime_profile_fresh_cached_children` | E81-R01 | Explicit production profile latched before any connection in every child; no silent cached-profile difference. |
| `controller_outcomes_quantiles_and_budgets` | E81-R02/04/05 | Nearest-rank indices, request IDs, offered/refused/unfinished accounting, no replacement retries; stop/campaign exhaustion retains null unqualified percentiles and pending cells. |

## Faults, state properties and recovery

Use engine-owned bounded rendezvous/fault hooks and a child-process crash runner.
Exercise before/after acceptance commit; target mutation before progress commit;
after committed progress; before/after publication; during old-generation
retirement; sink completion; WAL completion before final bookkeeping; close/reopen.
A crash is process-crash recovery, not a torn-sector/power-loss proof. Kill tests
must join children and release hooks; no unbounded sleeps.

Generate bounded state-machine sequences with fixed replay seeds: accept, clean
one unit, mutate unrelated source, pin/release reader, inject failure, crash,
reopen, advance time and retry. Assert immutable scope/deadline, idempotent progress,
no premature completion, no resurrection, and survivor preservation after each
transition. The generator exercises real SQLite state; human-authored invariants
and raw positive controls are the oracle, not implementation-produced snapshots.

Stress single-unit size separately with a 1-MiB body, common-term posting lists,
large dependency fanout, duplicates and exhausted temporary disk. No claim that
one row/page implies bounded wall time or that allocator OOM must be recoverable.
Test no leaked handles/transactions/private copies or orphan workers after every
error/close path. Conditional C adds mutation-capture, atomic publication, pinned
old readers, crash-abandoned generation and erasure of both old/new copies.

## RED/GREEN and verification order

Each new behavior group has its own staged/committed observed RED, compile success
and retained failure log, followed by GREEN. Tests are read-only during fix-to-spec.
Baseline invariants already passing are guards, not fabricated RED evidence.
The original shipping erasure diagnostic remains visibly FAIL until production is
actually repaired; candidate defaults must not disguise it.

During implementation run scoped fmt/clippy, typecheck and focused behavior tests
in latency order. The unpublished experiment crate's exact commands are added to
its committed README when it exists; do not cite nonexistent commands as runnable
now. Use the unchanged existing erasure-probe command in S81-5 for baseline proof.
Run the required final code gate once for the final code phase; repair findings
with scoped reruns. No production regressions for this documentation-only phase.

An independent verification subagent reproduces at least the physical positive
control, selected candidate RED/GREEN, rollback/reopen, actual writer progress,
blocked-WAL retry and controller tests. It reconciles all manifest cells/attempts,
recalculates quantiles, validates observed family/copy counts and checks no running
children. Astra reviews code before timing and conclusions afterward. Record
actual commands/source SHAs/logs and distinguish mechanism, simulated composition,
actual Engine paths and product acceptance. `gh` is used only for CI work.
