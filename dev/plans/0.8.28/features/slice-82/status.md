---
title: Slice 82 status
status: COMPLETE
date: 2026-11-03
---

# Slice 82 status

Complete on `dev/0.8.28` and `release/0.8.28`, code head
`ab046b2167c64b5edc3109c0d9cd087ec4ef8487`. Requirements and baseline deltas are
in [plan](plan.md); the [production design](design.md) received Astra approval
before code. Astra code review and independent correctness/measurement audits
passed. Slice 83 is next; no physical deadline or cleanup API was selected.

## Delivered and accepted

| Acceptance | Evidence |
| --- | --- |
| CH1-1A | All four FTS families positively populated; shadow/content/MATCH/main-file/live-WAL witnesses; exact survivor rows/history and independent survivor-only BM25 order/score bits. |
| CH1-1B | Legacy ghosts and dirty-only offline cleanup; optimized, Unicode, zero-token and 1-MiB inputs; clean-to-update/rebuild invalidation; genuine fresh-store certification distinguished from missing existing metadata. |
| CH1-1C | Root-only and dependent process exits before/after deletion commit; merge/progress rollback and positive continuation across exit/reopen with asserted segment geometry; completion replay; 24 real-SQLite randomized retry sequences; exact mapped non-head vector IDs. |
| CH1-1D | Busy WAL, absent/wrong/multiple original sinks, malformed/unbound sink debt, late materialized search emission and external-rewrite crash replay; exact retry/offline completion preserve owed debt and scope. |
| CH1-1E | Reports, provenance/dependency closure, receipts, deleted-first purge, shared vector deletion and projection publication guards retained; original mutation-owner manifest and existing erasure/CLI regressions pass. |
| CH1-1F | Three completed paired original 10k-node runs establish repeatable dominant-stage and total native improvement. Edge/scale comparisons complete; property baseline supplemental comparison remains explicitly incomplete below. |

Native erasure now admits root debt atomically even without dependents. FTS
sanitation uses committed negative-first/positive-continuation units with durable
progress. Original telemetry sink fingerprints are registered before capture;
malformed/unbound debt refuses, and late events cannot republish erased IDs.
Scalar/batch deletion retain one registry mutation owner and mapped-vector checks.
Conservative global fencing and synchronous physical-success meanings remain.
No scheduler, new public command, migration or eager-rebuild fallback was added.

## Cost qualification

Release profile, performance runtime, SQLite 3.54.1, fixed offline provider,
warm host, no concurrent project builds/tests. Frozen original Slice 81 masters
and baseline/candidate binaries were hashed; all hashes matched after the run.
Baseline product is `bdadf19f6`; the candidate is the code head above.

| Workload: owners / revisions / target share | Baseline native seconds | Candidate native seconds | Literal markers, baseline → candidate |
| --- | --- | --- | --- |
| Node 10k / 5 / 10%, pair 1 | 271.280543 | 1.228860 | 27 → 0 |
| Node 10k / 5 / 10%, pair 2 | 269.826238 | 1.213349 | 27 → 0 |
| Node 10k / 5 / 10%, pair 3 | 271.090288 | 1.233843 | 27 → 0 |
| Edge 10k / 5 / 10%, one pair | 139.937053 | 1.365655 | 18 → 0 |
| Property 10k / 5 / 10% | Unavailable: process timeout | 2.646438 | Unknown → 0 |
| Node 1k / 5 / 10%, one pair | 2.506011 | 0.155377 | 20 → 0 |
| Node 5k / 5 / 10%, one pair | 65.615555 | 0.754607 | 24 → 0 |

Node median: **271.090288 → 1.228860 seconds**, **220.603×**, 99.5467% less
native time. All six completed pairs have identical reports. The 10k node
report is 5,000 nodes/15,000 projections; edge is 5,000 edges/25,000 projections.
The completed property candidate reports 5,000 nodes/19,000 projections.
Baseline calls returned success with residue: this is corrected native-call
performance, not a comparison between two physically complete implementations.

Top-level FTS deletion profiles explain the change: the original 10k-node runs
issued 20,000 cursor DELETE commands across four families, totaling
268.764–270.185 profiled seconds. Candidate calls issue four, totaling
0.222–0.226 seconds. The human-authored 20-target oracle falls from 80 to four.
Candidate merge-command profiles sum to 86–87 ms for node, 44 ms edge, 215 ms
property, 10 ms at 1k and 50 ms at 5k. These are SQL command costs, excluding
transaction/checkpoint overhead; nested shadow timings overlap and are not added.

Ordinary-mutation controls use the candidate, 1,000 owners and five revisions:

| Family/policy | Initial write seconds | Mean of four superseding rounds, seconds |
| --- | --- | --- |
| Node/default | 0.078247 | 0.155184 |
| Node/prospective secure-delete | 0.066059 | 0.140388 |
| Property/default | 0.117935 | 0.489850 |
| Property/prospective secure-delete | 0.116428 | 0.654055 |

Prospective secure deletion costs 31.55% more for the full property sequence
and 33.52% more for its superseding rounds. Retain default deletion plus merge
sanitation. This single, unreplicated policy comparison establishes no statistical
node benefit and measures no baseline-to-candidate ordinary-write regression.

Fixture preparation is measured separately. Recorded process wall time totals
1,659.48 seconds; adding the initial 277.21-second diagnostic is 1,936.69 seconds,
within 45 minutes. Manifest-to-final-result elapsed time plus the diagnostic was
2,004.41 seconds, also within that envelope. Peak recorded RSS is 917.664 MiB;
native commands had an 8-GiB address-space cap. Completed scratch observation is
about 4.06 GiB against a 12-GiB
allowance; its historical peak was not sampled. Source and commands are retained
in the [existing experiment package](../../../../experiments/erasure-availability/README.md).
Raw generated outputs are local under `/tmp/fathomdb-slice82`, not tracked.

## Verification and explicit limits

- Behavioral fixes used committed RED/GREEN; failing assertions were preserved.
  Fresh-state setup required a fault-fixture upsert after new metadata became
  present; assertions were retained. Existing CLI/mutation-manifest oracles were
  preserved. Narrow review findings have their own failing tests and fixes.
- Eighteen focused integration tests and five SQLite units pass, including real
  child-process crashes and the 24-case property sequence. Independent Astra
  reran the new coverage and 55 existing erasure/dependency tests, then verified
  the narrow final corrections and measurement arithmetic.
- One full final code-phase collection ran all 205 suites: 203 passed; root
  inventory metadata and Rust were the two failing suites. Rust named three CLI
  targets (fresh empty stores falsely dirty) and the mutation manifest (parallel
  batch owner). Corrected these with the fresh-only certificate and original
  shared mutation primitive, preserving existing contract tests. All four failed
  Rust targets pass targeted reruns; inventory passes all eight checks. Rust
  clippy, module boundaries and Markdown checks pass. No second full collection
  or claim of a fresh all-workspace green run.
- Linux verification does not qualify Windows-specific WAL behavior. Marker
  witnesses are finite; reviewed mechanisms/registered ownership establish
  completion, not searches for known markers in production.
- Astra approved narrowing original Phase 1 requirement 1: dominant SQL/merge
  profiles and total native cost are measured. Drain, writer wait/hold and
  checkpoint are **not separately measured**; detailed stage instrumentation
  carries into Slice 83. The original all-stage profile is not claimed fulfilled.
- Property baseline is **incomplete**: whole-process timeout at 600.01 seconds,
  exit 124, blank result, no native-duration observation. There is no property
  speedup, native lower bound, matched baseline report or baseline physical
  verdict. Astra accepted this supplemental comparison as an unfulfilled
  qualification limitation; core CH1-1F's three node pairs remain satisfied.
- No availability, p99, unconditional physical deadline, storage-remanence or
  external backup/rotated-copy guarantee follows. Unregistered historical sink
  paths cannot be reconstructed; unbound legacy telemetry debt stays owed.

## Landing and cleanup

Existing development/release checkouts were used serially and fast-forwarded;
no new worktree or branch was created. Disposable Slice 82 SQLite copies are
removed after result preservation. Original Slice 81 masters, unrelated main
checkout deletions and shared caches remain untouched. No tag or publication.
