---
title: Slice 135 exact-candidate E01–E12 workload and test coverage overlay
status: INTERIM_DIAGNOSTIC
target_release: 0.8.27
---

# Exact-candidate E01–E12 coverage overlay

This result is historical on `3f29d649d`. The
[current-source refresh](../2026-10-08-e12-current-coverage-refresh/README.md)
repeats the overlay after later product repairs.

This diagnostic uses clean product source
`3f29d649d0213e595c0dab251a449d92fd625792` and separate
`rustc -C instrument-coverage -Z coverage-options=branch` workload and test
executions, both with `test-hooks,default-embedder`. The E01–E12 workload
completed all twelve semantic cells once. The measurement binary SHA-256 was
`f07a298eb7bbae1a0b6a4e4cafff3469fa7a8323a204ff14fb63b03cbacf37c9`.
The coverage build is diagnostic; it supplies no latency evidence or
production-traffic frequency estimate.

The accepted test profile contains 14 passing targeted integration-test
binaries (119 tests) and two additional rank-stream cases run in separate
passing processes. It excludes the failed full rank-stream target. In the
60 engine source files represented in both exports, the workload hit 8,132
source lines and 888 branch IDs. The accepted tests also hit 8,119 of those
lines and all 888 branch IDs. All workload-hit IDs mapped to the test build.
The 13 missed line IDs are `embedding.rs` 159–161 and 170, and `lib.rs`
738–746. The first group is the public `embed_text` dispatcher and error
mapping; the second is the test-only `query_text_col_for_test` path. This is
a selected-workload-versus-selected-tests overlap, not a global coverage
percentage, proof of assertion strength, or Pareto-path production coverage.
The [per-file overlay](overlay.json) preserves exact line and branch IDs.

The raw local archive is `/tmp/slice135-e12-current-coverage`.
`workload.lcov` SHA-256 is
`bb857f2cd8590437a529fe494f180e405e6c8a69203d4af99425bba150d63daa`;
`tests-accepted.lcov` SHA-256 is
`416504b309f4df0972a8045ffc0215457b1352cf9c53d670495cd49a8a11a83f`.
The tracked `overlay.json` SHA-256 is
`16063e63c41d54557eda36fc50f7554633507adc0e745404e948fa75b3421dbf`.
The parser is
[slice135_coverage_overlay.py](../../../../../../../scripts/slice135_coverage_overlay.py);
its focused test checks missed versus unmapped branch classification.

## Failing existing rank-stream gate

The full `slice20_fts_rank_stream` target failed on the exact candidate.
`edges_are_ineligible_and_stream_row_errors_fall_back_without_partial_output`
inserted a malformed FTS `write_cursor`, expected successful full-sort
fallback, and received `EngineError::Storage`. The other two cases in that
process then failed because the shared environment lock was poisoned; each
passes in a fresh isolated process. The first failure reproduced without
coverage instrumentation against the same clean product source. Failed-run
profiles were excluded from the accepted overlay, and both passing isolated
profiles were included.

The older rank-stream fallback oracle conflicted with the newer fail-closed
row-error contract. The follow-up changed the older test expectation to
`EngineError::Storage` for a malformed persisted FTS row and clarified the
contract in `dev/design/retrieval.md` and `dev/interfaces/rust-sdk.md`. The
separate successful fallback test with valid rows remains. The focused
`slice20_fts_rank_stream` target then passed all three cases with
`test-hooks,default-embedder`, as recorded in its
[stdout](rank-resolved.stdout) and [stderr](rank-resolved.stderr); the product
source did not change. The local
failed and uninstrumented outputs remain in `test-runs/` in the raw archive
as the RED evidence. The full repository gate has not been rerun, so this
focused pass is not a full-gate green claim.
