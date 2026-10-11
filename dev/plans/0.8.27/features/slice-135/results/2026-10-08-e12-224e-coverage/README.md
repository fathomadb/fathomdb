---
title: Slice 135 exact-candidate E01–E12 coverage overlay
status: INTERIM_DIAGNOSTIC_NOT_PHASE1_CHECKPOINT
target_release: 0.8.27
---

# Exact-candidate E01–E12 coverage overlay — 2026-10-08

The separate instrumented workload and test runs used clean product source
`224e44c593c13d86ece648adabe445723db04070`, Rust 1.95.0, and
`-C instrument-coverage -Z coverage-options=branch`. The workload completed
all twelve E01–E12 paths with one diagnostic observation per path. The selected
test set ran 20 binaries: 128 tests passed and six were ignored. Two further
test binaries ran 23 passing tests to classify the selected set's line gaps.
The [audit](audit.json) independently recomputes the overlay from separate
workload, selected-test and supplemental-test LCOV exports, checks binary
hashes and test outcomes, and detects a deliberately zeroed test branch.

| Workload-hit engine source | Selected tests | Supplemental disposition |
| --- | ---: | --- |
| Lines | 8,121 / 8,134 | All 13 remaining selected-set lines hit |
| Branch IDs | 888 / 888 | No selected-set branch gap |

The four selected-set misses in `embedding.rs` lines 159–161 and 170 are the
public `embed_text` dispatcher and error mapping. The nine in `lib.rs` lines
738–746 are the SQL text-column test seam. They were exercised by the
passing `slice90_foreground_dispatch` and `slice15e_prekn_filterable` test
binaries respectively. Search included in-crate `#[cfg(test)]` modules;
there is no in-crate caller for either path. These 13 lines were absent from
the **selected** tests, not from the repository's tests.

The [selected overlay](overlay.json) retains file-level line and branch IDs;
the audit records the supplemental gap disposition. The raw profiles, LCOV exports, logs
and instrumented build are local and untracked pending the end-of-phase
retention decision. Their hashes are in the audit; the tracked summaries do
not imply that raw evidence is published. The coverage runs are diagnostic
and must not be used as latency samples. Selected-route overlap does not
measure global coverage, production traffic, or assertion strength. The
broader mixed-operation, CPU/queue and rare-path analyses remain open.
