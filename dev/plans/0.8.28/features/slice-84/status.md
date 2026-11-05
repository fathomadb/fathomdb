---
title: Slice 84 — bounded optimization rejected
status: COMPLETE
date: 2026-11-03
---

# Slice 84 status

The bounded investigation and all 54 measurements are complete. **S84-6 failed
for every family; the optimization is NOT QUALIFIED.** Astra reviewed the
negative result and recommended rejecting the production scheduling change.
Original Slice 83 scheduling and ADR semantics are restored. Profiling and the
bounded measurement tooling are retained. Verification closed through scoped resumption (see final closeout below);
no performance repair, full-green result, numerical guarantee or cleanup API is claimed.

## Scope, design and implementation

The [plan](plan.md#execution-reconciliation--2026-11-03) enumerates changes since
its draft: final Slice 83 diagnostic/reader fixes and verified closeout. All
CH1-2A–2F guards remain constraints. The
[design](design.md#rejected-scheduling-candidate--synchronous-sanitation-priority)
records the measured candidate selection, exact lock/authority boundaries and
rejection. No proof cache, index, durable format or mutation class was added.

The final Slice 83 production baseline is `af84f12f8`, closed at `dcf48f28e`.
Instrumented baseline `79adbe451` and candidate `af683705a` use identical
metrics/driver code, fixed provider, performance runtime and original
10k-owner/five-revision/10%-target masters. Candidate production is `0b21f1909`;
`82d063459` only synchronizes a test against autonomous retry. Frozen binaries,
inputs and commands are recorded in the measurement manifest. No new worktree
or branch was created; work uses the existing `dev/0.8.28` checkout.

TDD and review evidence:

- Profiling RED `53a5bc4ec` failed on missing envelope-stage observations;
  GREEN `c185440d9` records actual nested work and restores scope.
- Driver RED `732e97653` exposed idle traffic; GREEN `79adbe451` passes idle,
  strict-mode and unchanged-rate schedule tests.
- Controller RED `e97631818` failed without the implementation; GREEN
  `9db421d92` passes eight protocol/accounting tests after Astra review.
- Rejected scheduling RED `4058e8f31` exposed newcomer bypass of the proposed
  priority policy; GREEN `0b21f1909` passed the actual synchronous rendezvous
  and error-release tests. Astra approved the mechanism and code, then recommended
  removing it after performance failure. The new test expressed a proposed
  policy, not an existing contract defect. Candidate-only tests and its normative
  ADR sentence were removed with the rejected implementation; original safety
  oracles are unchanged.

## Acceptance disposition

| Requirement | Result |
| --- | --- |
| S84-1 attribution | PASS for elapsed/count attribution. Envelope, eligibility, root decoding, exact cursor/vector proof, journal, foreground wait/hold and existing cleanup stages are observed. Counters overlap; certification CPU was not measured. |
| S84-2 equivalent safety | Candidate retained every SQL authority check and passed independent unchanged safety suites. Final production restores the original scheduler. |
| S84-3 reusable authority | Not applicable: neither candidate reused proofs nor added accounting authority. The small exact-MAX idea was rejected before implementation because measured boundary cost was immaterial. |
| S84-4 contention/progress | Proposed priority rendezvous passed, but integrated contention/service benefit failed. The new scheduling policy is rejected, not reclassified as a necessary correctness fix. |
| S84-5 physical completion | All 54 cells met their physical/survivor/control witnesses; no erase call returned an error or needed recovery retry. Existing crash/reopen, mapped residue, rollback, scoped-write and reader/worker guards passed independently. |
| S84-6 integrated benefit | FAILED for all three families. Thresholds and failed controls are preserved; no replacement attempts or relaxed qualification. |
| S84-7 honest scope/evidence | PASS: all attempts, resource/accounting results, residual Phase 82 costs and negative verdict retained. Broader guarantee/API remains open. |

## Fixed paired results

Three pairs per family, each with both no-erasure controls; 15-second traffic
windows, eight readers at 100 Hz and one writer at 100 Hz for node/edge or
20 Hz for property. These medians are descriptive, **not qualified improvements**.

| Family | Final83 public erase median (s) | Candidate median (s) | Final83 completed writes median | Candidate completed writes median |
| --- | ---: | ---: | ---: | ---: |
| Node | 7.783 | 7.705 | 757 / 1500 | 764 / 1500 |
| Edge | 3.291 | 3.404 | 1192 / 1500 | 1182 / 1500 |
| Property | 15.412 | 13.957 | 26 / 300 | 44 / 300 |

Node improved only about 1%, lost reads in two pairs, and its third candidate
control missed one write (1499/1500). Every edge pair was slower with fewer
completed writes. Property improved median duration about 9.4%, but two pairs
were slower and two lost reads. All other controls served every offer. One core
baseline node write was Engine-refused; candidate core operations introduced no
additional Engine refusals. Partial streams and 300-write property controls do
not qualify p99; protocols/repetitions are not pooled to manufacture it.

Profiling explains the unsuccessful choice. Baseline node's 8.612-second
call included 3.020 seconds of eligibility and 4.038 seconds of certification
(envelope 1.519, registered-cursor proof 2.727; nested stages overlap).
Boundary MAX cost only 0.077 seconds. Candidate priority reduced primary-mutex
wait but sanitation wait still spent 6.033 seconds acquiring the existing writer
turn, versus 6.948 seconds baseline. It moved waiting without removing an
already-admitted write's expensive certification. The full scans remain intact.

Single-pair Phase 82 residual context was node 1.308→7.051 seconds,
edge 1.351→3.402, property 2.674→10.735. These are context-only comparisons,
not safety baselines or three-pair qualifications. Idle baseline/candidate public
calls were 1.436/1.470, 1.461/1.459 and 2.963/2.979 seconds respectively.
Observers are identical; these do not isolate observer-disabled overhead.
Maximum candidate core sanitation units were 0.010582/0.007447/0.023703 seconds;
maximum reader-gate windows were 0.463621/0.185908/0.885970 seconds. No value is
an SLA or independently additive stage cost.

All 54 cells finished within the 90-second per-cell limit. Charged active time
was 1194.034 seconds, including a conservative 180-second profile-analysis/setup
charge, below the shared 2700-second allowance. Peak sampled RSS was
1,016,299,520 bytes; owned scratch 1,087,680,191 bytes. Across distinct protocols,
124,800 offers reconcile to 112,209 issued plus 12,591 generator misses; issued
operations split into 112,186 completions and 23 Engine refusals. Twenty-two
refusals belong to Phase 82 context and one to core baseline. Failed, unfinished,
unknown and accounting-error counts are all zero. These totals are accounting,
not a pooled performance sample.

[Compact measurements](../../../../experiments/erasure-availability/slice84-measurements.json)
retain manifests, per-cell stages and dispositions. Raw events remain under
`/tmp/fathomdb-slice84/campaign/`; frozen binaries and source identities are in
`/tmp/fathomdb-slice84/`. No large raw events or database replicas are tracked.

## Verification and follow-up before final closeout

Astra independently ran maintenance 7/7, availability 16 passing tests (the
ignored crash child is exercised by its parent), and physical-erasure 6/6,
including randomized retries and rollback, on the reviewed candidate. Scoped engine Clippy and Markdown
passed. The final retained profiling test passes after candidate rejection.
Astra independently reconciled all raw offers, native physical success, frozen
input/binary/controller/helper hashes, resource limits, every failed service gate
and context/idle results. The final verifier started at `7b7cf2472` and stopped
in lint because the new observer test module lacked an ownership entry.
`10a5a2a34` added that entry; resumed lint and typecheck passed, and strict security
passed with zero violations, blockers or downgrades. Independent final checks
passed five restored maintenance tests and the retained observer test.

The full test collection found a pre-existing Windows WAL harness selector
mismatch: `test_windows_wal_attribution_ci_job.sh` inspects
`complete_erasure_at_rest`, but Slice 83 moved the observed checkpoint into
`checkpoint_erasure_at_rest`. Astra confirmed both files are unchanged from the
Slice 83 baseline and actual observation ordering is intact. No unrelated repair
was made. The remaining broad collection was stopped to prioritize the requested
Slice 84b planning handoff; uncompleted suites are not claimed passed. Logs are
`/tmp/fathomdb-slice84-final-verify.log` and
`/tmp/fathomdb-slice84-{lint,typecheck,security,tests}-resume.log`.
As of `04b08895e`, this was incomplete workspace verification, not a full-green gate. Slice 84 stays
unqualified; the follow-up [Slice 84b](../slice-84b/README.md) evaluated the
measured certification costs.

At the owner's request, primary-source web research and Astra applicability
review were completed before disposition. The
[research](design.md#web-research-before-disposition) identifies query-level
VM/full-scan attribution and conditional exact eligibility probes as the next
narrow investigation; transactional aggregate/invariant maintenance needs a
larger explicit authority/rollback/reopen design. It rejects generic writer,
checkpoint or PRAGMA tuning and unsafe FTS identity shortcuts. No further
implementation or measurement budget is implied. The existing cleanup service
regression remains unresolved; Slice 90 and the open guarantee/API decision
remain separate work.

## Final closeout

Slice 84 and its 84b follow-up are complete as negative investigations. The stale
Windows harness selector was fixed RED/GREEN at `77e2de796` and all 344 checks
passed. Astra approved retained-code disposition and independently verified the
measurements and safety. Final retained-tree lint, remaining seven heavy suites
(including the Rust/Python/TypeScript workspace tests), eight post-Rust fast
registrations and the 84b controller passed; earlier completed harness, typecheck
and strict-security results are reused. This is scoped resumption rather than
a single uninterrupted full-gate pass. See [84b status](../slice-84b/status.md).
The service regression and end-of-83 guarantee/API question remain unresolved.
