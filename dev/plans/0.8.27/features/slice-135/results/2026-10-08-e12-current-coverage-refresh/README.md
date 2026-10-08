---
title: Slice 135 current-source E01–E12 workload and test coverage refresh
status: INTERIM_DIAGNOSTIC_NOT_PHASE1_CHECKPOINT
target_release: 0.8.27
---

# Current-source E01–E12 coverage refresh — 2026-10-08

This diagnostic uses the clean product source
`8c2455b6ccf1d06d5bf87fc4278aecc631014da0` for both the workload and
selected engine tests. Rust 1.95.0 compiled both with
`-C instrument-coverage -Z coverage-options=branch` and the
`test-hooks,default-embedder` features. The workload completed all twelve
semantic cells once against the pinned expected checks. Its binary SHA-256 is
recorded in [workload-command.json](workload-command.json). This coverage
build is diagnostic; it is separate from the unprofiled paired latency run.

The [test manifest](test-runs/manifest.json) records 20 passing integration
test binaries, 128 executed tests and six ignored tests, including the new
edge-explanation and importance-lookup row-error regressions. The selected
test set repeats the prior overlay's routes and adds current defect paths.
Compiler profiles from those passing runs were merged separately from the
workload profile. LLVM exported both as LCOV; the
[per-file overlay](overlay.json) was recomputed byte-identically from the
copied LCOV archive. The
[negative control](negative-control.json) zeroed a test-hit workload branch
count in `search_api.rs` and the overlay reported it as a workload-only
branch.

Across 60 engine source files, the workload hit **8,133 lines and 888 branch
IDs**. The selected tests hit **8,120 of those lines and all 888 branches**;
all workload-hit IDs mapped to the test build. The 13 missed lines are
`embedding.rs` 159–161 and 170 (the public `embed_text` dispatcher and error
mapping) and `lib.rs` 738–746 (the test-only `query_text_col_for_test` helper).
The [overlay](overlay.json) preserves the exact IDs and per-file denominators.
These are selected-route overlaps, not global coverage, assertion-strength
proof or production-traffic frequency.

The raw 131 MiB profile, LCOV and log archive is retained locally in this
result directory pending the end-of-phase retention decision. SHA-256:
`workload.lcov` `738fc5f121d25fd8e51d440f3e8976ce619cc3f6cecb1b67c035d2678bbc16d1`;
`tests.lcov` `cbdc61e6d53bf75a70ec7b99a92c658231cb557e12bf565d9e28c2e5096baa12`;
[overlay.json](overlay.json)
`7bf94c94b2a8aada6bf8d51a811250cab10119b06a76687f9794bf827edfe512`.
The untracked raw files are local evidence, not published branch evidence.

The test build exited zero but emitted LLVM profile-write warnings from
instrumented build steps run in a read-only checkout. The separately
executed test binaries wrote profiles under `/tmp`; all 20 runs and both
LLVM exports exited zero without profile diagnostics. The build-step
warnings do not enter either accepted profile set.

This refresh covers the E01–E12 engine routes and selected regression tests.
The broader declared installed-SDK and S01–S03 operation mix, CPU/queue cost
ranking, missed-line disposition and rare severe paths remain open for the
full Phase 1 checkpoint.
