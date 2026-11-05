---
title: FathomDB 0.8.28 Slice 42 — requirements and acceptance criteria
status: IN_PROGRESS (R-DES APPROVE-WITH-FIXES applied; implemented 2026-10-31; results in status.md)
target_release: 0.8.28
observed_on: 2026-10-31
---

# Slice 42 requirements and acceptance criteria

Slice 42 has two units. Unit A fixes a test that fails for a reason that
is not a product defect. Unit B moves FathomDB to `rusqlite` 0.41.1 and
`libsqlite3-sys` 0.39.1 and retires the vendored `libsqlite3-sys` 0.38.1.
[design.md](design.md) gives the changes, [test-plan.md](test-plan.md)
the runs.

The earlier Unit B (an investigation of routes to per-connection caches
for published crates, rulings Q-B1 and Q-B2, review R-INV, owner ruling
O2) is superseded by the owner's re-scope of 2026-10-31. Its AC42-07 to
AC42-12 are retired; the numbers below are new.

## 1. Evidence

### 1.1 Unit A

Todos ledger (`dev/todos-and-considerations-ledger.jsonl`, not the steward
ledger) seq 343 (`TC-f9fa3b87-6f23-427b-b027-6889e52b3bef`), characterised
on `slice/0.8.28-40-fts-ranking-defects` at `d42c8a7c6`. Line numbers below
were re-checked at `59079d5c5`; the file has not changed since `d10ad65d5`.

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
  parent.
- Results: in parallel runs of the whole binary 9 of 10 fail (7 before;
  12, 19 or 30 after). The test alone passes 5 of 5, and the binary with
  `--test-threads=1` passes 3 of 3.
- The feature-complete gate passes `--test-threads=1`, so the repository
  gate passes. No CI workflow runs this binary. An ad hoc
  `cargo test -p fathomdb-engine --features test-hooks` runs threads in
  parallel and fails.
- No engine descriptor leak has been shown.
- The other two tests that call `fd_count()` (lines 81/152 and 267/282)
  only print the counts.

### 1.2 Unit B

Todos ledger seq 344 (`TC-4786ddae-2a6a-4da9-b618-3f659cfacb6e`); owner
re-scope 2026-10-31; [ADR-0.8.28-sqlite-per-connection-page-cache](../../../../adr/ADR-0.8.28-sqlite-per-connection-page-cache.md).
Checked on 2026-10-31 against the registry sources:

- `libsqlite3-sys` 0.39.1 bundles SQLite 3.54.1
  (`SQLITE_SOURCE_ID` `2026-10-16 14:45:20 555e07ef…`). Its `build.rs`
  passes the same `-D` flags as the vendored 0.38.1 build script, which
  already omits `-DSQLITE_ENABLE_MEMORY_MANAGEMENT`. The registry 0.38.2
  still passes that flag. 0.39.0 (SQLite 3.54.0) omits it too.
- `winIsUNCPath` in the 3.54.0 and 3.54.1 amalgamations returns the
  test-locking value for a local `\\?\C:\` path. That is the 3.53.3
  correction (check-in `246f46614f`) that the vendored crate backports.
- `rusqlite` 0.41.1 depends on `libsqlite3-sys` `^0.39.0`. Its
  `rust-version` is 1.89; `libsqlite3-sys` 0.39.1 declares 1.88. The
  workspace declares 1.95 and pins toolchain 1.95.0.
- So both exit conditions in
  `third_party/libsqlite3-sys-0.38.1/FATHOMDB-PATCH.md` hold: a registry
  release contains rusqlite commit `f31f7324`, and the bundled SQLite
  contains the Windows WAL path fix.
- Today the vendored crate reaches only workspace builds (root
  `[patch.crates-io]`). `cargo install fathomdb-cli` and consumers of the
  published crates resolve the registry 0.38.x and keep the shared page
  cache (Slice 40 design § 2h).
- The published engine guards the Windows path case with the
  `FATHOMDB_WIN_VERBATIM_SHM_SAFE` compile-option marker, which only the
  vendored build defines (`connection_runtime.rs`, lines 129–174).
- A compile probe on 2026-10-31 (manifests at 0.41, root patch removed,
  `cargo check --workspace --all-targets`) found two breaking API changes
  in `rusqlite` 0.41. Design § 2.2 lists them.

## 2. Requirements

### Unit A

- **REQ42-A1.** The descriptor assertion in
  `slice135_persistent_sqlite_full_preserves_state_until_cap_lifts` gives
  the same result whatever the other tests in the binary do on other
  threads.
- **REQ42-A2.** The assertion still fails when the engine leaves a
  descriptor open on the test's database files.
- **REQ42-A3.** Only the lines the owner ruled on (design § 1.4, Q-A1,
  ruled 2026-10-16: a helper after line 56, lines 366 and 429)
  change. No other test changes, no engine source changes, and the
  assertion message on line 437 stays.
- **Accepted gap (Q-A1).** On macOS and Windows the helper returns
  `None`, so the check is vacuous there. No job runs this binary on
  those platforms.
- **REQ42-A4.** Runs that pass today still pass: the test alone, and the
  binary with `--test-threads=1`.

### Unit B

- **REQ42-B1. Registry SQLite.** Every workspace build links SQLite
  3.54.1 or later from the crates.io `libsqlite3-sys` 0.39.1, through
  `rusqlite` 0.41.1.
- **REQ42-B2. No vendored SQLite.** The repository carries no
  `libsqlite3-sys` source and no Cargo patch for it, in the root manifest
  or the standalone SDK driver. The pinned-override guard refuses such a
  patch.
- **REQ42-B3. Per-connection caches everywhere.** A build that resolves
  the published crates from outside this workspace gives each SQLite
  connection its own page cache, without `LIBSQLITE3_FLAGS`.
- **REQ42-B4. Windows guard by version.** The engine decides whether a
  local `\\?\C:\` path is safe from the linked SQLite version alone. The
  `FATHOMDB_WIN_VERBATIM_SHM_SAFE` marker is no longer read or defined.
  An SQLite older than 3.53.3 still refuses an unsimplifiable local
  verbatim path before any file is created.
- **REQ42-B5. No regressions.** The `rusqlite` 0.41 API changes are
  migrated without changing behaviour. sqlite-vec 0.1.9 works on SQLite
  3.54.1. The full gate passes, and the Windows WAL path tests pass on
  Windows.
- **REQ42-B6. MSRV unchanged.** The workspace `rust-version` stays 1.95.
- **REQ42-B7. Records.** The quality roadmap's patch line, the page-cache
  ADR and decision-index row 64, the Rust interface docs, the Rust
  install doc, the pinned-override design note, `dev/deps/rusqlite.md`,
  the release state and the CHANGELOG describe the new state. No live file
  cites the deleted crate or the marker.
- **REQ42-B8. Perf baseline.** The `AGENT_LONG` perf gates run once on
  the final tree. Their numbers are recorded as the 0.8.28 baseline on
  SQLite 3.54.1, for Slices 50 to 90 to measure against.

## 3. Non-goals

- Slice 41, and any change to Slice 40 Unit I.
- Serialising or changing the other tests in `slice135_robustness_first.rs`.
- A minimum SQLite version check on platforms other than Windows.
- Any performance claim beyond recording the baseline (REQ42-B8).
- Upstream issues, PRs or release requests.

## 4. Acceptance criteria

| AC | Checks | How |
| --- | --- | --- |
| AC42-01 | REQ42-A1 | The binary, built with `test-hooks`, run 30 times with default test threads: 0 failures |
| AC42-02 | REQ42-A1 (baseline) | The same 30-run loop at the slice base: failures recorded (expected near 9 in 10) |
| AC42-03 | REQ42-A2 | Uncommitted edit: a `File` opened on the test database and forgotten before the final count. The test fails on line 437 |
| AC42-04 | REQ42-A2 | Uncommitted edit: the reopened engine is forgotten instead of closed. The test fails on line 437 |
| AC42-05 | REQ42-A4 | The test alone 5 of 5; the binary with `--test-threads=1` 3 of 3 |
| AC42-06 | REQ42-A3 | The Unit A commit's diff touches only the lines ruled in Q-A1 |
| AC42-07 | REQ42-B1 | New test `bundled_sqlite_is_3_54_1_or_later` fails at its RED sha (reports 3053002) and passes after the bump |
| AC42-08 | REQ42-B1 | `Cargo.lock` lists `rusqlite` 0.41.1 and `libsqlite3-sys` 0.39.1, once each, with the crates.io source and checksum; `cargo tree -i libsqlite3-sys --workspace` shows one registry 0.39.1 |
| AC42-09 | REQ42-B2 | The two new shell cases (a root `libsqlite3-sys` patch is refused; a tree without it passes) and the SDK-driver pytest refusal fail at their RED sha and pass after; `scripts/check-pinned-override-rot.py` passes on the final tree |
| AC42-10 | REQ42-B2 | `git ls-files third_party` is empty; no `Cargo.toml` in the repository has a `libsqlite3-sys` key under `[patch]` |
| AC42-11 | REQ42-B3 | A scratch crate outside the workspace, with a path dependency on this branch's `fathomdb-engine` and no patch: `cargo tree -i libsqlite3-sys` shows registry 0.39.1, and AC40-95 to AC40-97 pass. The same crate at the slice base shows registry 0.38.2 and AC40-95 fails |
| AC42-12 | REQ42-B4 | New portable test of the version-only predicate fails at its RED sha and passes after; `git grep FATHOMDB_WIN_VERBATIM_SHM_SAFE -- src scripts` is empty |
| AC42-13 | REQ42-B4, B5 | On Windows (the local VM): the engine's Windows tests pass, including `local_canonical_path_does_not_strand_idle_wal_readers` with no version skip, `unpatched_sqlite_refuses_unsimplifiable_local_verbatim_paths` and `engine_open_refuses_trailing_dot_before_database_or_lock_creation`. Local disks only; a mapped network drive takes SQLite's shared lock handle by design (design § 2.1) |
| AC42-14 | REQ42-B5, B6 | On the final tree: `cargo clippy --workspace --all-targets -- -D warnings -A missing-docs`, `cargo check --workspace --all-targets`, and `./scripts/agent-verify.sh` pass on toolchain 1.95.0; the vector suites (sqlite-vec) are among the passing tests |
| AC42-15 | REQ42-B7 | `./scripts/agent-lint-md.sh` and `./scripts/agent-lint-docs.sh` pass; `git grep -n "libsqlite3-sys-0.38.1"` finds no live file (the historical-evidence paths in test-plan § 4.6 excepted) |
| AC42-16 | REQ42-B8 | `AGENT_LONG=1` perf-gate results on the final tree, with host, commit, SQLite version and load recorded in `status.md` |
