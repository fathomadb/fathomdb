---
title: 0.8.25 Slice 71 — AC-072 read latency and 71B write recovery
status: IN_PROGRESS
depends_on: 60
design: design.md
design_status: REVIEW_REQUIRED
updated: 2026-09-09
---

# Slice 71 plan

## Current boundary

Slice 71 has two independent performance tracks. The write track, 71B, is
complete at product candidate `eda95b07`; its correction, focused tests,
bounded measurements, code review, and evidence audit must not be reopened or
traded away. The remaining track is AC-072 vector-read latency.

The [71B recovery record](71b-performance-recovery.md) reports Scale-02 10k
acknowledgement/total at 1,403/1,408 ms, about 23% faster than historical, and
projection-active 10k total at 1,311 ms, about 44% faster than historical.
All registered small-write criteria passed.

The retained AC-072 campaign is not a pass. Its branch-point baseline p50 is
stable at 163–165 ms, already about twice the binding 80 ms limit. The Slice 71
candidate p50 is 200–201 ms, about 22% slower again, and candidate p99 spread
makes the registered classification `environment_invalid`. Removing only that
recent increment would still leave AC-072 failing.

Naming is historical: the executable test remains
`ac_013_vector_retrieval_latency`, but it enforces AC-072's revised limits. It
times full `Engine.search`—query embedding, vector retrieval, the text-search
arm, fusion, and result processing—not only vector distance calculations.

Historical comparisons must remain distinct. The accepted real-embedding
36/49 ms result used 7,667 records. The closer synthetic comparison used
10,000 records, 384 dimensions, and recorded 15/17 ms. Its exact runner,
runtime, and configuration must be verified from retained evidence before it
is used diagnostically. Neither historical result substitutes for the
prospective Slice 71 acceptance campaign.

## Outcome

Explain and correct the AC-072 discrepancy without weakening search semantics,
changing its p50/p99 limits, or losing 71B's completed write recovery. Close
Slice 71 only when the exact prospective candidate passes AC-072, the 71B gains
remain protected, focused reviews pass, and the release-state writer advances
to Slice 72. An explicit later owner disposition may replace a passing result,
but must be recorded as an exception or deferral rather than a pass.

## Requirements

- **S71-R1 — protect 71B:** Treat `eda95b07` and its retained evidence as the
  write-performance baseline. Do not assume another write optimization will
  improve read latency. Avoid the governed write, visibility, projection, and
  drain paths unless a measured AC-072 cause requires them.
- **S71-R2 — establish equivalence:** Before product changes, bind the current
  and historical fixture definitions, exact 10,000 canonical/vector rows,
  dimension 384, corpus/query seeds, synthetic embedder, candidate fanout,
  query count, warmup, public operation, build mode/features, SQLite and
  sqlite-vec versions, CPU identity/instructions, affinity, and host controls.
  Record effective values, not intended defaults.
- **S71-R3 — use the real acceptance path:** AC-072 times full
  `Engine.search`, including its text arm. Vector-only measurements may
  attribute a component but cannot establish acceptance.
- **S71-R4 — decompose before campaigning:** Instrument one representative
  slow search with bounded, diagnostic-only tracing. Attribute query embedding
  and serialization, reader dispatch/wait, vector eligibility, binary KNN,
  float reranking, canonical hydration, FTS retrieval/eligibility/ranking, and
  fusion before selecting a correction.
- **S71-R5 — inspect work, not guesses:** Retain statement execution counts,
  rows processed, preparation cost, SQLite VM/full-scan/sort counters where
  available, and `EXPLAIN QUERY PLAN`. Acceptance timing must run without
  intrusive tracing.
- **S71-R6 — preserve semantics:** Keep eligibility before cap, lifecycle and
  dependency rules, ranking/fusion behavior, snapshot consistency,
  cross-process invalidation, and time-dependent validity. Never cache
  eligibility solely by write generation.
- **S71-R7 — TDD correction:** Add a deterministic RED test for the measured
  dominant mechanism before product edits. Preserve its oracle through GREEN
  and run only affected correctness suites plus affected-crate check/clippy.
- **S71-R8 — exact evidence:** Run the registered release-mode AC-072 campaign
  at 10k/384 with 1,000 measured queries after warmup, three repetitions per
  arm in `B,C,C,B,B,C` order, and the existing environment-validity rules.
- **S71-R9 — preserve write recovery:** If product code changes, rerun only the
  two 71B 10k candidate workloads under their retained protocol. Compare them
  with the protected 71B results and unchanged historical limits. Do not rerun
  historical write baselines.

## Measurement-equivalence audit

The current fixture defaults to 768 dimensions unless the environment
overrides it. Existing Slice 71 evidence records 384 dimensions, so verify the
effective execution rather than inferring dimension drift. Also exclude
inherited experimental environment variables and require no active background
projection work during measured searches.

The historical synthetic 10k record was produced from a documented working
tree with environment-tunable dimension/drain settings. Recover its exact
runner and runtime from retained artifacts where possible. If an exact field
cannot be recovered, mark it unavailable and use that observation only as a
diagnostic reference—not as a matched acceptance arm.

## Diagnostic decomposition

| Component | Question to resolve |
| --- | --- |
| Query embedding and serialization | Did runtime configuration or preprocessing become expensive? |
| Reader dispatch and waiting | Is elapsed time CPU work or queue/lock delay? |
| Vector eligibility | Does every query scan or join the entire corpus? |
| Binary KNN and float reranking | Did execution plan, runtime, or fanout change? |
| Canonical hydration | Are lookups indexed, repeated, or using expensive correlated predicates? |
| FTS and fusion | Does the synthetic query match many rows and make the hybrid text arm dominate? |

Begin with two code-grounded suspects:

1. `vector_arm_requires_fallback` checks for any unsafe vector row through
   canonical-state joins. On an entirely eligible corpus, proving that no such
   row exists may scan the full vector set. Measure its plan, VM steps, rows,
   and elapsed contribution.
2. Full hybrid search runs the FTS arm with lifecycle/dependency predicates and
   ranking. The streaming optimization is limited to direct text-only search,
   so AC-072 does not inherit it automatically. Measure match count, cap
   placement, eligibility work, ranking, and fusion.

Inspect canonical hydration against the actual schema and query plan. Existing
comments about absent cursor indexes are not authority when migrations now
create those indexes.

## Correction selection

Choose the smallest correction supported by the dominant measured component.
Permitted shapes include prepared-statement reuse when preparation is material;
join/correlated-predicate rewrites or a justified index when scanning dominates;
FTS execution improvements that preserve eligibility-before-cap and
ranking/fusion; removal of proved redundant checks within one snapshot; or a
runtime/fixture correction when the product path is not responsible.

Do not reduce candidate counts, skip eligibility, disable a search arm,
substitute vector-only search, change the fixture after seeing results, or
loosen AC-072. Any cache must bind the read view, effective time,
cross-process mutation authority, and snapshot identity.

## Acceptance criteria

- **S71-AC1 — equivalent basis:** A durable comparison record accounts for
  every S71-R2 field and clearly separates the 7,667-row real result, the 10k
  synthetic historical result, the branch-point baseline, and the prospective
  candidate.
- **S71-AC2 — causal attribution:** Bounded diagnostic evidence identifies the
  dominant component and demonstrates the selected mechanism. A vector-only
  microbenchmark or wall-clock-only observation is insufficient.
- **S71-AC3 — deterministic correction:** The selected mechanism has committed
  RED/GREEN evidence and preserves all affected search, lifecycle, dependency,
  eligibility, ranking, and snapshot contracts.
- **S71-AC4 — AC-072 passes:** A new committed final-candidate manifest binds
  the exact source, executable, runner, environment rules, workload, and output
  path without modifying the retained v1 campaign. Its receipt is written
  under `dev/plans/runs/0.8.25-slice-71/ac072-final/`. All three valid
  prospective candidate repetitions satisfy p50 <= 80 ms and p99 <= 300 ms.
  Passing p50 alone, an unstable arm, or a vector-only result does not close
  the gate.
- **S71-AC5 — 71B remains recovered:** When product code changed, the two
  candidate-only 71B 10k checks remain within their historical limits and do
  not regress more than 10% from the protected `eda95b07` medians: Scale-02
  acknowledgement <= 1,543.539 ms and total <= 1,548.545 ms; projection-active
  AC-013 total <= 1,442.198 ms. Scale-02 acknowledgement/total and AC-013 total
  each retain the <= 25% spread rule; every cell retains the 71B environment
  rules. AC-013 acknowledgement remains reported but is not a gate because it
  is the variable asynchronous work partition. If no product code changed,
  Git proves the measured product sources are unchanged and no write rerun is
  required.
- **S71-AC6 — focused review:** Independent design review precedes
  implementation. Independent code review and a separate evidence/verification
  review pass on the exact final product candidate.
- **S71-AC7 — state closure:** `status.md` and the release-state JSON record the
  exact candidate and evidence; generated views agree; Slice 71 becomes
  `COMPLETE_ON_RELEASE_BRANCH`; `next_slice` becomes 72; Slice 75 consumes the
  Slice 71 receipts without repeating their historical investigations.

## Execution sequence

1. Complete the measurement-equivalence audit and update design v5 from actual
   runner, runtime, schema, and query-plan evidence.
2. Obtain independent design review before product implementation.
3. Add bounded diagnostic instrumentation and decompose one representative
   slow full `Engine.search` call.
4. Select the correction from the dominant measured component and commit its
   deterministic RED test before GREEN implementation.
5. Run affected correctness suites and affected-crate check/clippy.
6. After the final product candidate is committed, commit a new AC-072
   final-candidate manifest and validator fixture, leaving the v1 manifest and
   receipt untouched; then run that exact prospective campaign once.
7. If product code changed, run only the two candidate-side 71B 10k checks.
8. Obtain independent code and evidence reviews, resolve blocking findings,
   update status/release state, remove owned temporary artifacts, and advance
   to Slice 72.

## Verification boundary

Slice 71 uses focused tests, bounded diagnostics, the exact AC-072 campaign,
and—only after product changes—the two candidate-side 71B checks. Do not run
`agent-verify`, `scripts/check.sh`, long stress, Windows, CUDA, packaged
cross-SDK, hosted CI, or a historical write-baseline campaign. Slice 75 owns
the integrated full regression matrix.

## Stop conditions

Stop without weakening a gate when fixture/runtime equivalence cannot be
established, environment identity drifts, a candidate arm is mixed or exceeds
the registered spread rule, the candidate misses either AC-072 boundary, the
selected correction changes a public/accepted contract, or a product change
spends the recovered 71B write margin. Retain all observations. A replacement
campaign or release-specific AC-072 exception requires an explicit prospective
owner decision.
