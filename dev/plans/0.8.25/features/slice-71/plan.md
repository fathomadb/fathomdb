---
title: 0.8.25 Slice 71 — latency and ingest investigations
status: READY
depends_on: 60
design: design.md
design_status: APPROVED
---

# Slice 71 plan

## Outcome

Resolve two previously observed performance signals without weakening product
correctness: AC-013 vector-query latency and the Slice 35 bulk-ingest cost.
Slice 71 produces reproducible, commit-bound evidence and makes only a
targeted TDD correction whose cause is demonstrated. It does not execute the
release-wide regression matrix reserved for Slice 75.

## Reconciliation and audit

The post-Slice-60 handoff assigned both signals to Slice 75. The owner scope
adjustment dated 2026-09-08 moves them here. Review of work since their first
allocation establishes these corrections and constraints:

- AC-072 is authoritative only at release-mode 10k corpus, at least 1,000
  measured samples, p50 <= 80 ms, and p99 <= 300 ms. The 100k/1M cells are
  tracking-only and AC-013b recall is a separate report-only experiment.
- Slice 60 observed `n=10000 samples=1000 seed_ms=10264 p50_ms=919
  p99_ms=991` in a non-isolated run. The canonical runner currently selects
  tests by substring, so exact AC-013 selection is itself a required harness
  correction.
- The Slice 35 data contain six preserved five-repetition campaigns. Their
  candidate median 10k-ingest regressions range from 80.71% to 103.56% versus
  the Slice 30 parent. The final measured candidate is `0aff1cb`; the baseline
  is `b2bfb1f318f58041144acb2356a6a4c9624068b9`.
- “42 per-row triggers” was incorrect. Slice 35 had 42 triggers total (14
  tables times three). The current schema has 54 total triggers (18 times
  three), while this fixture fires three INSERT triggers per node and rotates
  one 32-byte nonce per trigger. A 10k ingest therefore expects 30,000 ingest-
  caused visibility-generation events, plus independently recorded setup
  events.
- Existing trigger, rollback, frozen-read drift, cross-process, and virtual-
  table coupling are product invariants. Diagnostic ablations are not product
  candidates.

## Requirements

- **S71-R1:** Bind every comparison to exact source commits, toolchain,
  compiler mode, SQLite/runtime identity, host pressure, configuration,
  fixture digest, and raw output.
- **S71-R2:** Run AC-013 in isolated release mode with a fixed 384-vector
  dimension and separate seed/drain timing. Use the same exact runner for both
  arms and three counterbalanced repetitions per arm.
- **S71-R3:** Add an opt-in `test-hooks` reader trace of normalized search-
  statement identities before modifying search. A correction is permitted only
  when a deterministic fixed-fan-out test observes 24
  `post_filter_source_lookup` invocations before GREEN and zero afterward, and
  SQL eligibility still excludes barred vector-node, node-FTS, vector-edge,
  and edge-FTS hits before truncation.
- **S71-R4:** Reproduce ingest against fresh databases using the existing 10k
  input and batch size 256. Record ingest acknowledgement, projection drain,
  row counts, trigger count, generation delta, nonce, database bytes, CPU,
  RSS, errors, and host pressure for three counterbalanced repetitions per
  arm.
- **S71-R5:** If candidate ingest remains more than 20% slower at the median,
  run production-trigger, generation-only/no-nonce, and no-op-body diagnostic
  cells before proposing a correction. These counterfactuals never constitute
  shipped behavior.
- **S71-R6:** Preserve visibility invalidation, committed-state distinction,
  rollback behavior, authoritative-table coverage, dependency eligibility,
  and all public/schema contracts. Stop for an owner decision if a remedy
  needs a schema, ADR, or public-contract change.

## Acceptance criteria

- **S71-AC1 — exact runner:** RED proves substring selection can include other
  tests. GREEN uses `--exact ac_013_vector_retrieval_latency`, preserves the
  cargo pipeline exit status, and emits a parseable raw log.
- **S71-AC2 — AC-013 evidence:** Three candidate and three branch-point runs,
  ordered `B,C,C,B,B,C`, each retain 1,000 warm-treatment samples and fixed
  dimension 384. Every repetition is judged independently against AC-072; an
  arm is passing only when all three repetitions pass and failing only when all
  three fail. Mixed results or a greater-than-20% within-arm p50/p99 range are
  `environment_invalid`. The total classification is `both_pass` (both pass),
  `candidate_regression` (baseline passes/candidate fails),
  `candidate_recovery` (baseline fails/candidate passes),
  `pre_existing_gate_failure` (both fail), or `environment_invalid`.
- **S71-AC3 — attribution:** The normalized reader trace binds any search
  change to the redundant-query hypothesis without depending on wall time or
  the slow-statement callback. Real-database tests for barred vector-node,
  node-FTS, vector-edge, and edge-FTS paths plus ineligible dependency
  degradation pass unchanged or are added RED before the common post-filter is
  removed.
- **S71-AC4 — ingest evidence:** Three candidate and three baseline fresh-DB
  repetitions retain all S71-R4 fields. The pre-visibility baseline records
  `trigger_inventory: 0`, `visibility_state: not_applicable`, null
  generation/nonce fields, and reason `schema_predates_visibility_state`; it
  never fabricates a zero generation delta. Candidate trigger and generation
  accounting distinguishes setup events from the expected 30,000 ingest
  events.
- **S71-AC5 — ingest disposition:** At or below 20% median regression, retain
  evidence and do not optimize. Above 20%, retain the ablation evidence and
  either land a narrowly proved TDD correction or stop with the precise
  invariant/decision blocker.
- **S71-AC6 — review and verification:** Independent design review precedes
  implementation. Tests are committed RED before GREEN. An independent code
  review and a separate focused-verification pass bind their verdicts to the
  exact candidate commit.
- **S71-AC7 — proportional gate:** Run only changed harness tests, relevant
  Rust/Python/shell tests, the two experiments, workspace clippy, and workspace
  check. Do not run `agent-verify`, `scripts/check.sh`, long stress, Windows,
  CUDA, packaged cross-SDK, or hosted CI; Slice 75 owns those full routes.

## Delivery sequence

1. Approve this plan and design through independent review.
2. Add deterministic harness and profiling tests; retain the failing RED
   diagnostics in `tdd-chronology.md` and commit the RED state.
3. Implement the minimum GREEN harness/search changes and run focused tests.
4. Execute the preregistered AC-013 campaign and classify it.
5. Execute the preregistered ingest campaign, then attribution/correction only
   if its threshold requires it.
6. Obtain independent code review and focused verification; resolve findings
   without changing the sealed experiment after seeing results.
7. Write `status.md`, update the release-state writer, and remove any temporary
   branches/worktrees created for baseline execution.

## Stop conditions

Stop rather than weaken a gate when AC-013 has a mixed-result arm or greater-
than-20% within-arm p50/p99 range, ingest acknowledgement has greater-than-25%
max/min spread within an arm, the environment identity drifts, both AC-013
arms fail, the candidate remains over
AC-072 after a proved local regression is removed, barrier correctness breaks,
ingest attribution requires changing a public/schema/ADR contract, or an
independent reviewer returns a blocking finding.
