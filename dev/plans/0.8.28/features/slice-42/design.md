---
title: FathomDB 0.8.28 Slice 42 — design
status: PLANNED (R1 APPROVE-WITH-FIXES applied; Q-A1 pending; Q-B1, Q-B2 ruled)
target_release: 0.8.28
observed_on: 2026-10-16
---

# Slice 42 design

Unit A (§ 1) is a small test change that needs an owner ruling on the
exact lines. Unit B (§ 2) is the method for an investigation; its result
is `investigation.md`, written during the slice.

## 1. Unit A: a descriptor check that ignores other tests

### 1.1 Cause

`fd_count()` counts every entry in `/dev/fd`, so the check in
`slice135_persistent_sqlite_full_preserves_state_until_cap_lifts` also
counts descriptors that other tests hold at that moment: their engines'
database, WAL and shared-memory files, their raw `rusqlite` connections,
and the descriptors a child-process spawn opens. The count after depends
on what the other threads are doing, not on whether this test's engine
closed its files (requirements § 1.1).

### 1.2 Options

| Option | What it does | For | Against |
| --- | --- | --- | --- |
| **1. Serialise the binary** | A file-level `static` `Mutex` taken first by all five non-ignored tests, as `slice40_reader_page_cache.rs` and `slice40_projected_fts_stream.rs` do | Keeps the process-wide count, so it still sees any descriptor kind (pipes, sockets, event descriptors). Matches the header's `--test-threads=1` | Edits all five tests. The count is still process-wide, so a future test that forgets the lock breaks it again. The binary loses its parallelism |
| **2. Count only the test's own files** (recommended) | Read `/proc/self/fd`, follow each link, and count the links whose target is under the test's `TempDir` | Exact for the leak this test is about: the engine keeping its database files open. Independent of other threads by construction. Touches one test and adds one helper | Linux only. It returns `None` elsewhere, so the check becomes `None == None` on macOS (where `/dev/fd` counts today) as well as on Windows (where it is already `None`). No macOS or Windows job runs this binary; those runners only build wheels. Does not see a leaked pipe, socket or event descriptor, or a file outside the directory |
| **3. Run the scenario in a child process** | The test re-runs its own binary with `--exact` on an ignored victim test, as `slice135_crash_victim` does, and the child counts its own `/dev/fd` | Process-wide count, every descriptor kind, no cross-test effect | Most code: the scenario and its assertions move into a victim, and failures come back as an exit status and captured output. A larger edit to rule on |

### 1.3 Choice

Option 2. The assertion's message is "no persistent descriptor growth",
and the descriptors the engine owns are its database, WAL and
shared-memory files, which all live in the test directory. Counting
those answers the question directly and does not depend on scheduling.
Option 1 keeps a process-wide count that libtest itself, a child spawn or
an untidy neighbour can move, and it edits four tests that are not
broken. Option 3 is the fallback if the owner wants every descriptor
kind covered; it is the only option that keeps both properties.

Known limit of option 2: a leak of a descriptor that does not point under
the test directory passes. The engine opens no such descriptor today: no
socket, no pipe, no event descriptor. If one is added later, its own
tests have to cover it.

### 1.4 The edit (owner ruling Q-A1)

In `src/rust/crates/fathomdb-engine/tests/slice135_robustness_first.rs`
at the slice base `d10ad65d5`:

1. **Add** a helper after `fd_count()` (after line 56). It is new code,
   but it is in a test file, so it is part of the ruling:

   ```rust
   fn fd_count_under(dir: &Path) -> Option<usize> {
       if !cfg!(target_os = "linux") {
           return None;
       }
       let root = dir.canonicalize().expect("canonical test directory");
       let entries = std::fs::read_dir("/proc/self/fd").expect("read /proc/self/fd");
       Some(
           entries
               .filter_map(|entry| std::fs::read_link(entry.ok()?.path()).ok())
               .filter(|target| target.starts_with(&root))
               .count(),
       )
   }
   ```

   `read_link` fails for a descriptor closed between the listing and the
   read; such an entry is skipped, which is correct because it is no
   longer open. A file deleted while open reads as `<path> (deleted)`;
   its parent components still match.
2. **Line 366:** `let fd_before = fd_count();` becomes
   `let fd_before = fd_count_under(dir.path());`.
3. **Line 429:** `let fd_after = fd_count();` becomes
   `let fd_after = fd_count_under(dir.path());`.

Nothing else changes. Line 437, its message, `fd_count()` and its two
other callers stay. The `fd_before` and `fd_after` fields in the
`SLICE135_ROBUSTNESS` record now hold the directory-scoped counts; no
script reads them (a repository search finds them only in this file).
The expected values are 0 and 0 on Linux.

### 1.5 Showing it still catches a leak

Two uncommitted edits, each applied, run and reverted with
`git restore` (test-plan § 2.3):

- **Leaked file:** before line 429,
  `std::mem::forget(std::fs::File::open(&path).expect("leak"));`. The
  count after is 1; line 437 fails.
- **Leaked engine:** on line 416, `reopened.engine.close()` replaced by
  `std::mem::forget(reopened.engine);`. The engine's files stay open; line
  437 fails.

The old helper would also fail these, but not reliably pass the clean
case. That is the point of the change.

## 2. Unit B: investigation method

### 2.1 Question

Which route gives users of the published Rust artifacts the
per-connection page cache (or an equivalent latency result), at what cost
and risk? The artifacts are the `cargo install`ed CLI and the published
`fathomdb`, `fathomdb-sdk` and `fathomdb-engine` crates. The routes that
the owner prefers not to take (a forked `libsqlite3-sys` with a forked
`rusqlite`, and waiting for upstream) are assessed for completeness but
are not the target.

### 2.2 Current behaviour (evidence E1–E3)

- **E1, the installed CLI.** `cargo install fathomdb-cli --version
  '=0.8.27' --locked --root <scratch>` (Q-B2: allowed, scratch directory
  only; no publish, no tag). Record the `libsqlite3-sys` version in the
  install log, and check whether the binary carries
  `ENABLE_MEMORY_MANAGEMENT` in SQLite's compile-option table
  (`strings <bin> | grep -x ENABLE_MEMORY_MANAGEMENT`; the table drops
  the `SQLITE_` prefix). Repeat with
  `LIBSQLITE3_FLAGS=-USQLITE_ENABLE_MEMORY_MANAGEMENT`, and with the
  no-file form `cargo install … --config
  'env.LIBSQLITE3_FLAGS="-USQLITE_ENABLE_MEMORY_MANAGEMENT"'`; the string
  should be gone in both. The with/without pair is the control: once R2
  puts the literal into engine code, every binary contains it and
  `strings` stops telling the builds apart.
- **E2, a crate consumer.** A scratch crate outside the workspace, so the
  root `[patch]` does not apply, with a path dependency on this branch's
  `fathomdb-engine` (`features = ["test-hooks"]`). Record
  `cargo tree -i libsqlite3-sys` (it must show the registry source and
  version 0.38.2, not the vendored 0.38.1). The crate gets a fresh
  lockfile, so record the resolved `rusqlite` and other SQLite-related
  versions. Copy `tests/slice40_reader_page_cache.rs` into it with the
  same dev-dependencies and run AC40-95 to AC40-97: expected RED. No CUDA
  feature is enabled, so the `candle-kernels` patch does not matter.
- **E3, the override.** E2 again with
  `LIBSQLITE3_FLAGS=-USQLITE_ENABLE_MEMORY_MANAGEMENT`: expected GREEN.
  Then E2 again with the variable set in the scratch crate's
  `.cargo/config.toml` `[env]` table instead of the shell: this shows
  whether a config file reaches the dependency's build script
  (`build.rs` line 332 declares `rerun-if-env-changed=LIBSQLITE3_FLAGS`,
  and line 323 appends `-U…` after the fixed `-D` list, so it wins).

### 2.3 Routes

| Id | Route | Reaches | First reading |
| --- | --- | --- | --- |
| R1 | Prebuilt CLI binaries built in the workspace, attached to the GitHub release for each target, so `cargo binstall fathomdb-cli` fetches them (its default template needs no `[package.metadata.binstall]` if assets are named `{name}-{target}-v{version}.tgz`); install docs lead with them | CLI users | Per-connection cache and the Windows WAL backport, from the same vendored crate the wheels use. Today the GitHub release carries only the Python distributions. Release-pipeline work: a build job per target, including aarch64 Linux for Jetson and Tegra (the sealed CUDA CLI archive is a starting point), checksums, smoke. Does nothing for crate consumers |
| R2 | Report the mode at run time (todos ledger seq 309): read `sqlite3_compileoption_used("ENABLE_MEMORY_MANAGEMENT")` at open; report it in the open report or runtime configuration, warn once, and show it in `doctor` | Everyone, as information | Small, follows the Windows marker precedent. Fixes nothing. A public field needs interface docs and parity in three SDKs. The warning text points at the R3 one-line override |
| R3 | Document the persistent override: `LIBSQLITE3_FLAGS` in the consumer's `.cargo/config.toml` `[env]` or in `$CARGO_HOME/config.toml` (which `cargo install` also reads), or the one-line `cargo install fathomdb-cli --config 'env.LIBSQLITE3_FLAGS="-USQLITE_ENABLE_MEMORY_MANAGEMENT"'` | Users who read the docs | Free. Depends on E1 and E3 showing that `[env]` and `--config` reach the build script |
| R4 | A system SQLite: `LIBSQLITE3_SYS_USE_PKG_CONFIG=1` makes `libsqlite3-sys` link a system library even with `bundled` on (0.38.2 `build.rs` line 80). Or make `bundled` a default feature that consumers can turn off | Users who opt in | Still a user-set variable or a feature change in three crates. The system library may be older, may lack FTS5, may itself set the flag, drops the 3.53.2 version pin, and lacks the vendored amalgamation patches, including the Windows WAL backport. Evidence: `PRAGMA compile_options` of the system SQLite on Ubuntu 22.04, 24.04 and macOS |
| R5 | `PRAGMA mmap_size` on reader connections, so pages that are not in the WAL are read from the mapping | Everyone | Canonical. Hypothesis to measure: the pager looks a page up in the cache before using the mapping, so the group mutex stays, but fewer pages occupy the cache and the robbing matters less. Changes runtime configuration for every build (ADR-0.8.25-sqlite-runtime-configuration), with `SIGBUS` risk on I/O errors or truncation; could be applied only when E2's mode is detected |
| R6 | A larger `cache_size` per reader | Everyone | Slice 40 § 1.7: `-8000` delays the defect but does not remove it, and the ceiling grows four times. Measure how large a heavy read must be to trigger it at each size |
| R7 | Fewer readers | Everyone | Fewer connections share the group. Costs read concurrency. Measure with the reader count at 2, 4 and 8 |
| R8 | Release memory after a heavy read (`sqlite3_db_release_memory`), Slice 40 option D | Everyone | Rejected in Unit I: it empties every connection's cache in mode 2. Carried for completeness |
| R9 | A FathomDB page cache through `SQLITE_CONFIG_PCACHE2`, Slice 40 option E: a per-connection cache, a port of the separate-cache path of `pcache1` | Everyone | Canonical SQLite and stock crates. Estimated 400 to 600 lines of unsafe FFI on the read path. Process-wide: must be installed before `sqlite3_initialize`, so it also applies to the consumer's own SQLite use in the process and conflicts with any other `sqlite3_config` caller (the runtime latch of Slice 40 Unit J). Evidence: line count and a feasibility spike only if the owner asks |
| R10 | A build-time check or a cargo feature | — | Desk check: `fathomdb-engine`'s build script cannot see `libsqlite3-sys`'s compiler flags, and a feature cannot change them. `libsqlite3-sys` 0.38.2 exports `include` and `lib_dir` (`build.rs` lines 147 and 336), not the flags. A direct dependent's build script could link a probe against `DEP_SQLITE3_LIB_DIR` and call `sqlite3_compileoption_used`, but only when host and target match; assess, do not build. A `cargo:warning` from a registry dependency's build script is not shown to consumers (Cargo shows warnings only for path and workspace members), so a build-time warning cannot reach crate consumers; that leaves R2 |
| R11 | Wait for a `libsqlite3-sys` release that contains `f31f7324`, then bump (todos ledger seq 311) | Everyone | Owner prefers not to wait. Remains the exit of the vendored change. Record the release history only; no upstream request |
| R12 | A FathomDB-published `libsqlite3-sys` with a `rusqlite` fork | Everyone | Owner prefers not to. Needs its own `links` key; a consumer that also uses stock `rusqlite` would then link two SQLite images. Desk assessment only |

### 2.4 Decision criteria

Each route is scored on:

- **C1 Reach.** Which users get the per-connection result without an
  extra step: CLI users, crate consumers, both, or only users who opt in.
- **C2 Canonical.** The published crates keep the unmodified SQLite
  amalgamation and the stock crates.io `libsqlite3-sys` and `rusqlite`.
- **C3 Effect, measured.** In the Unit I timing report, the ratio of
  body-search p50 after one selective projected query to p50 before it;
  and the AC40-96 and AC40-97 counters. Mode 1 is the reference. A route
  passes C3 if its ratio is within 10% of mode 1's (owner ruling Q-B1,
  2026-10-16), measured as follows: at least 3 runs, median p50; mode 1
  and the route on the same host, binary and session; the AC40-96 and
  AC40-97 counters reported alongside (the counters are the deterministic
  signal, timing confirms them); on an idle host, with load average and
  `nproc` recorded. The timing report has never been run (Slice 40 status,
  I-W2/W4/W5), so this slice produces the mode-1 reference numbers too.
- **C4 Risk.** Unsafe code, new crash or correctness exposure on the read
  path, and effects on other SQLite users in the consumer's process.
- **C5 Memory.** The change in the page-cache ceiling and in resident
  memory, in particular for Tegra and Jetson targets.
- **C6 Cost.** Lines of code, release-pipeline jobs, maintenance, and how
  it is removed when R11 arrives.
- **C7 Public surface.** Interface docs, SDK parity, or new features
  needed.
- **C8 Time.** Whether it fits in 0.8.28 or needs a later release.

C1, C2 and C3 decide. C4 to C8 break ties and set the cost the owner is
asked to accept.

### 2.5 Measurements

- **Raw-SQLite probe** (scratch consumer, registry `libsqlite3-sys`):
  the Slice 40 § 1.7 probe shape, eight connections each warming its own
  set, then one heavy read; misses per connection on the next pass and
  the timing of that pass. Run for R5 (`mmap_size` 0 and 256 MiB), R6
  (`cache_size` -2000, -8000, -32000) and R7 (2, 4 and 8 connections),
  each in mode 2 and in mode 1 (`LIBSQLITE3_FLAGS`).
- **Engine timing report** (`reader_page_cache_timing_report`, release
  build, `FATHOMDB_TC5_INPUTS` = the `tc5-10000` pack) in the scratch
  consumer, mode 2 and mode 1, for the routes that need no engine change
  (R3, R4 if a usable system SQLite exists). R5 and R7 at engine level
  need an uncommitted engine edit; they run only if the raw probe shows a
  C3 pass, and the edit is reverted.
- Every number records host, commit, build profile, mode (from
  `sqlite3_compileoption_used`) and the command.

### 2.6 The deliverable

`investigation.md` in this directory, short:

1. current behaviour (E1 to E3);
2. the route table with C1 to C8 filled and evidence cited;
3. measurements;
4. one recommendation, which may combine routes (for example a CLI route
   plus a crate-consumer route);
5. numbered owner questions.

It is reviewed (R-INV) before it goes to the owner.

## 3. Open questions

- **Q-A1 (owner), pending.** Approve the edit in § 1.4: a new helper
  after line 56 and the changes to lines 366 and 429 of
  `slice135_robustness_first.rs`, nothing else. Option 2 is the
  recommendation (R1 agreed); the alternatives are option 1 (a lock in
  all five tests) or option 3 (a child process). R1 also noted that
  asserting `fd_after == Some(0)` would be more direct, but would widen
  the ruled edit; it is not proposed.
- **Q-B1 (owner), ruled 2026-10-16.** The C3 threshold is 10%, with the
  precision in § 2.4.
- **Q-B2 (owner), ruled 2026-10-16.** Allowed, scratch directory only:
  `cargo install fathomdb-cli --version =0.8.27 --locked --root
  <scratch>`; no publish, no tag, nothing written outside the scratch
  directory.
