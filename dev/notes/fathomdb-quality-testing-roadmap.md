---
title: FathomDB quality testing roadmap — SQLite methods fitted to the product
status: RELEASE_PLANNING_REFERENCE
source_date: 2026-10-06
---

# FathomDB quality testing roadmap

The source is [How SQLite Is Tested](https://www.sqlite.org/testing.html),
collected locally at
`/tmp/fathomdb-slice135-sqlite-testing-2026-10-06.html` on 2026-10-06
(SHA-256 `d1a8b5c43a42e573f7165483fdf02b91c605076b869a823d924241fb308feb97`).
The locally collected page informed a read-only `gpt-6-astra` medium review of
the repository. This note adapts its methods; it is not a commitment to copy
SQLite's entire test program or its historical test-count claims.

## FathomDB's change curve

| Git window at `d3cd5600a` | Reachable commits | Commits touching `src/` | Gross `src/` additions/deletions | Unique `src/` files |
| --- | ---: | ---: | ---: | ---: |
| Since 2026-09-06 | 1,527 | 529 | +111,402 / -74,674 | 318 |
| Since 2026-09-29 | 464 | 173 | +38,924 / -28,619 | 159 |

These are read-only `git log --since=...` counts of reachable history, not
mainline-only work. Refactors, moves and repeated edits inflate gross churn.
The `v0.8.26` to `d3cd5600a` source diff touches 229 files, adding 69,269 and
deleting 53,701 lines. The history starts in March 2026. These observations
support the inference that FathomDB is a rapidly changing beta data plane,
not a settled storage engine. The actual engineer and compute budget was not
measured. Bind tests to stable public behavior and persistence invariants so
refactoring does not require rewriting a large internal-snapshot ecosystem.

FathomDB owns lifecycle/dependency closure, readiness, governed writes,
retrieval eligibility, evidence, erasure, provider transitions and three SDK
boundaries. SQLite's transaction atomicity does not establish that those
composed operations recover correctly or report truthful completion. A
structurally valid SQLite file can still contain the wrong FathomDB state.
This checkout also carries a [targeted Windows SQLite VFS patch](../../third_party/libsqlite3-sys-0.38.1/FATHOMDB-PATCH.md),
so its exact downstream behavior deserves regression coverage.

## What to adopt from SQLite

| SQLite practice | FathomDB adaptation | Limit |
| --- | --- | --- |
| [Anomaly testing](https://www.sqlite.org/testing.html#anomaly_testing) | Inject first and persistent failures at selected FathomDB-owned commit, queue, provider and erasure boundaries. Reopen the real database and assert canonical state, projection status, queued work, typed outcome and cleanup. | Process killing checks process-crash recovery, not torn/reordered power-loss writes. Declare which failure model was exercised. |
| [Resource-leak checks](https://www.sqlite.org/testing.html#automatic_resource_leak_detection) | Measure open handles, worker/queue shutdown and temporary/WAL artifacts after cancellation, failure and close. | Do not reduce robustness to error return or `PRAGMA integrity_check` alone. |
| [Branch and mutation testing](https://www.sqlite.org/testing.html#mutation_testing) | Map workload-hot branches to tests and inject a few plausible wrong-condition, wrong-order or missing-cleanup defects; retain cases that kill them. | Coverage is a meta-test of tests. Optimization-only mutants may preserve output while harming performance; evaluate those with latency evidence. |
| [Boundary-value tests](https://www.sqlite.org/testing.html#forcing_coverage_of_boundary_values_and_boolean_vector_tests) | Target validity edges, cursor limits, time windows, filter combinations and transaction states with property/state-machine cases. | Do not add test-only macros throughout a moving codebase to chase a global percentage. |
| [Disabled-optimization tests](https://www.sqlite.org/testing.html#disabled_optimization_tests) | Where an optimized FathomDB route has a trustworthy reference route, compare selected outputs and eligibility on the same state. | Do not claim an independent oracle if both paths share the same flawed predicate. |

SQLite's [coverage discussion](https://www.sqlite.org/testing.html#experience_with_full_test_coverage)
explains the value of exhaustive coverage for its mature, long-lived engine.
Its [fuzzing/MC/DC tension](https://www.sqlite.org/testing.html#tension_between_fuzz_testing_and_100_mc_dc_testing)
also shows that a percentage can conflict with defensive testing. For this
release, measure line/branch coverage of the important FathomDB paths and
explain gaps. Do not create a whole-workspace 100% MC/DC requirement, all-
allocation failure sweep, independent four-harness program or general SQLite
SQL/B-tree fuzzer. SQLite already owns and tests those internals; FathomDB
should concentrate on its own composition. Rust OOM may terminate the process,
so specify bounded pressure and process/reopen outcomes before promising typed
recovery for every allocation failure.

## Release-line proposal

| When | Testing investment | Scope control |
| --- | --- | --- |
| Slice 135 | Complete the first four measurement areas and publish the Phase 1 checkpoint before the dedicated correct-results campaign. Use benchmark corpora as workloads where useful. Add replayable seed/fault-point receipts, selected first/persistent failure cases, exact post-state oracles, targeted branch/mutation checks and one regression per confirmed defect. | Use existing harnesses and real databases. Results are diagnostic except accepted release gates. No exhaustive new test framework. |
| Remainder of 0.8.x | Consolidate reusable fault controls and workload runners. Retain minimized regression seeds. Add bounded parser/filter/protocol and FFI fuzz smoke cases, and track runtime, flakiness, maintenance effort and actionable findings. Verify dependency upgrades and packaging on supported platforms. | Keep routine checks small and behavior-oriented; avoid release-specific harness proliferation. |
| 0.9.x | Broaden stateful write/supersede/erase/project/reopen sequences, compound failures during recovery and selected VFS fault injection only where existing seams cannot represent the fault. Add targeted sanitizer/resource checks for unsafe/FFI/extensions and qualify the declared platform/scale envelope. | Budget expensive campaigns separately and replay only high-value minimized cases in ordinary checks. |
| 1.x.x | Maintain stable public-contract and compatibility fixtures; systematically cover critical persistence and erasure branches, selected OOM/I/O sweeps, and cross-platform/configuration release checks. Document supported failure and crash models. | A stable contract justifies a larger durable regression suite, not an automatic 100% MC/DC target. |
| 2.x.x | Requalify new ANN/index/backend seams: fidelity against independent exact neighbors, eligibility, rebuild/cutover/crash recovery, concurrent mutation and scale/resource envelopes; compare backends when their semantics match. | Expand engine-level proofs only for algorithms FathomDB actually owns. |

The release-line rows are recommendations, not changes to the existing
release ladder. Promote them only when the corresponding architecture and
support contract exist. Track test cost and defect yield so assurance grows
without making FathomDB resistant to necessary change.

## Release-planning use

Before authoring each new release plan, read this analysis and record in that
plan which recommendations apply, which are deferred and why, and what new
evidence or changed architecture revises the assessment. Update this document
when its change-curve evidence or recommendations become stale. Consultation
does not make every recommendation an automatic release gate.
