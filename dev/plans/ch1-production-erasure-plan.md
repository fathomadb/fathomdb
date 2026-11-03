---
title: CH1 — production erasure in two phases
status: ACTIVE
date: 2026-11-03
target_release: 0.8.28 Slices 82 and 83
---

# CH1 — production erasure in two phases

This is the approved CH1 plan replacing the earlier eager-first repair sketch
in the [follow-up sequence](current-history-followup-sequence.md#ch1--two-phase-production-erasure-proposal).
HITL approved the Astra-reviewed plan on 2026-11-03 and allocated Phase 1 to
[Slice 82](0.8.28/features/slice-82/plan.md) and Phase 2 to
[Slice 83](0.8.28/features/slice-83/plan.md), both before Slice 90.
The live release ladder is 82 → 83 → 90. Slice 82's exact design received Astra
approval and its implementation is in verification/qualification; see its
[status](0.8.28/features/slice-82/status.md). Slice 83 has not started and retains
its specified pre-code design and review gate.

At the end of Phase 2, HITL will decide the physical-erasure guarantee. HITL is
considering a specific API callers could invoke to ensure physical cleanup.
That is an end-of-Slice-83 consideration: neither its existence, shape, blocking
behavior nor guarantee is selected here. Approval of these phases covers
engineering and evidence, not a new numerical SLA or weaker existing success rule.

## Evidence and changes to the earlier proposal

Authoritative evidence: [Slice 81 results](../experiments/erasure-availability/RESULTS.md),
[native benchmark](../experiments/erasure-availability/native-benchmark.json), and
[requirement dispositions](0.8.28/features/slice-81/requirements.md).

| Finding since the original CH1 sketch | Consequence for this plan |
| --- | --- |
| Native fresh node erase took 264.781 seconds for 1,000 of 10,000 owners, five revisions each; logical removal succeeded but 27 literal markers remained. One idle warm-cache sample. | Profile and reduce native cost as a distinct task; add physical FTS sanitation. Do not present this duration as a bound, or lock occupancy as total erase time. |
| 34 of 72 campaign attempts completed physical proofs; 38 incomplete, including all 36 large fresh timeouts. | Candidate safety/legacy evidence is useful; integrated fresh behavior still needs qualification. Do not repeat or reclassify old outcomes. |
| Four qualified legacy comparisons reduced maximum maintenance writer hold by 86.8–99.3%; foreground write offers were still missed. | Prefer B's committed incremental sanitation, retain A as reference; qualify actual Engine availability separately. No service guarantee follows from page units. |
| Native registered-dependency physical debt globally fences writes; the POC did not narrow it. | Phase 1 retains the fence. Phase 2 needs an explicit ADR successor and integrated denial/publication tests before narrowing it. |
| POC sanitation admission followed native erasure. Crashes in that gap, same-source reingestion, projection races, broad routing, telemetry and high dependency fanout were unqualified. | Make these explicit production acceptance obligations, not inherited POC passes. |
| All four registered FTS families, including search_index_v2, can hold content. Optimized-input probe succeeded; post-delete segment geometry was not asserted. | Inventory each creation/recreation mode and positively populate every claimed family. Do not reject or certify merge by assumed geometry or no-work alone. |
| Expiry was deferred; C staged replacement was not implemented. | Keep authorized-history expiry and temporal changes outside CH1. Use C only after a separately reviewed proposal if B fails a real requirement. |

## Shared contract and scope

- Repair both public physical verbs, `erase_source` and `purge`, and their shared
  operator path. Preserve source provenance, dependency closure, deleted-first
  purge, exact survivor scope, reports and existing error precedence.
- Preserve current public success: erasure success cannot mean only accepted or
  logically absent. A blocked physical boundary returns the existing truthful
  incomplete/refusal outcome. No new public accept/status/cleanup API here.
- Cover every registered row-owned projection (FTS, vectors and terminal/work
  metadata), dependent closure, receipt redaction, original telemetry sinks and
  WAL. Inspect the current shared projection registry instead of copying lists.
- Physical proof concerns declared database/application-owned files and sinks.
  No new claim about SSD remanence, external backups, snapshots, swap or returned
  results. Tests use unique positive markers as finite witnesses, not a universal
  encoding proof. Production completion relies on reviewed mechanism invariants,
  registered-copy bookkeeping and successful finalization, not known test markers.
- Preserve format refusal before mutation; no cross-version migration. Any
  durable metadata/schema addition needs schema/admission design in the same
  phase; no legacy canonical data copy or INSERT…SELECT migration.
- Retained ghosts in admitted current-format files are a separate store-wide
  sanitation debt. A target-specific completion must not certify the whole store.
  Unknown historical source identities cannot be reconstructed from deleted rows.
- Existing [closure ADR](../adr/ADR-0.8.25-dependency-lifecycle-closure.md),
  [offline completion ADR](../adr/ADR-0.8.27-offline-physical-erasure-completion.md)
  and [Rust interface](../interfaces/rust.md) remain authoritative until a reviewed
  successor lands. Update affected Python/TypeScript/CLI contracts with behavior
  changes, preserving existing SDK parity and operator-only recovery boundaries.

## Phase 1 — physical correctness and native cost (Slice 82)

### Requirements and design work

1. Profile native erasure by closure/target discovery, projection drain, writer
   wait/hold, canonical and registered projection deletion, metadata/redaction,
   sanitation and checkpoint. Count scanned/affected rows and statements where
   useful. Inspect `erasure.rs`, the shared projection eraser, dependency closure,
   telemetry and schema FTS creation/recreation owners. Repeated projection work
   and unindexed FTS cursor deletes are hypotheses, not established dominant cost.
2. Implement the smallest measured native-cost correction. Prefer set-oriented
   deletion/shared inventory reuse where the actual profile justifies it; preserve
   atomic canonical/dependency semantics in this phase. No wholesale engine rewrite.
3. Integrate B-style sanitation under the real Engine writer: one negative merge
   start followed by positive continuation units with atomic durable progress,
   without restarting on each turn. Per-family applicability and completion
   conditions must be specified. A eager rebuild is the safety/reference fallback,
   not an unreviewed production fallback with hidden latency.
4. Persist the target's cleanup obligation and required lookup/proof information
   no later than the first committed destructive mutation, including cases without
   registered dependents. Reuse/extend native debt rather than add a parallel POC
   queue. State must survive loss of target canonical rows; commit work and progress
   together. A crash after external cleanup but before bookkeeping revalidates
   idempotently. Content-free status must not retain erased subject data.
5. Add a distinct, durable whole-store retained-term sanitation state for admitted
   files whose old deletion policy cannot certify clean history. Run it through
   the existing controlled erasure/recovery paths before those paths claim the
   applicable physical completion; no cleanup on every read. Specify how concurrent
   ordinary updates and table recreation preserve sanitation validity. A policy
   flag alone cannot mark old debt complete. Compare prospective per-table secure
   deletion with continued sanitation, measuring mutation cost before selecting it.
6. Preserve existing conservative read/publication/write fencing while debt is
   owed. Finish original-sink telemetry redaction and a successful truncate
   checkpoint before complete. Missing sink, busy reader or failed proof stays
   incomplete and retryable. Extend the existing offline diagnostic/completion
   path to recognize the new debt so it cannot falsely report a clean store.

Astra reviews the production design, exact persisted state/admission rules and
ADR/interface deltas before code. Phase 1 does not require asynchronous public
acceptance, narrowed global fences or a deadline scheduler. Exact retries remain
supported; automatic work is Phase 2. Existing engine-owned physical fault hooks
are preferred over additional arbitrary SQL interfaces.

### Acceptance and TDD groups

| ID | RED-first behavior and GREEN acceptance |
| --- | --- |
| CH1-1A | Shipping positive ghost: populated node, edge, property and v2 markers survive before the repair and disappear after complete. Verify content/MATCH/shadows/main file/applicable freelist/WAL and registered sinks, with exact survivors, including authorized history. |
| CH1-1B | Fresh and already-deleted ghosts: prospective policy cannot certify old files; interrupted legacy sanitation remains owed; retry converges without rebuilding on each ordinary operation. Recreated tables do not lose policy/readiness tracking. |
| CH1-1C | Durable admission: process exit before commit admits nothing; after deletion commit but before sanitation cannot lose debt, with and without dependents. Unit mutation/progress rollback together. Completion bookkeeping replay is idempotent. |
| CH1-1D | Busy WAL and unavailable/wrong original sink prevent success; exact retry and offline recovery recognize all debt, preserve scope and finish only after actual cleanup. |
| CH1-1E | Native-cost change preserves reports, dependency/source ownership, purge preconditions, exact row identities/content and survivor search order/score semantics against independent fixtures. Supported ordinary write/update/supersede/rebuild paths cannot recreate erased content. |
| CH1-1F | Paired baseline/candidate profiling shows a repeatable improvement in the dominant native stage and total native duration on the original 10k node workload; all measured costs and ordinary-mutation regressions are reported. No new numerical product latency gate is inferred. |

Cost work uses a human-authored deterministic access/work-count oracle where
possible, plus paired timing; do not manufacture RED from noisy elapsed-time
assertions. Already passing safety guards remain guards. A physical or recovery
failure blocks Phase 1 completion. If no repeatable cost improvement is obtained,
report the measured blocker for plan adjustment rather than declare that goal met.

Phase 1 exits with production physical-cleanup correctness under existing
conservative admission, integrated regression tests, measured cost changes and
explicit known limits. This does not choose the project's eventual guarantee.

## Phase 2 — resumable work and availability (Slice 83)

Depends on accepted Phase 1 results. Reconcile its measured bottlenecks before
final design; do not assume FTS merge is still the limiting stage.

### Requirements and design work

1. Extend existing debt to resumable native cleanup where profiling shows long
   writer ownership. Define immutable target generation/cut, closure membership,
   commit linearization and internal phases: admitted/denied, removing, sanitizing,
   finalizing, complete; blocked is orthogonal and never means discharged.
   Bound discovery/admission too, or explicitly refuse before acceptance when its
   resource envelope cannot be supported. No claim that rows/pages bound time.
2. Before incremental deletion can expose partial state, atomically install a
   native denial/publication barrier. Block same-source and affected-dependent
   reingestion until completion; stale queued/in-flight projection, repair and
   rebuild cannot republish it. Preserve unaffected-source writes between committed
   units. A reviewed successor to the global-fence ADR is required; never bypass
   the fence solely to pass an availability test.
3. Inventory every actual read route: body/property/dense/hybrid, graph/caps/seeds,
   relaxed/historical/frozen and evidence. New reads after denial must have coherent
   authorized membership/statistics or explicitly await/refuse the affected scope.
   Post-ranking filtering alone fails. Conservatively refuse uncertified routes;
   no new temporal backend or full current-isolation redesign is authorized.
4. Add finite internal backlog admission and maintenance service while the Engine
   is available, using existing lifecycle/runtime ownership where practical.
   Resume committed work on reopen under a reviewed successor to the current
   open/recovery policy; schedule without future user writes, handle close/join
   and failures, and prevent continuous unrelated updates from restarting cleanup
   indefinitely. Persist blockers and owed work. Measure service and drain rate;
   no numerical deadline, wall-clock lease or capacity-backed SLA is selected.
5. Qualify physical finalization with pinned and continuously arriving readers.
   Preserve current run-to-completion for existing reads; test a finite-attempt
   admission/drain window and truthful incomplete retry. Do not add cancellation
   or reader leases without a separately approved contract. Record any global
   pause and unbounded external-reader/downtime condition. An unsupported bound
   becomes input to HITL, not a false guaranteed completion.
6. Keep public erase/purge success meanings intact while internal units yield.
   An existing call may drive/wait for work or return its existing truthful
   incomplete error according to the reviewed interface; it cannot return an
   accepted-only success. No public cleanup trigger or new polling service is
   implemented before the end-of-83 guarantee decision.

Astra reviews denial/read-route coverage, scheduler ownership, successor ADRs,
interfaces and recovery design before implementation. If safe progress requires
new public semantics or broad temporal redesign, bring that concrete change back
for approval instead of hiding it inside Phase 2.

### Acceptance and TDD groups

| ID | RED-first behavior and GREEN acceptance |
| --- | --- |
| CH1-2A | Native rendezvous between committed units lets actual unrelated Engine reads/writes progress with real registered-dependency debt; affected mutations remain fenced. No adapter-only or disabled-gate witness. |
| CH1-2B | After denial, all supported routes either enforce correct scope before statistics/caps/seeds or explicitly refuse. Independently constructed survivor-only comparisons vary excluded target lengths, duplicates and fanout; exact IDs/order/score semantics and evidence authority hold. |
| CH1-2C | Same-source reingestion, projection publication and rebuild races cannot resurrect targets. Process crash/reopen resumes the immutable operation without caller target data, lost debt or double completion; idle maintenance needs no future user traffic. |
| CH1-2D | Finite saturation refuses before destructive acceptance; admitted work progresses under bounded unrelated updates. Close/error paths join workers and release connections/hooks. No unbounded per-job thread creation or hidden queue. |
| CH1-2E | Pinned and new readers exercise finalization. Busy/absent-sink/failed cleanup never completes; release/reattach plus retry converges. Report measured pauses, exclusions and maximum observed units separately from guarantees. |
| CH1-2F | Integrated loaded measurements reconcile scheduled/issued/completed/refused/unfinished operations, physical completion, queue/writer wait/hold, total lifecycle and finalization. Missed offers stay visible; no qualified p99 from partial or refusal-bearing streams. |

Phase 2 exits with these native correctness/availability witnesses, a supported
operating-envelope report, and the HITL decision package below. A missing safety
or availability criterion cannot be converted into a pass by a benchmark timeout;
unmet work needs explicit plan adjustment. It need not demonstrate an unconditional
physical deadline or invent an API to qualify as complete under this scope.

## Measurement, review and verification plan

- Reuse Slice 81 tools and frozen fixtures, plus production-path adapters where
  needed. Keep original results immutable. No third cleanup implementation or
  broad new harness. Time campaigns without simultaneous builds/tests.
- Phase 1: native stage profile at 1k/5k/10k node owners, fixed five revisions and
  10% targets. Three paired baseline/candidate 10k node runs, and one 10k edge and
  property comparison; use preserved master fixtures when valid. Test high fanout,
  1-MiB/common terms, Unicode and zero-token input with small focused fixtures.
  Compare representative ordinary write/update/supersede costs for any policy
  change. Time fixture construction and whole-store sanitation separately.
- Phase 2: three paired loaded runs per node/edge/property on the same 10k/five-
  revision/10%-target workload, using the Phase 1 version as baseline; reuse
  15-second offered read/write windows and no-erasure controls. Include the native
  phase, not only post-native sanitation. Extend accounting to all actual issued
  work; retain missing offer dispositions as unknown rather than refusals.
- Before each phase's timing, freeze commands, controls, source/SQLite/runtime,
  input hashes, ordering, resource ceilings and a budget informed by the Phase 1
  profile (264.8s node and prior 775s property preparation invalidate 30s native
  cutoffs). Submit any increased experiment budget for approval before running.
  Use small tests for faults/races; do not repeat the 72-cell grid to fill blanks.
- Each new behavior follows committed RED then GREEN, with tests read-only during
  fix-to-spec. State-machine/property tests cover admit/delete/unit/fail/reopen/
  retry transitions on real SQLite, with human-authored invariants. Exercise
  process-crash recovery; no power-loss claim from killing a process.
- Astra reviews phase designs and code. An independent verification subagent
  reruns selected RED/GREEN, positive physical proof, rollback/reopen, unavailable
  sink/WAL retry and native availability witnesses; independently checks outcomes
  and timing calculations. Use existing platform/SDK erasure suites in the blast
  radius, including Windows WAL paths for checkpoint changes.
- Scoped checks during work; one final `agent-verify.sh` per completed code phase,
  then only failed/affected suites for narrow repairs. Markdown-only planning uses
  `agent-lint-md.sh` and release-state checks if allocation later changes. GitHub
  CI inspection/dispatch uses `gh` only. No tag/publication authorization.
- Quality-roadmap application: real-database differential oracles, bounded state
  sequences, commit/external-sink fault injection and worker/resource cleanup.
  New evidence is native scan cost, unclosed admission gap and actual global
  fencing. Defer exhaustive OOM/VFS/MC/DC, external-file staging and new frameworks.

## End-of-Slice-83 HITL decision: physical-erasure guarantee

Bring back actual results and an explicit contract choice. This decision remains
open even if both phases pass their engineering acceptance:

1. Declared physical scope and what the application can observe/prove, including
   current-format old sanitation debt and original external-sink obligations.
2. Whether to retain only current erase/purge completion behavior, add a specific
   API that ensures physical cleanup, or pursue capacity-backed automatic service.
   HITL's API idea is a consideration, not a chosen solution or committed surface.
3. For a possible API: scope/identity, idempotence, concurrency, blocking versus
   incomplete outcomes, pinned-reader handling, crash/reopen behavior, unavailable
   sinks, offline Engine and binding parity. No signature is invented in advance.
4. Any numerical target/guarantee: supported size/fanout/load/backlog, clock and
   downtime accounting, resource reservation, admission/refusal, reader policy and
   escalation when blocked/overdue. Sample maxima cannot establish a worst case.
5. Exact successor ADR/interface changes and any additional implementation or
   qualification required. HITL approval precedes guarantee/API implementation.

Phase 1 is independently useful. CH5 production-current isolation still requires
CH1 correctness plus qualified maintenance; CH3–CH8 and ordinary expiry receive no
allocation here. Existing 0.8.29 Slice 100 owns broader interruption qualification;
reuse CH1 regression coverage there, without deferring CH1's own crash acceptance.

## Approval and review record

HITL approved the two-phase scope and acceptance on 2026-11-03 and directed
0.8.28 allocation 82 → 83 → existing 90. The release state, board and master plan
record this allocation. The physical-erasure guarantee/API decision stays at the
end of Slice 83. This allocation update changes no production code or contract.

Astra reviewed this plan and the reconciled CH1/CH2 follow-up on 2026-11-03:
APPROVE for HITL plan review and proposed allocation, with no blocking findings.
Source checks confirmed the no-dependent debt gap, existing offline proof/WAL/
telemetry boundary and need for a successor before automatic reopen completion.
Exact state, fencing and completion invariants remain required at each phase's
pre-code design review. HITL subsequently approved this plan and allocation; exact phase designs still
require the specified Astra review before code.
