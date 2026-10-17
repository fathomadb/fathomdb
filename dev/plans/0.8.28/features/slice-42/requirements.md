---
title: FathomDB 0.8.28 Slice 42 — requirements and acceptance criteria
status: PLANNED (for plan review)
target_release: 0.8.28
observed_on: 2026-10-16
---

# Slice 42 requirements and acceptance criteria

Slice 42 has two units. Unit A fixes a test that fails for a reason that
is not a product defect. Unit B is an investigation with a written
recommendation; it changes no product code. [design.md](design.md) gives
the options and the method, [test-plan.md](test-plan.md) the runs.

## 1. Evidence

### 1.1 Unit A

Ledger seq 312 (`TC-f9fa3b87-6f23-427b-b027-6889e52b3bef`), characterised
on `slice/0.8.28-40-fts-ranking-defects` at `d42c8a7c6`:

- The test is
  `slice135_persistent_sqlite_full_preserves_state_until_cap_lifts` in
  `src/rust/crates/fathomdb-engine/tests/slice135_robustness_first.rs`
  (lines 362–438). Line 437 asserts `fd_before == fd_after`. Both counts
  come from `fd_count()` (lines 54–56), which counts the entries of
  `/dev/fd`, that is every open descriptor in the process.
- The binary has four other tests that are not ignored. They open and
  close engines on other threads. One of them
  (`slice135_process_kill_before_and_after_acknowledged_write`) also
  spawns child processes; each spawn opens short-lived descriptors in the
  parent (`/dev/null` for stdout and stderr, and the spawn's own pipe).
- Results: in parallel runs of the whole binary 9 of 10 fail (7 before;
  12, 19 or 30 after). The test alone passes 5 of 5, and the binary with
  `--test-threads=1` passes 3 of 3 (4 and 4).
- The file header says to run it with `--test-threads=1`. The
  feature-complete gate does that (`scripts/lib/feature_complete.py`
  passes `--test-threads=1`), so the repository gate passes. An ad hoc
  `cargo test -p fathomdb-engine --features test-hooks` runs threads in
  parallel and fails, as it did in the Slice 40 Unit I checks.
- The target requires `test-hooks` (`fathomdb-engine/Cargo.toml`).
- No engine descriptor leak has been shown. The serial runs end at the
  count they started with.
- The other two tests that call `fd_count()` (lines 81/152 and 267/282)
  only print the counts; they assert nothing about them.

### 1.2 Unit B

Ledger seq 313 (`TC-4786ddae-2a6a-4da9-b618-3f659cfacb6e`); context seq
308, 309 and 311; Slice 40 design § 2h;
[ADR-0.8.28-sqlite-per-connection-page-cache](../../../../adr/ADR-0.8.28-sqlite-per-connection-page-cache.md).

- Slice 40 Unit I removed `-DSQLITE_ENABLE_MEMORY_MANAGEMENT` from the
  vendored `libsqlite3-sys` 0.38.1 build. The flag makes `pcache1Init`
  put every connection into one page-cache group, so one heavy read takes
  the other readers' pages and later reads run about 2.5× slower until
  the engine is reopened.
- The vendored crate reaches only builds made in this workspace, through
  the root `[patch.crates-io]`. That covers the Python wheels and the npm
  binaries.
- `cargo install fathomdb-cli` and every consumer of the published
  `fathomdb`, `fathomdb-sdk` or `fathomdb-engine` crates resolve the
  registry `libsqlite3-sys` 0.38.x. Its `build.rs` still passes the flag
  (0.38.2 line 162). They keep the shared cache. Results are correct;
  only latency is affected.
- A dependent crate cannot set `LIBSQLITE3_FLAGS` for `libsqlite3-sys`'s
  build script. The person running the build has to set it.
- Upstream removed the flag on rusqlite master (PR #1900, commit
  `f31f7324a82894be13e5de1665cb9af45dc5f869`, 2026-09-04). No crates.io
  release contains it; the latest are `libsqlite3-sys` 0.38.2 and
  `rusqlite` 0.40.2 (2026-08-08).
- The GitHub release made by `.github/workflows/release.yml`
  (`github-release` job) attaches only the Python distributions. No CLI
  binary is published there today. The release pipeline does build and
  seal a CUDA CLI archive for its CUDA rehearsal.
- The same `[patch]` gap applies to the vendored Windows WAL path
  backport. The published engine guards that case at run time with the
  `FATHOMDB_WIN_VERBATIM_SHM_SAFE` compile-option marker
  (`connection_runtime.rs`).
- Owner preference (2026-10-16): not keen on a non-canonical SQLite (a
  patched `libsqlite3-sys` with a `rusqlite` fork), and not keen on
  waiting for upstream.

## 2. Requirements

### Unit A

- **REQ42-A1.** The descriptor assertion in
  `slice135_persistent_sqlite_full_preserves_state_until_cap_lifts` gives
  the same result whatever the other tests in the binary do on other
  threads.
- **REQ42-A2.** The assertion still fails when the engine leaves a
  descriptor open on the test's database files.
- **REQ42-A3.** Only the lines the owner rules on (design § 1.4, Q-A1)
  change. No other test changes, no engine source changes, and the
  assertion message on line 437 stays.
- **REQ42-A4.** Runs that pass today still pass: the test alone, and the
  binary with `--test-threads=1`.

### Unit B

- **REQ42-B1.** Measure what each published artifact gets today: the
  `cargo install`ed CLI and a crate consumer, each built against the
  registry `libsqlite3-sys`.
- **REQ42-B2.** Assess every route in design § 2.3 against the criteria
  in design § 2.4. Back each assessment with the evidence named for that
  route, or say that the evidence is a desk reading.
- **REQ42-B3.** Deliver `investigation.md` in this directory: current
  behaviour, the route table, measurements, trade-offs, one
  recommendation, and the owner questions it needs ruled.
- **REQ42-B4.** Unit B changes no tracked source, `Cargo.toml`, lockfile,
  workflow or public doc. Probes live in the session scratch directory or
  in uncommitted edits that are reverted, and their commands and outputs
  are copied into `investigation.md`.

## 3. Non-goals

- Fixing the shared cache for published artifacts. That waits for an
  owner ruling on the investigation.
- Slice 41, and any change to Slice 40 Unit I.
- Serialising or changing the other tests in `slice135_robustness_first.rs`.
- Upstream issues, PRs or release requests (AGENTS.md; the slice plans'
  DoD).

## 4. Acceptance criteria

| AC | Checks | How |
| --- | --- | --- |
| AC42-01 | REQ42-A1 | The binary, built with `test-hooks`, run 30 times with default test threads: 0 failures |
| AC42-02 | REQ42-A1 (baseline) | The same 30-run loop at the slice base: failures recorded (expected near 9 in 10) |
| AC42-03 | REQ42-A2 | Uncommitted edit: a `File` opened on the test database and forgotten before the final count. The test fails on line 437 |
| AC42-04 | REQ42-A2 | Uncommitted edit: the reopened engine is forgotten instead of closed. The test fails on line 437 |
| AC42-05 | REQ42-A4 | The test alone 5 of 5; the binary with `--test-threads=1` 3 of 3 |
| AC42-06 | REQ42-A3 | `git diff` of the slice touches only the lines ruled in Q-A1 |
| AC42-07 | REQ42-B1 | `investigation.md` records, for the 0.8.27 CLI from `cargo install` and for a scratch consumer of this branch's engine, the resolved `libsqlite3-sys` source and whether `ENABLE_MEMORY_MANAGEMENT` is compiled in |
| AC42-08 | REQ42-B1 | The Unit I tests (AC40-95 to AC40-97), run in the scratch consumer: RED without `LIBSQLITE3_FLAGS`, GREEN with `-USQLITE_ENABLE_MEMORY_MANAGEMENT` |
| AC42-09 | REQ42-B2 | Each route row in `investigation.md` fills every criterion in design § 2.4 and cites its evidence |
| AC42-10 | REQ42-B2 | The measured routes (design § 2.5) have numbers from the Unit I timing report or the raw-SQLite probe, in both page-cache modes |
| AC42-11 | REQ42-B3 | `investigation.md` ends with one recommendation and numbered owner questions |
| AC42-12 | REQ42-B4 | `git diff --stat` for Unit B lists only files in this directory |
