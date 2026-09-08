---
title: 0.8.25 Slice 71 — TDD chronology
status: RED
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
