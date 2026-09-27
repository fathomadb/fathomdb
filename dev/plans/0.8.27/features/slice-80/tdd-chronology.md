---
title: FathomDB 0.8.27 Slice 80 - TDD chronology
status: IN_PROGRESS
started_on: 2026-09-27
baseline_sha: bb077cfa
---

# Slice 80 TDD chronology

## Baseline and method

Production source at commission is byte-identical to `bb077cfa`; the commits
after it are Slice 80 planning and review records only. Slice 80 is a structural
move and fixes no known behavior defect.

Every new characterization follows this recorded sequence:

1. add the test without changing production and show it GREEN against the
   shipped behavior;
2. apply only the temporary mutant named in `design.md` and show the test RED
   for the intended assertion;
3. restore production exactly, show the focused test GREEN again, and confirm
   no mutant diff remains.

If a new test is RED against unmodified production, that is a behavioral defect,
not characterization. Stop the move, preserve the RED oracle, implement the
smallest GREEN correction, and obtain review before resuming. Structural moves
must first reach compile RED on the moved boundary and then GREEN through only
the approved imports, visibility, re-exports, and path retargets.

## Existing owner map

The exact named owner suites are in `design.md` under "Characterization" and
are mapped as follows:

| Contract | Existing owners and added proof |
| --- | --- |
| R27-80B snapshot and transaction lifetime | Existing snapshot-race, linearization, frozen-read, and reader-pool owners; add the bounded reader-refusal release characterization. |
| R27-80C eligibility before caps | Existing search pretruncation, graph frontier, page, eligibility SQL, and query-plan owners; no duplicate test. |
| R27-80D ordering and fusion | Existing RRF, three-arm, reweight, tie-order, prefix-stability, and graph byte-determinism owners; no duplicate test. |
| R27-80E codec stability | Existing graph/evidence/frozen/pagination/trace fixtures and property tests; no generated oracle. |
| R27-80F view and filter contract | Existing hybrid, text-only, filtered, explained, existence-refusal, filter-grammar, and unification owners; add graph-arm and projected-text validity characterizations. |

## Planned RED/GREEN evidence

### Reader refusal releases every worker transaction

- Test: `tests/slice80_reader_refusal_release.rs`.
- Baseline: eight typed in-transaction refusals, with per-dispatch modulo
  progression proving one request reached each of the eight workers, followed
  by bounded erasure completion.
- RED mutant: omit rollback/release on the selected refusal path. The bounded
  erasure assertion must fail or time out for the intended WAL-retention reason.
- GREEN restoration: restore exact bytes and rerun the focused test.

### Graph-arm validity path

- Test: `tests/slice80_search_view_paths.rs`.
- Baseline: the graph neighbor is hidden by the default view, visible with
  `include_out_of_window`, visible at an in-window `valid_as_of`, and existence
  relaxation is refused with `InvalidArgument`.
- RED mutant: remove both neighbor-validity sites identified in `design.md`
  while keeping their parameters referenced. The default-view absence
  assertion must fail.
- GREEN restoration: restore exact bytes and rerun the focused test.

### Projected-text validity path

- Test: `tests/slice80_search_view_paths.rs`.
- Baseline: the projected-text hit has the same default exclusion,
  validity-relaxation inclusion, in-window inclusion, and typed
  existence-refusal behavior.
- RED mutant: remove the single projected-text validity site identified in
  `design.md`. The default-view absence assertion must fail.
- GREEN restoration: restore exact bytes and rerun the focused test.

## Structural batches

Not started. Each batch will record its commit, compile RED, import/visibility
GREEN, focused owner counts, source-scraper retargets, and surface evidence here.

## Review and final verification

Not started. The completed implementation will receive independent read-only
code review from a `gpt-5.6-sol` subagent at high reasoning. After its findings
are closed, a separate `gpt-5.6-terra` subagent will run the final test and
verification gates. Their durable records are `code-review.md` and
`review-verification.md`.
