---
title: Slice 80 — absolute read-performance acceptance and bounded verification
status: DRAFT
depends_on: 79
---

# Slice 80 — absolute read-performance acceptance and bounded verification

## Authority and placement

Owner ruling **seq-277** (2026-09-11; session decision
`TC-ac020-successor-0825`) replaces this slice's unchanged-AC-020 recovery
mandate. This is an acceptance-contract replacement, not another optimization
campaign. No renumbering: 79 remains complete; 80 owns this work; 85 retains
final verification, CI and non-publishing packaging. Reserved 78 and 81–84
remain unused.

The scope and thresholds are approved. This plan and [design](design.md)
require independent review and a sealed executable protocol before READY;
implementation and successor acceptance evidence remain pending. Do not
re-open the approved thresholds.

## Inputs and protected baseline

Consume Slice 79 closeout at `323db678`, its product candidate `a6650c81`,
and its [result](../../../runs/0.8.25-slice-79/result.md),
[manifest](../../../runs/0.8.25-slice-79/manifest.json) and reviews.
Preserve runtime configuration, performance-mode MEMSTATUS=0 and statement
reuse. Old AC-020 failed all seven runs despite medians of 174.632177 ms
sequential and 53.616722 ms concurrent; these remain historical failures.

All six 71B write cells passed. AC-072 met numerical limits in three runs,
but swap activity invalidated all three as acceptance evidence. The former
[experiment protocol](../../ac020-experiment-protocol.md) supplies historical
provenance and applicable measurement controls, not a continuing ratio gate.

## Approved successor

Specification derived from seq-277, for the unchanged 1,600-search fixture:

| Execution | Loud warning at or above | Hard limit, inclusive |
| --- | ---: | ---: |
| Sequential | 200 ms | 500 ms |
| Eight readers, 1,600 total searches | 80 ms | 100 ms |

Either exceeded hard limit fails. Warnings do not fail acceptance. Compare
full-precision durations; ratio is informational only. There is no hybrid
formula or additional 20% regression gate. Performance mode is acceptance
bearing; diagnostics remains descriptive.

## Work packages

1. **Contract registration.** Register an unused successor AC identifier
   through the acceptance/ADR process. Retire AC-020 while preserving its
   historical assertion/results. Update requirement/test/parameter mappings,
   ADR index, test-plan wiring, current release manifests and active selectors.
   Keep AC-072, AC-076, AC-073 and AC-015/016 unchanged.
2. **Reader independence.** Map REQ-018 to adequate existing deterministic
   coverage. If absent, add a focused real-database test proving another
   reader progresses while one reader is paused. Absolute timing alone
   cannot prove independent connections.
3. **Oracle/reporting TDD.** Stage or commit RED tests for the design's boundary,
   warning, missing-evidence and invalid-environment cases, then implement
   only the necessary harness/reporting changes. Preserve fixture, query mix,
   operation counts, result checks, mode and timing boundaries.
4. **Bounded acceptance.** Seal exact commands, toolchain/features, source and
   binary identities, executor/environment controls, timeout, positive counts
   and receipt paths before timing. Execute seven fresh performance-mode
   processes and the exact three-run AC-072 candidate campaign. No profiling,
   tuning sweep or repeated historical baseline.
5. **Review and handoff.** Independent design, code and evidence reviews;
   focused changed-target checks; input-invalidation map for Slice 85.
   Final artifacts and broad verification belong to 85, not this slice.

## Verification and stop policy

- Each valid successor run must satisfy both hard limits. Report every raw
  time and warning, medians and dispersion. Do not pass a campaign by averaging
  away a failed run or by interpreting zero/skipped measurements as success.
- AC-072 remains 10k/384d/1,000 queries, three repetitions, p50 <=80 ms and
  p99 <=300 ms with the retained Slice 71 environment policy. Swap activity
  invalidates evidence. Obtain valid candidate evidence, not a new historical
  baseline. Do not disable swap or terminate other applications without authority.
- Keep timing isolated from builds, profilers and other performance work.
  Permit at most one documented environment correction and one replacement
  bounded series for the affected gate; preserve all invalid evidence.
  Genuine performance failures are not eligible for repeat-until-pass.
- Reuse all six Slice 79 write receipts after checking relevant input identity.
  Only product-path changes invalidating them require rechecking the two 71B
  10k candidate workloads under their retained protocol; no historical reruns.
- Run diff-scoped lint/typechecks and focused unit/integration tests. Product
  defects return to focused TDD and a bounded scope decision. No speculative
  optimization, dependency fork, isolation, lookaside/cache tuning or new API.
- No broad suite, CI/platform/package campaign here. Slice 85 owns one final
  broad round; a second requires explicit owner authorization.

## Completion

Successor contract and oracle are implemented; REQ-018 coverage is proved;
seven successor runs and the AC-072 campaign have valid passing evidence;
protected write receipts remain applicable; independent reviews pass.

Report **AC-020 retired; successor passed**, never AC-020 recovered.
Warning-only outcomes do not block closure. Store receipts/reviews under
`dev/plans/runs/0.8.25-slice-80/`, with exact source/artifact/feature/fixture
identities and positive counts. Advance release state to 85 and regenerate
views only at actual completion. Slice 85 consumes applicable receipts and
completes the remaining inherited coverage.

No version cut, publishing, registry mutation, tags, push or merge to main.
