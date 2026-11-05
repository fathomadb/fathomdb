---
title: FathomDB 0.8.28 Slice 84 — cleanup contention and certification cost
status: COMPLETE
target_release: 0.8.28
---

# Slice 84 — cleanup contention and certification cost

HITL authorized execution on 2026-11-03 after Slice 83 and before Slice 90.
Use the existing development checkout with one writer and read-only Astra reviewers.
[High-level design draft](design.md) owns candidate mechanisms and proof boundaries.

The physical-erasure guarantee/API decision remains at the **end of Slice 83**.
Slice 84 neither postpones that checkpoint nor authorizes a cleanup API, numerical
SLA or different erase/purge success. Bring Slice 84 results back to HITL after
execution. Any guarantee/API implementation requires its own explicit approval.

## Reconcile before starting

Draft source inspected at `b721e8647`; Slice 83 is still IN_PROGRESS with its
final workspace gate pending. Freeze the actual completed Slice 83 code and
reviewed contract as the performance baseline when executing; do not use this
snapshot as a final gate receipt. Verify clean checkout/ownership and dependency
closure before coding. Another active writer requires an isolated worktree.

| Change since CH1 planning | Implication |
| --- | --- |
| [Slice 82](../slice-82/status.md) cut node native calls from 271.090s to 1.229s through set-oriented deletion and added durable physical sanitation. | Preserve atomic canonical/dependency deletion. No further deletion algorithm is justified. |
| [Slice 83](../slice-83/status.md) added scoped writes, autonomous idle/reopen service, finite admission, worker joins, denial and finalization safety. | Retain these mechanisms and their original regression tests as mandatory constraints. |
| Three-pair loaded medians: node 1.354→8.070s, edge 1.394→3.393s, property 3.108→16.176s; fewer completed writes. Final corrected node smoke: 1.384→7.237s. | Treat this as a measured service regression. These compare Phase 82 with distinct Phase 83 sources, not a frozen Slice 84 baseline. |
| Physical writer waits were 6.638/1.952/13.355s; native holds 0.740/0.925/1.353s. Counters overlap. | Measure admission/proof costs directly before assigning causality; do not add overlapping stages. |
| Prepared mapped-vector queries reduced deterministic work without a measured latency benefit. | Preparation-count improvements alone cannot pass performance acceptance. |
| Property 100-Hz no-erasure controls missed offers; both 20-Hz controls passed. | Use 20-Hz property writes and 100-Hz node/edge writes, with matched controls and 100-Hz reads. |
| Property's old Phase 82 comparison timed out as a whole process without a native result. | Preserve that limitation. No inferred speedup or native lower bound, and no rerun needed merely to fill the old cell. |
| Shared CH1 Phase 2 acceptance covers safe progress and truthful measurement, not improved total throughput. | Complete Slice 83 honestly; give this optimization separate acceptance and report residual Phase 82 overhead. |

Review assigned CH1-2A–2F and all Slice 83 fixes at execution, including trigger
fallback refusal, old/new endpoints, same-batch supersession, persisted denial
completeness, mapped non-head vectors, priority/lost wake and durable blockers.
Expiry, current/temporal isolation, larger envelopes and staged replacement are
not allocated here. Existing design/ADR successors remain authoritative.

## Requirements and acceptance

These S84 labels are local planning labels, not new canonical product ACs.

| ID | Requirement | Acceptance oracle |
| --- | --- | --- |
| S84-1 | Attribute costs before changing proof authority. | Instrument call counts and elapsed time for envelope components, root/denial decoding, exact cursor/vector/dependency proofs, per-write certification, journal updates and lock/priority/service-turn wait/hold. Separate public native call, cleanup lifecycle, reader gate and checkpoint. Reproduce the regression on the frozen baseline without competing workloads. |
| S84-2 | Preserve equivalent safety checks and supported mutation scope. | Optimized/reference paths make identical admit/refuse, rollback and completion decisions on real SQLite fixtures. Same-source, affected old/new identities/endpoints, provenance/admin/publication mutations and persistent/TEMP trigger fallback stay refused. Tests retain exact survivors, registered-copy proof and current errors/precedence. |
| S84-3 | Reuse authority only under a proved validity boundary. | Deterministic work-count RED shows repeated work targeted by the design; GREEN reduces it. Any reusable proof/accounting invalidates on the relevant mutation, rollback, journal phase/generation change and reopen; corrupt/missing/legacy authority takes the exact full validator or refuses. No process-local fact authorizes durable completion. |
| S84-4 | Reduce contention without starvation or leaked ownership. | Rendezvous verifies actual foreground progress and maintenance progress under supported writes, idle/reopen service, priority/wake behavior, blocked checkpoint/sink retry and close/join. No transaction/mutex retained through backoff and no unbounded per-job workers. |
| S84-5 | Preserve physical completion and recovery. | Existing positive FTS/main/WAL/sink witnesses, crash/reopen, non-head vector residue, generation mismatch, redacted tombstone/global fence and immutable target-scope tests pass. Finalization retains its exact authoritative validation and actual TRUNCATE requirement. |
| S84-6 | Demonstrate integrated benefit. | For each node/edge/property workload, three valid pairs versus final Slice 83 show ≥25% reduction in median public erase duration and strictly more median completed foreground writes. Duration improves and completed writes do not fall in every pair. All erasures finish physical proof; no additional Engine errors/refusals/unfinished operations or fewer completed reads. Matching no-erasure controls serve all offers. |
| S84-7 | Keep scope and evidence honest. | Report remaining differences from Phase 82, idle/control costs, maximum units/reader gates, all refused/missed/incomplete attempts and resource costs. No SLA, general availability or property speedup from an invalid baseline. Failed performance gates mean optimization not qualified, even if correctness passes. |

The 25% threshold is a comparative screening target for this regression, not a
product latency promise. Slice 84 is not automatically a full regression repair
merely because it improves Slice 83: report the remaining Phase 82 gap. Any target
change is a reviewed plan revision before new measurement, preserving old outcomes.

## Work order and TDD

1. Reconcile final Slice 83 code/status/gate and the end-of-83 HITL decision.
   Inventory actual callers in `physical_erasure/admission.rs`,
   `physical_erasure.rs`, `physical_maintenance.rs`, `write.rs`, `write_commit.rs`,
   `dependency_closure.rs` and existing `physical_metrics.rs`. Freeze baseline
   binary/input/runtime identities and commands; confirm a checkout-owned venv.
2. Add the minimum stage instrumentation using the existing metrics/driver.
   Measure the same instrumentation in baseline and candidate. Distinguish
   nested counters and observer overhead. No production telemetry format change.
3. Write a concrete design amendment selecting at most one certification change
   and one scheduling change justified by the profile. Map every omitted query
   to equivalent authority, invalidation and fallback. Astra reviews the exact
   design and ADR/interface impact before code. A new durable format or weaker
   accepted check requires explicit design/contract review, not a quiet shortcut.
4. Implement each behavior through its own observed, committed RED then GREEN.
   RED uses deterministic certification work or safety/progress failure, not noisy
   timing assertions. Existing passing guards remain guards. Never edit an oracle
   to accept weaker isolation or let a cache hide corrupt debt.
5. Astra code review, then independent verification of decisive RED/GREEN,
   full-validator comparisons, rollback/process crash/reopen, trigger/endpoint
   guards, physical residue, late telemetry, pinned readers and worker close.
6. Freeze the reviewed final candidate and run the bounded paired protocol below.
   If only preparation/work counts improve, record an unsuccessful optimization;
   do not label service improved. No endless tuning or automatic replacement runs.
7. Run one final `./scripts/agent-verify.sh` for the final code phase from a clean
   tree. Use scoped checks while working and failed/affected suites after narrow
   repairs. Exercise Windows WAL suites if checkpoint/reader behavior changes.
   Keep initial full-run and targeted-rerun claims distinct. CI uses `gh` only.
8. Write `status.md` with each S84 disposition, final source, review/verification,
   measured benefit/limits and next action. Land into the development release line
   and update state/views if execution has been allocated. Remove any temporary
   worktree/branch after clean landing; retain raw benchmark evidence. Bring the
   results back to HITL. No tag/publication is part of this slice.

## Fixed measurement protocol

- Reuse the Slice 83 driver and actual Engine workload, original 10k-owner,
  five-revision, 10%-target masters, runtime performance mode, release builds,
  SQLite/tokenizer/provider identities and eight readers/one writer cycling eight
  unrelated identities. Hash inputs before/after. Inventory populated families,
  actual managed triggers and candidate creation paths. Do not rebuild a new grid.
- Freeze actual commands and current limits before timing. Measure baseline and
  candidate with identical observer code. Keep existing primary-connection/worker
  ownership; no SQL adapter or disabled fence may establish a product pass.
- Core: three paired repetitions per family, four cells each: Slice-83 erase,
  candidate erase and each one's no-erasure control. That is 36 cells. Alternate
  pair order. Reads offer 100/s; writes offer 100/s node/edge and 20/s property;
  all windows last 15s. Start erasure at the same declared offset, retain the
  traffic window if erasure finishes earlier and continue physical observation
  after the window until completion or the 90s whole-cell limit.
- After the core, one paired Phase-82/candidate erase comparison per family at
  the same sustainable load (six cells), as residual-cost context only. Freeze
  its exact source; it lacks newer resumability checks, so cannot serve as safety
  oracle or authorize removing them. Core controls remain the service controls.
- Up to six diagnostic cells: profile one baseline per family before design
  selection, then one matching candidate per family after reviewed implementation.
  Carry the same six-cell allowance across both stages; these diagnostics are
  separate from final core results.
  Three idle/no-traffic baseline/candidate pairs (six cells) measure policy and
  observer costs. Maximum 54 cells; no broad knob sweep or extra cleanup arm.
- One 45-minute active measurement budget covers all diagnostics, fixture/setup,
  profile work, final core/reference/idle measurements, validation and shutdown;
  pause only while implementing/reviewing code, record each active segment and
  carry total allowance forward. No reset per binary/restart. Sample 8-GiB RSS
  and 12-GiB total owned scratch ceilings every 100ms, retaining overshoot.
  Use original masters when still valid. If remaining time/resources cannot
  complete required cells, stop and report incomplete; budget revision needs
  explicit approval before further execution. Do not delete shared caches/masters.
- Retain every attempt with source and reason. A failed control disqualifies its
  paired service claim; no silent load lowering or replacement. Fixing a monitor
  cannot promote earlier attempts. Later authorized repetitions get distinct IDs.
- Reconcile every scheduled/issued/completed/Engine-refused/generator-missed/
  unfinished/unknown offer. A killed recorder's absent dispositions are unknown,
  not measured refusals. Qualified p99 requires ≥1,000 successful samples and
  complete, refusal/error/miss/unfinished-free accounting; 300 property writes
  cannot establish write p99. Never pool protocols or repetitions to cross that
  sample threshold. Report completion counts and median latency where qualified.
- Report native writer hold, foreground certification CPU/hold, maintenance wait/
  hold, service-turn waits, sanitation SQL, reader admission pause, checkpoint,
  public-call latency and full lifecycle separately. Inclusive stages overlap;
  do not sum them as total. Retain raw event locators and compact results in the
  existing experiment package, without tracking large generated events/DB files.

## Scope control and quality

Atomic deletion, singleton debt, current resource limits, certified write class,
trigger refusal, exact closure/denial authority, reader policy, original telemetry
sinks, final global fence and physical-success semantics are fixed constraints.
No new public API/configuration, deadline, reader cancellation, always-on general
cache, cross-version migration or temporal rewrite. Unsupported mutation paths
remain refused. An optimization requiring broader scope returns for approval.

Apply the [quality roadmap](../../../../notes/fathomdb-quality-testing-roadmap.md):
real-database differential oracles, deterministic operation counts, boundary and
property/state-machine sequences, selected fault injection and resource cleanup.
Defer exhaustive VFS/OOM/MC/DC and new test frameworks. Planning validation is
`./scripts/agent-lint-md.sh`; this documentation task runs no production regressions.

## Review and handoff

Astra reviewed the plan and high-level design on 2026-11-03: APPROVE with one
protocol clarification, applied. Baseline diagnostic profiling precedes design
selection; matching candidate profiling follows reviewed implementation, within
the same six-cell/45-minute active allowance. No other blocking findings.

This draft changes neither Slice 83's
completion status nor the end-of-83 HITL guarantee/API decision. Execution begins
in a separate session after Slice 83 completion, final-baseline reconciliation,
and the specified concrete design review.

## Execution reconciliation — 2026-11-03

The completed Slice 83 baseline is `af84f12f8` production code, closed by
`dcf48f28e`; the drafted snapshot was `b721e8647`. Since that draft:

1. `8150e0f8a` added diagnostic and idle-profile deadlock regressions.
2. `af84f12f8` fixed reader-gate/metadata lock ordering, actual SQLite work
   accounting, bounded checkpoint observations after drain refusal, idle probe
   subscriber suppression and the pre-primary-lock rendezvous.
3. `dcf48f28e` reconciled configuration worker counts, release state and all
   205 collected gate suites with scoped repairs. It closed engineering but
   retained the measured regression and open guarantee/API decision.

All CH1-2A–2F work is implemented and remains a safety constraint. No expiry,
staged replacement, historical graph route, cleanup API or expanded write class
is allocated. Existing root/denial checks, same-batch and old/new endpoint guards,
managed-trigger refusal, exact vector/dependency proofs and tombstone/TRUNCATE
remain intact. The current execution request authorizes optimization under that
accepted contract; it does not decide the broader guarantee/API question.

The first candidate to evaluate is an exact query algebra rewrite of the
structural-zero boundary: maximum of per-table maxima instead of maximum over
all cursor rows. It retains the full validator and has no reusable authority or
invalidation state. Three input tables have indexed cursors; operational mutations
still require their existing scan. No added index or scheduler change is assumed.
Final selection awaits the diagnostic profile and concrete Astra review.

Apply S84-3 by deterministic VM-step reduction and differential SQLite results
for this exact query; cache invalidation requirements are conditional on reuse,
which this design does not introduce. Retain all recovery/rollback guards. S84-6
requires candidate completed reads at least the baseline count in each pair and
all controls serving every offer. A refused initial erase followed by successful
retry is physical recovery, not a faster successful public erase.

## Execution outcome

The fixed 54-cell campaign completed with all three S84-6 gates failed. Astra
recommended rejecting the new scheduling policy; original scheduling and the
accepted ADR are restored. Profiling, driver/controller and complete negative
evidence remain. See [status](status.md) and the design's reviewed web research.
Verification closed by scoped resumption (see status); no replacement experiment, threshold
relaxation, guarantee/API choice or broader implementation is authorized here.

## Final closeout reconciliation — 2026-11-03

The completion request begins at `04b08895e`, after the fixed campaign,
candidate rejection `5214ccb01`, observer ownership repair `10a5a2a34` and
84b draft. These supersede the original execution assumptions:

1. The experiment and implementation are finished; every S84-6 family failed.
   Approve the negative disposition and retained instrumentation/tooling. Reject
   the production scheduler candidate; preserve the original physical contract.
2. S84-1 supplies elapsed/count attribution; certification CPU remains unmeasured.
   S84-3 reusable authority is inapplicable because no proof reuse was retained.
   Successful investigation closure does not mean the optimization qualified.
3. The full test stage was interrupted. Finish its outstanding collection and
   repair the stale Windows checkpoint-owner harness selector using the observed
   failing test, then rerun that suite including its ordering mutation. Preserve
   the independent completion-owner assertion and production checkpoint code.
4. Astra independently reviewed the existing design and retained diff: approved
   for negative closure, with that narrow harness repair. A separate Astra agent
   verifies safety/tooling and the outstanding collection. Use existing passed
   lint/typecheck/security evidence and scoped checks for the selector repair.
5. Record final outcomes and reconcile state/views. The 84b draft remains a
   separate follow-up; no new experiment, threshold relaxation or broader
   guarantee/API decision follows from closing this investigation.
