---
title: 0.8.25 Slice 71 — TDD chronology
status: GREEN_BLOCKED_AT_ACCEPTANCE
date: 2026-09-08
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

## Acceptance stop

The six sealed AC-013 repetitions all fail p50. Candidate p99 range/median is
38.46%, exceeding the 20% environment-validity limit. The receipt therefore
records `environment_invalid`; the approved stop condition prevents the ingest
campaign and any unregistered second optimization treatment. Product and
harness GREEN changes remain reviewable, but Slice 71 cannot close COMPLETE.
