---
title: FathomDB 0.8.28 Slice 50 — plan
status: revised for Unit S implementation; awaiting design review
target_release: 0.8.28
observed_on: 2026-11-01
---

# Slice 50 plan (`DRAIN-SCAN-BOUND`)

Baseline: `630af0bfcb0c5779d436ddbd5ffe3a0e876574b5` on `dev/0.8.28`.
Complete the remaining scan Unit S using the existing slice worktree. The
provider subunit is complete; its [record](built-in-provider-implementation-record.md)
remains authoritative. No Steward or Orchestrator workflow is used.

## Changes since the draft and disposition

1. The original `ac83688a1` plan used baseline `544bf74f8` and proposed
   two units: scan bounds and a refused-runtime drain change. SQL inspection
   at the new baseline confirms both scan statements remain unbounded.
   Retain Unit S, its equivalence proof, VM-step check and scaling study.
2. Characterization (`2990ebbf1`, `5d0dacea5`) moved the refused-runtime
   question from drain behavior to successful-open policy. Owner `seq-359`
   approved the narrow provider contract. Its implementation is complete
   through `db2baf1ec`, recorded at the current baseline. Actual refusal now
   fails open without returning an Engine. Reject implementing the obsolete
   D50-1/D50-2 choices or copying the now-invalid refused-handle fixtures.
3. Existing TC-68 cache behavior and Mean pooling are preserved by that
   completed subunit. Unit S changes neither provider admission nor cache,
   runtime configuration, errors or binding surfaces.
4. Slice 42 supplied SQLite 3.54.1. The prototype's 33 s to 5.7 s result
   used Slice 60's 64-row layout; this slice still uses format 34 and
   1,024-row chunks. Measure on the current baseline and default cache;
   retain the predeclared scaling thresholds, without claiming the prototype
   timing or the later AC-013 latency goal.
5. Allocated D28-12 latency work is divided across Slices 50–70; only
   pending-scan work belongs here. Release-state rulings move D28-01–07,
   D28-09–11 and populated-open work elsewhere; D28-14 is Slice 90.
   No draft candidate adds work to this slice.
6. Simplify the draft's ceremony: one private SQL-builder extraction and
   test hook, then one scan RED/GREEN pair; no artificial compile-failure
   commit for a test hook. Equivalence guards may pass at RED and are
   identified as guards. Independent Astra reviews design and code; a
   separate agent verifies. Avoid unrelated long latency gates already
   known to need Slices 60–90.

## Execution

1. Review [requirements](requirements.md), [design](design.md) and
   [test plan](test-plan.md) with an Astra design review; address findings.
2. Extract the two SQL builders unchanged, add a test-only page/plan/VM-step
   hook and equivalence fixtures. Establish that guards reproduce baseline.
3. Commit failing plan/cost tests (RED); bound each arm (GREEN). Preserve
   predicates, generation join, pagination, in-flight handling and rewind.
4. Commit the reproducible scaling probe and small driver/analysis with
   measurement correctness tests. Run paired current-baseline/head cells.
5. Astra code review; a separate test agent runs the affected suites and
   one final full gate. Fix findings with scoped reruns.
6. Record actual results and limitations in status, CHANGELOG and release
   state. Merge to `dev/0.8.28`, then clean the slice worktree and branch.

No schema, public production API, CUDA policy, cache or binding change is
needed. The test hook is behind `test-hooks`; its contract is in the design.
