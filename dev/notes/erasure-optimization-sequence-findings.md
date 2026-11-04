---
title: Erasure optimization sequence — delivered work, rejected candidates and findings
status: COMPLETE
date: 2026-11-04
---

# Erasure optimization sequence — Slices 81 through 84b

This is the durable synthesis of the completed mechanism, production repair,
resumability and optimization work. It is based on retained source `cd12e5e36`
and the slice records linked below. On 2026-11-04 HITL directed that this
optimization sequence be set aside and that a separately reviewed explicit
maintenance-action contract/API proposal be prepared before implementation.
No further optimization campaign follows from this record.

## Outcome

- Slice 81 established scoped physical-sanitation feasibility and exposed the
  large native cost; it did not repair production or qualify an SLA.
- Slice 82 repaired physical correctness/durable debt and changed dominant FTS
  deletion from per-cursor scans to set-oriented deletion. Node native calls fell
  from 271.090 to 1.229 seconds median, while eliminating observed residual markers.
- Slice 83 delivered autonomous idle/reopen recovery and safe scoped concurrency.
  It preserved truthful physical success but materially worsened loaded service.
- Slice 84's scheduling candidate and Slice 84b's proof/accounting candidate
  failed integrated service qualification and were rejected. Their production
  changes were removed; baseline safety, observers and measurement tools remain.
- Atomic canonical/dependency deletion remains justified. No new cleanup API,
  finite physical deadline or universally improved availability was established.

The product decision is whether callers should explicitly choose when to incur
physical-cleanup interference. It is the original end-of-Slice-83 guarantee/API
question, now informed by the follow-up evidence, not a new optimization gate.

## Evidence map

| Slice | Authoritative records | Retained result |
| --- | --- | --- |
| 81 | [Status](../plans/0.8.28/features/slice-81/status.md), [results](../experiments/erasure-availability/RESULTS.md), [native receipt](../experiments/erasure-availability/native-benchmark.json) | Bounded POC, immutable negative/incomplete outcomes and test-only seam |
| 82 | [Status and timings](../plans/0.8.28/features/slice-82/status.md), [production design](../plans/0.8.28/features/slice-82/design.md) | Durable production erasure and native cost repair; code head ab046b216 |
| 83 | [Status and service accounting](../plans/0.8.28/features/slice-83/status.md), [design](../plans/0.8.28/features/slice-83/design.md), [ADR](../adr/ADR-0.8.28-resumable-physical-cleanup.md) | Resumability and scoped safety; final source af84f12f8, closeout dcf48f28e |
| 84 | [Status](../plans/0.8.28/features/slice-84/status.md), [evaluated design/research](../plans/0.8.28/features/slice-84/design.md) | All 54 cells completed; scheduling candidate rejected |
| 84b | [Status](../plans/0.8.28/features/slice-84b/status.md), [evaluated design](../plans/0.8.28/features/slice-84b/design.md) | Eight-cell screen failed; candidate 1e9c6d584 removed at 426d22782; closeout cd12e5e36 |

These sources retain exact commits, manifests, raw locators, review and test
commands. The [experiment package](../experiments/erasure-availability/README.md)
retains reusable tools; large raw event streams and databases are local artifacts,
not embedded in this synthesis. Source hashes identify different experiments;
rows below must not be pooled as one statistical distribution.

## Slice 81 — mechanism feasibility and the native bottleneck

Actual Engine-created format-35 fixtures exercised node, edge, property and
search_index_v2 families under SQLite 3.54.1. The workload used 10,000 owners,
five revisions each, 1,024-byte primary payloads and 10% target owners. A compared
prospective secure deletion/legacy rebuild with B's committed negative-first,
positive-continuation FTS merging. C staged replacement was not implemented.

The 72 unique campaign attempts ended as 34 complete and 38 incomplete, without
replacement/retry. All 36 large fresh attempts hit native erasure timeouts. Four
qualified legacy comparisons reduced maximum maintenance writer hold by
86.8–99.3%, but missed foreground offers remained. This was post-native sanitation
occupancy, not an integrated native latency or refusal-free service guarantee.

A separately requested fresh node run completed native erasure in 264.781348412s
(265.87s whole command). It removed 5,000 revisions/1,000 target owners logically,
but retained 27 literal FTS marker traces. A successful native return therefore
did not establish physical cleanup. This was one idle, warm-cache sample.

Safety tests established finite positive physical witnesses, survivor/ranking
parity, transaction progress, process exits/reopen and blocked checkpoint behavior.
Native dependency debt still globally fenced writes. The erase-to-sanitation
admission gap, reingestion/projection races, broad routing, high fanout and sinks
required production follow-up. Ordinary expiry remained separate and unqualified.

## Slice 82 — production correctness and approximately 221-fold native improvement

The repair added root debt even without registered dependents, atomic admission,
durable per-family sanitation, original-sink registration/validation, late-event
safeguards and shared mapped-vector-aware batch deletion. It preserved conservative
global fencing, exact retry/offline recovery and synchronous physical success.

| Workload | Baseline native seconds | Candidate native seconds | Evidence scope |
| --- | ---: | ---: | --- |
| Node 10k, median of three pairs | 271.090288 | 1.228860 | 220.603-fold improvement; all three candidate physical witnesses clear |
| Edge 10k, one pair | 139.937053 | 1.365655 | One paired observation, physical markers 18→0 |
| Property 10k | No native result | 2.646438 | Baseline whole process timed out at 600.01s; no speedup or native lower bound |
| Node 1k, one pair | 2.506011 | 0.155377 | Scale context |
| Node 5k, one pair | 65.615555 | 0.754607 | Scale context |

Profiling located the dominant repair: 20,000 FTS DELETE commands across four
families became four set-oriented commands. Top-level FTS deletion time fell
from 268.764–270.185s to 0.222–0.226s. Merge SQL added approximately 86–87ms in
node fixtures. SQL cost excludes transaction/checkpoint overhead, and nested
shadow timings overlap. Baseline success still contained residual bytes, so this
compares corrected native-call performance, not two fully correct erasers.

Candidate-only mutation controls found prospective secure deletion cost 31.55%
more across the property sequence, 33.52% more during superseding rounds. Default
deletion plus controlled sanitation was retained. This unreplicated policy control
established neither a node benefit nor baseline-to-candidate write-regression proof.
Detailed writer/drain/checkpoint attribution moved explicitly to Slice 83.

## Slice 83 — safe progress with a measured service regression

Retained mechanisms include singleton/envelope admission, atomic logical removal,
committed sanitation units, an Engine-owned joined worker, idle/reopen resumption,
scoped unrelated plain writes and finite owned-reader finalization attempts.
Same-source/affected identities and endpoints, provenance/admin/publication changes
and application-trigger fallback remain fenced. Late telemetry cannot restore
erased identities. Missing sinks or pinned readers leave durable incomplete work.

Scoped concurrency requires exact proof of surviving writes: eligibility,
resource envelope, safe watermark, generation and structural zero. A redacted
finalization tombstone removes temporary target identities before TRUNCATE and
retains a global mutation fence until actual completion. That fence survives
window/reader-gate release and process reopen.

| Family | Phase 82 loaded call median (s) | Slice 83 candidate (s) | Completed writes, baseline→candidate |
| --- | ---: | ---: | --- |
| Node | 1.354 | 8.070 | 1365→729 of 1500 |
| Edge | 1.394 | 3.393 | 1361→1182 of 1500 |
| Property at 100-Hz offered writes | 3.108 | 16.176 | 598→26 of 1500; no-erasure controls also overloaded |

Property at 20-Hz writes had passing 300/300 controls, but baseline/candidate
erasure windows completed 247/82 writes. A corrected final-build node smoke
retained the direction: 1.384→7.237s and 1361→807 completed writes.

Candidate cumulative physical writer waits were 6.638/1.952/13.355s for
node/edge/property; atomic native holds were 0.740/0.925/1.353s. These inclusive
counters overlap and are not additive. Interleaved admission/certification and
writer contention, not native deletion alone, motivated the follow-up.

The witnesses establish real progress between units and recovery correctness.
They do not establish improved overall availability. Public-call duration is not
a continuous global database pause, but lost throughput makes the interference
material. No low-latency logical-revocation-only operation was implemented.

## Slice 84 — scheduling change rejected

Added identical baseline/candidate stage observers and a fixed 54-cell protocol.
Actual node profiling attributed 3.020s to eligibility and 4.038s to certification
within an 8.612s call; nested envelope/proof stages overlap. Boundary MAX was only
0.077s, so the small MAX optimization was rejected before implementation.

Synchronous-sanitation priority moved waiting from primary mutex acquisition to
the existing writer turn. It did not remove the expensive certification of an
already admitted writer. The candidate passed scoped safety but failed every
family's integrated S84-6 gate.

| Family | Final83 baseline median (s) | Candidate median (s) | Completed writes median |
| --- | ---: | ---: | --- |
| Node | 7.783 | 7.705 | 757→764 /1500; read/control criteria failed |
| Edge | 3.291 | 3.404 | 1192→1182 /1500; every pair worse |
| Property at 20-Hz writes | 15.412 | 13.957 | 26→44 /300; inconsistent pairs/read loss |

Idle baseline observations were 1.436s node, 1.461s edge and 2.963s property.
These are individual idle observations, not three-pair medians, new API timings
or upper bounds. Single-pair Phase 82 context remained much faster under load.
All 54 cells completed physical witnesses, but that did not turn the failed
service screen into a qualified optimization. Original scheduling was restored.

## Slice 84b — exact work savings rejected after integrated screening

The candidate used a connection-bound, process-local cleanup certificate and
actual transactional resource deltas. It retained full admission/reopen/final
proofs, original scope, rollback/COMMIT behavior, trigger denial and physical
success. Focused tests covered exact SQLite bytes/NULL/history/property/retention,
thresholds, actual commit failure and the external COMMIT/publication race.

Isolated warm commits eliminated repeated full envelope reconstruction and zero
proofs. Small-store VM work was 7411 cold versus 6725 warm. Integrated execution
did not remain warm: node reconstructed totals for all 33 modeled commits, edge
for all 21. Exact invalidation causes were not instrumented; assigning them to
one maintenance transition would be speculation.

The first node screen changed 7.632→7.245s and 771→808 writes, with unchanged
read completions. Edge changed 3.223→3.408s, 1198→1181 writes and 1487→1486 reads.
Matched controls passed. Edge failed the predeclared screen; eight cells executed
and 34 remaining dispositions were explicitly not run. Property and later pairs
therefore provide no result, median or p99 claim. Astra recommended rejection;
baseline production and original tests were restored. Candidate-only work savings
do not establish user-visible improvement.

## Verification and what completion means

Every production behavior was developed/reviewed with RED/GREEN and real-database
safety oracles. Astra reviewed designs, code/corrections and measurement outcomes;
independent verification checked safety and offer accounting. Failed optimizations
were removed without weakening the original safety tests.

Verification was assembled through full collectors plus scoped repairs/resumption:
81 collected 204/205 suites then passed the corrected inventory check; 82 collected
203/205 then passed corrected targeted suites; 83 collected all 205 with four
failed suites resolved through scoped corrections/reruns. Slice 84 verification
was interrupted for 84b; its stale Windows checkpoint-owner selector was fixed
RED/GREEN with 344 checks passing. Final retained-tree lint and remaining heavy
Rust/Python/TypeScript/native suites, fast registrations and controller passed,
reusing unchanged typecheck/security and earlier completed harness checks.
This is not one uninterrupted final all-workspace green invocation.

Linux physical/WAL evidence does not qualify every Windows/hardware path. Model,
long-running and platform-specific exclusions remain as recorded in each status.
Finite marker tests support reviewed mechanisms; production cannot find every
possible data encoding by searching known test markers. No storage-remanence,
snapshot, external backup, swap or already-returned-result erasure was established.

## Decisions supported and work now set aside

Retain Slice 82's native correction and Slice 83's safety/recovery; retain useful
observers/tools. Reject the 84/84b production candidates. Set aside further
certification/scheduler optimization: no automatic 84c, new campaign, proof cache
or wider index/accounting program. Reopening that work requires a concrete need,
separately reviewed scope and new authorization.

The next proposal is [explicit maintenance erasure](../design/explicit-maintenance-erasure/README.md),
not a claim of faster background cleanup. Its approval must decide accepted versus
logically excluded versus physically complete, who grants disruption, durable
blockers and reopen behavior, existing erase/purge compatibility and exact scope.
At the time this synthesis was written, the original end-of-83 guarantee/API
question was open. The subsequent approval is recorded below. This write-up
itself changes no shipped API, contract or ladder.

## Subsequent contract approval — 2026-11-04

After this synthesis and Astra review, HITL approved the explicit-maintenance
contract and assigned [Slice 85](../plans/0.8.28/features/slice-85/plan.md) to
implement it separately. This resolves the original end-of-83 decision without
changing the historical findings above. No timing guarantee or immediate logical
revocation was approved; rejected optimizations remain set aside.
