---
title: 0.8.25 Slice 71 — TDD chronology
status: GREEN_71B_COMPLETE
date: 2026-09-09
---

# Slice 71 TDD chronology

## RED

Candidate base: `7ee9f47e`.

1. `bash scripts/tests/test_scale_ac013_fixn3.sh` exited 1 because the captured
   runner command lacked the required exact selector
   `--exact ac_013_vector_retrieval_latency`.
2. `PYTHONPATH=. .venv/bin/python -m pytest
   tests/experiments/test_release_0825_slice71.py -q` failed during collection
   with `ModuleNotFoundError: No module named
   'experiments.release_0825_slice71'`.
3. `cargo test -p fathomdb-engine --features test-hooks --test
   slice71_search_statement_trace --no-run` failed with Rust error E0432:
   unresolved import
   `fathomdb_engine::take_slice71_search_statement_trace_for_test`.

These failures independently bind exact runner isolation, the sealed manifest
contract, and the deterministic no-per-hit-query oracle. No production code or
fixture was changed before these RED observations.

After adding only the opt-in SQLite profile classifier, the reader-trace test
executed and failed with exactly 24 `post_filter_source_lookup` events for 24
returned FTS hits. This converts the initial missing-seam compile failure into
the design's deterministic behavioral RED.

## GREEN — search and runner

- The canonical runner now uses exact libtest selection and preserves its
  pipeline status/treatment-record checks.
- Node and edge eligibility SQL now carries active-barrier exclusion before
  ranking/hydration, and the redundant common per-hit post-filter is removed.
- The 24-hit trace observes zero post-filter lookups. Node FTS, edge FTS,
  edge-vector, and vector pre-truncation dependency tests pass.
- Focused `fathomdb-engine` clippy with `test-hooks` passes with warnings denied.

The strict manifest schemas, receipt schema, checked-in manifest, and seven
negative/positive validator tests are GREEN against source candidate
`5546585d`.

## RED/GREEN — strict receipt follow-up

The first contract GREEN covered the sealed manifest but omitted the design's
promised executable receipt rejection. Commit `666de9f2` adds the missing
tests as a second RED: collection fails because `validate_receipt` does not
exist. The follow-up GREEN adds exact top-level, cell, identity, metrics,
arm-order, ref, and blocked-classification validation without changing the
already-observed evidence or its thresholds.

## Exploratory observation

The first six AC-013 repetitions all fail p50. Candidate p99 range/median is
38.46%, exceeding the intended 20% environment-validity limit. Because the
manifest had not yet been committed, independent review correctly rejects
this as acceptance evidence. Product and harness GREEN changes remain
reviewable, but Slice 71 cannot close on the exploratory observation.

## Review correction RED/GREEN

Independent code review at `8e586e66` found that the first campaign was not
preregistered, the validator trusted rather than recomputed digests and
classification, and AC-013 cells omitted runtime/host-pressure observations.
The first campaign is retained as exploratory only.

Commit `a6a2ccf2` is the correction RED: 10 tests fail because receipt
validation has no manifest-byte argument and cannot prove digest or derived
classification. GREEN recomputes the exact manifest digest, derives the result
from retained metrics and environment observations, closes the runtime and
environment shapes, and provides a checked-in cell wrapper. The revised
manifest is committed before its first admissible measurement.

## Admissible acceptance stop

The post-correction campaign completes all six cells with valid captured host
pressure. B1/B2/B3 record p50 163/165/163 ms and p99 173/174/172 ms;
C1/C2/C3 record p50 200/201/201 ms and p99 210/210/294 ms. Candidate p99
range/median is 40.00%, so the executable classifier returns
`environment_invalid`. The sealed stop condition blocks the ingest campaign
and further treatments; this is a truthful blocked outcome, not COMPLETE.

## 71B write RED/GREEN

The retained current-code measurements are the performance RED: Scale-02 10k
median acknowledgement/total were 4,251.744/4,262.412 ms against historical
1,833.777/1,834.571 ms; projection-active AC-013 total was 4,929.785 ms against
historical 2,337.097 ms.

Deterministic RED commits then required one visibility advance per canonical
transaction, one per projection commit, statement preparation proportional to
SQL shape rather than rows, rollback on generation exhaustion, and custom main-
and TEMP-trigger fallback. GREEN preserves those tests while adding only the
measured hot-path corrections. Exact candidate `eda95b07` passes both 10k
limits and every small-workload conjunctive bound; see
[71b-performance-recovery.md](71b-performance-recovery.md).
