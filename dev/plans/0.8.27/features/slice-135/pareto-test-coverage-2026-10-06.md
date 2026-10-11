---
title: Slice 135 Pareto-path test coverage sample — search and edge FTS
status: PROVISIONAL_TEST_COVERAGE_HOST_CONTENTION
target_release: 0.8.27
source_sha: 8cbd330c8f83f35ef46d538ea23811c08af8066b
---

# Pareto-path test coverage sample — 2026-10-06

This is the **test-side half** of the first Pareto-path coverage comparison.
Four selected existing real-database tests ran on the exact post-Slice-132
candidate. The workload-side coverage and measured cost ranking must be
overlaid before any path can be called a Pareto path. Test execution counts
below are not production frequency estimates.

This coverage run overlapped a baseline noise pilot that was invalidated by
swap activity. All four tests passed, and the coverage files record the paths
they exercised, but the host was not quiet. Treat timing and execution counts
as unusable for performance comparison; rerun this selection on a quiet host
before using it as release-grade test/workload overlap evidence.

The measurement checkout was `172c724cd921726f7b2679ee51fdb703be823db4`.
It contains documentation and evidence commits after the source commit above.
`search.rs` SHA-256 was
`4122b0f7df03b96763d5fed020b2a17a963fad8b04f364c661cb77c2d0ae8e5f`;
`Cargo.lock` SHA-256 was
`9e9d7b5e82184a0bddfbe96de28fa0615ef4443fc84639b271c1cff29600ccfe`.
The primary Slice 135 checkout at `61a751079` had the same `search.rs`,
selected test sources, and lockfile when this run began. Each binary and
the source-file hashes are in the [coverage metadata](evidence/pareto-test-coverage/slice135-pareto-tests-coverage.json).

## Selection and result

| Test target | Exact selection | Result | Why selected |
| --- | --- | --- | --- |
| `slice23_text_limit_prefix_stability` | Both tests in the target | 2 passed | Direct text-only ordering and duplicate edge-body handling at two limits |
| `tc33_fix2_edge_validity_on_search` | `tc33_fix2_expired_edge_is_excluded_from_search_valid_edges_survive --exact` | 1 passed | Ordinary search must exclude an expired edge and retain two valid edges |
| `slice15b_search_validity` | `text_only_search_also_hides_out_of_window_nodes --exact` | 1 passed, 7 filtered | Text-only validity window rejection |

The [raw outputs](evidence/pareto-test-coverage/slice135-pareto-s23.stdout),
[edge-validity output](evidence/pareto-test-coverage/slice135-pareto-tc33.stdout),
and [text-validity output](evidence/pareto-test-coverage/slice135-pareto-s15b.stdout)
name every executed test. All four passed. The TC33 test took about 10 seconds
because its fixture polls search until projected edge results are visible;
that duration is not a system-latency sample.

With `operator` enabled and Rust 1.95.0 diagnostic branch instrumentation,
the combined test profiles covered `search.rs` as follows:

| Source scope | Covered lines / executable lines | Covered branches / instrumented branches |
| --- | ---: | ---: |
| Whole `search.rs` | 371 / 1,500 (24.73%) | 39 / 236 (16.53%) |
| Edge FTS decode, lines 2050–2085 | 23 / 25 | 2 / 4 |
| Node FTS selection, lines 1830–2010 | 39 / 96 | 9 / 14 |
| Edge explanation, lines 2100–2120 | 0 / 15 | 0 / 6 |

The [raw LCOV export](evidence/pareto-test-coverage/slice135-pareto-tests-search.lcov)
and [LLVM text report](evidence/pareto-test-coverage/slice135-pareto-tests-search-report.txt)
record `LF:1500`, `LH:371`, `BRF:236`, and `BRH:39`. The
[normalized LCOV](evidence/pareto-test-coverage/slice135-pareto-tests-search.normalized.lcov)
changes only the `SF:` checkout prefix to the repository-relative
`src/rust/crates/fathomdb-engine/src/search.rs`. Its `DA:` and `BRDA:` counts
are byte-for-byte otherwise preserved for a later workload overlay.

The edge FTS row mapper was heavily exercised by these tests, yet both
recorded `if let Ok` error-side branches at lines 2061–2062 have zero hits.
The `rows.flatten()` path at line 2079 ran 969 times. Most of that count comes
from fixture polling and must not be used as a Pareto frequency. The
[separate fault probe](logic-first-result-2026-10-06.md) demonstrates that
an invalid UTF-8 edge FTS row is silently dropped on this path; these selected
passing tests did not detect that error condition. The untested edge
explanation region may be important, but its workload frequency is not yet
known.

## Reproduction and overlay rule

The test binaries were built together with the same feature set and flags:

```sh
RUSTC_BOOTSTRAP=1 CARGO_TARGET_DIR=/tmp/slice135-logic-branchcov \
  RUSTFLAGS='-C instrument-coverage -Z coverage-options=branch' \
  LLVM_PROFILE_FILE='/tmp/slice135-pareto-tests-build-%p-%m.profraw' \
  cargo test --locked --offline -p fathomdb-engine --features operator \
  --test slice23_text_limit_prefix_stability \
  --test tc33_fix2_edge_validity_on_search \
  --test slice15b_search_validity --no-run --message-format=json
```

The three binaries were run directly with the selections in the table, each
with a distinct `LLVM_PROFILE_FILE` prefix. Their
[compressed raw profiles](evidence/pareto-test-coverage/slice135-pareto-s23.profraw.gz),
[TC33 profile](evidence/pareto-test-coverage/slice135-pareto-tc33.profraw.gz),
and [Slice 15b profile](evidence/pareto-test-coverage/slice135-pareto-s15b.profraw.gz)
were merged with `llvm-profdata merge -sparse`; `llvm-cov report` and
`llvm-cov export --format=lcov` read all three test binaries and the merged
profile. Tool versions were `rustc`/Cargo 1.95.0 and matching LLVM
22.1.2-rust-1.95.0-stable. `-Z coverage-options=branch` requires
`RUSTC_BOOTSTRAP=1`; it is an unstable diagnostic setting, not a release build
or gate. The [SHA-256 manifest](evidence/pareto-test-coverage/SHA256SUMS)
binds the retained bytes.

For the workload overlay, require an identical source-file hash, lockfile,
feature set, Rust version, and instrumentation flags, then compare normalized
`SF`, `DA`, and `BRDA` records. Rank workload paths by measured total cost
first; calculate test overlap on the observed high-cost lines and branches.
Do not infer usage from these tests or combine incompatible coverage maps.

A plausible targeted code mutant is to omit the edge validity predicate from
the edge FTS query. The selected TC33 test should reject that mutant because
the expired edge would surface while valid controls remain present. This
mutant was **proposed, not run**. For the confirmed swallowed-row defect,
a durable regression test should first assert `Storage` for the corrupted FTS
row, then kill any mutant that restores `rows.flatten()` after the product fix.

This bounded selection is neither the full suite nor a robustness or latency
gate. It does not measure user workload frequencies, total-cost rank, other
source files, Python/TypeScript wrapper coverage, or generated answer quality.
