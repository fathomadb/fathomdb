---
title: FathomDB 0.8.28 Slice 42 — design
status: PLANNED (rewritten 2026-10-31 for the Unit B re-scope; R-DES APPROVE-WITH-FIXES applied)
target_release: 0.8.28
observed_on: 2026-10-31
---

# Slice 42 design

Unit A (§ 1) is a small test change the owner ruled on (Q-A1). Unit B
(§ 2) moves to `rusqlite` 0.41.1 and `libsqlite3-sys` 0.39.1 and deletes
the vendored `libsqlite3-sys` 0.38.1. The earlier Unit B, an
investigation with routes R1–R12 and evidence E1–E3, is superseded by the
owner's re-scope of 2026-10-31 and is removed from this file; git history
at `59079d5c5` keeps it.

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
| **1. Serialise the binary** | A file-level `static` `Mutex` taken first by all five non-ignored tests | Keeps the process-wide count, so it still sees any descriptor kind | Edits all five tests. A future test that forgets the lock breaks it again |
| **2. Count only the test's own files** (ruled) | Read `/proc/self/fd`, follow each link, and count the links whose target is under the test's `TempDir` | Exact for the leak this test is about. Independent of other threads by construction. Touches one test and adds one helper | Linux only: `None == None` on macOS and Windows. Does not see a leaked pipe, socket or event descriptor, or a file outside the directory |
| **3. Run the scenario in a child process** | The test re-runs its own binary on an ignored victim test that counts its own `/dev/fd` | Process-wide count, every descriptor kind, no cross-test effect | Most code |

### 1.3 Choice

Option 2. The assertion's message is "no persistent descriptor growth",
and the descriptors the engine owns are its database, WAL and
shared-memory files, which all live in the test directory. A leak of a
descriptor that does not point under the test directory passes; the
engine opens no such descriptor today.

### 1.4 The edit (owner ruling Q-A1, 2026-10-16: approved as written)

In `src/rust/crates/fathomdb-engine/tests/slice135_robustness_first.rs`.
The line numbers were re-checked at `59079d5c5` on 2026-10-31: the file
is unchanged since `d10ad65d5`, so every number below still holds
(`fd_count()` at 54–56, the test at 362–438).

1. **Add** a helper after `fd_count()` (after line 56):

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
   its parent components still match. `Path` is already imported
   (line 7).
2. **Line 366:** `let fd_before = fd_count();` becomes
   `let fd_before = fd_count_under(dir.path());`.
3. **Line 429:** `let fd_after = fd_count();` becomes
   `let fd_after = fd_count_under(dir.path());`.

Nothing else changes. Line 437, its message, `fd_count()` and its two
other callers stay. The `fd_before` and `fd_after` fields in the
`SLICE135_ROBUSTNESS` record now hold the directory-scoped counts; no
script reads them. The expected values are 0 and 0 on Linux.

### 1.5 Showing it still catches a leak

Two uncommitted edits, each applied, run and reverted (test-plan § 2.3):

- **Leaked file:** before line 429,
  `std::mem::forget(std::fs::File::open(&path).expect("leak"));`. The
  count after is 1; line 437 fails.
- **Leaked engine:** on line 416, `reopened.engine.close()` replaced by
  `std::mem::forget(reopened.engine);`. The engine's files stay open;
  line 437 fails.

### 1.6 TDD shape

The RED case is the existing test run in parallel at the base (9 of 10
fail, AC42-02); the Unit A commit body cites that loop. No new test is
added (Q-A1). It is one commit, independent of Unit B.

## 2. Unit B: `rusqlite` 0.41.1 and `libsqlite3-sys` 0.39.1

### 2.1 What the new crates give (checked 2026-10-31)

Read from the registry sources in `~/.cargo/registry`:

- `libsqlite3-sys` 0.39.1 bundles SQLite 3.54.1. The `-D` flags in its
  `build.rs` equal the vendored 0.38.1 build script's flags exactly
  (both omit `-DSQLITE_ENABLE_MEMORY_MANAGEMENT`); registry 0.38.2 has
  that one extra flag. So the workspace build changes the SQLite version
  and nothing else in the compile configuration.
- `winIsUNCPath` in 3.54.1 (and in 3.54.0, from `libsqlite3-sys` 0.39.0)
  returns `sqlite3_win_test_unc_locking` for a `\\?\` path followed by a
  drive letter, a colon and a separator, and 1 for every other `\\` path.
  That is the logic the
  vendored crate backports from 3.53.3 (check-in `246f46614f`).
- 3.54.x also has `winIsRemoteVolume` (3.54.1 `sqlite3.c` line 52543),
  OR'd with `winIsUNCPath` when `winOpenSharedMemory` picks the lock
  handle (line 52581). The vendored 3.53.2 has no such function. So a
  `\\?\X:\` path on a mapped network drive (`DRIVE_REMOTE`) now takes
  the shared lock handle, as a UNC path does. That is upstream's intended
  network-volume handling, not the local-path defect; AC42-13 covers
  local disks only.
- `rusqlite` 0.41.1 requires `libsqlite3-sys` `^0.39.0`, so any
  resolution of `rusqlite` 0.41 bundles 3.54.0 or later without the flag.
- MSRV: `rusqlite` 0.41.1 declares 1.89, `libsqlite3-sys` 0.39.1 declares
  1.88; the workspace declares 1.95 and pins 1.95.0. No change
  (REQ42-B6).
- The `rusqlite` feature table still has `bundled`, `fallible_uint`,
  `hooks` and `trace`. `hooks` now enables a new `wal` feature. Default
  features add `ffi-sqlite-wasm-rs`, a dependency only on
  `wasm32-unknown-unknown`.

### 2.2 `rusqlite` 0.41 API breakage (compile probe)

Probe, 2026-10-31, in a scratch export of `59079d5c5`: the three
manifests moved to `0.41`, the root `libsqlite3-sys` patch removed,
`cargo update -p rusqlite --precise 0.41.1`, then
`cargo check --keep-going --workspace --all-targets`, the engine with
`test-hooks,operator,migration-test-hooks,tc5-benchmark`, the engine
`--lib` with no features, and `fathomdb-schema` and `fathomdb-cli` alone.
The lockfile diff is two packages: `rusqlite` 0.40.1 → 0.41.1 and
`libsqlite3-sys` local 0.38.1 → registry 0.39.1. No other package moves.

Two breaking changes reach FathomDB code:

1. **`Connection::trace_v2`** now takes `&mut self`, a generic
   `Option<F>` with `F: FnMut(TraceEvent<'_>) + Send + 'static`, and
   returns `Result<()>` (it took `&self` and `Option<fn(TraceEvent)>` and
   returned nothing). Five calls, all behind `test-hooks`:
   - `reader_pool.rs` 696 (install) and 715 (clear), on the reader
     worker's `connection`, which is already a `mut` binding;
   - `evidence.rs` 2993 (install) and 2995 (clear), in the unit test
     `graph_nonce_generation_executes_no_sql_statement`.

   The clear calls pass a bare `None`, which no longer infers `F`
   (E0283). Migration: `None::<fn(rusqlite::trace::TraceEvent<'_>)>`;
   the `evidence.rs` test binds `let mut connection`; each call handles
   the `Result` with `.expect("…")` naming the install or clear. The
   reader-pool calls run only in `test-hooks` builds, which no shipped
   artifact enables, so a failure panics a test rather than a user's
   reader.
2. **`Connection::busy_handler`** (and `commit_hook`, `rollback_hook`,
   `update_hook`, `progress_handler`, `authorizer`) now take `&mut self`;
   `busy_handler` also takes a generic closure. The engine's existing
   hook calls already hold `&mut` and compile unchanged. Three test calls
   do not (E0596): `tc57_governed_write_race.rs` 471 and 526 (bindings
   `let (a, b)` on lines 466 and 523) and `tc90_transition_write_race.rs`
   843 (binding on line 839). Migration: make the one binding `mut`.
   The comment above line 471 ("takes a bare `fn` pointer, so the
   counter must be a static") is no longer true and is deleted; the
   static counter stays.

With those edits the probe compiled every target and
`cargo clippy --workspace --all-targets -- -D warnings -A missing-docs`
and the engine `test-hooks,operator` clippy were clean.

The test-file edits above are compile-forced migrations of call syntax.
They change no assertion; the reviewer should check that.

**Consumer-visible effect.** `fathomdb_schema::migrate(&rusqlite::Connection)`
is public, so its parameter type moves to `rusqlite` 0.41, and
`libsqlite3-sys` declares `links = "sqlite3"`, so a consumer cannot hold
`rusqlite` 0.40 and FathomDB 0.8.28 in one graph. The 0.8.27 move to
`rusqlite` 0.40 had the same effect; the CHANGELOG records it under
"Changed" in the same way.

### 2.3 Deleting the vendored crate

Delete `third_party/libsqlite3-sys-0.38.1/` (and with it
`FATHOMDB-PATCH.md`). `third_party/` is then empty and goes too. Every
live reference moves with it:

| File | Change |
| --- | --- |
| `Cargo.toml` (root) | Delete the `libsqlite3-sys` line under `[patch.crates-io]` and its two comment lines; the `[patch]` comment keeps the `candle-kernels` paragraph |
| `experiments/sdk_drivers/rust/Cargo.toml` and its `Cargo.lock` | Delete the duplicate `libsqlite3-sys` patch; refresh the driver lock to `rusqlite` 0.41.1 / `libsqlite3-sys` 0.39.1 |
| `src/rust/crates/{fathomdb-cli,fathomdb-schema,fathomdb-engine}/Cargo.toml` | `rusqlite` `"0.40"` → `"0.41"` (four lines; the engine has a dependency and a dev-dependency). The lockfile pins 0.41.1; `"0.41"` matches the file's style and every 0.41 release bundles a SQLite with the fix and without the flag (§ 2.1) |
| `scripts/check-pinned-override-rot.py` | Delete the `SQLITE_*` constants, the SQLite branch of `sdk_driver_duplicate_patch`, and the SQLite branches of `validate_cargo_pins` (the expected key, the missing-patch message, the lock, manifest, license and digest checks). A `libsqlite3-sys` patch then falls through to the existing "unsupported Cargo override/git source" failure |
| `scripts/pinned-override-rot.json` | Delete `cargo_sqlite_exception`; rewrite the `cargo` description to name two admitted mechanisms (the `candle-kernels` Git patch and the fork registry pins) |
| `scripts/tests/test_check_pinned_override_rot.sh` | B-RED-2 adds the two refusal cases of § 2.6. B-GREEN-2 deletes the obsolete SQLite fixtures and cases: `seed_sqlite_fixture` (lines 198–224) and its two callers, the lock stub, the drift, build and path cases (528–564), and the driver's SQLite patch line and `missing-sqlite` case (696–697, 738–740) |
| `tests/experiments/test_sdk_rust_patch_policy.py` | B-RED-2: the driver SQLite patch becomes an assertion of refusal (§ 2.6) |
| `dev/design/pinned-override-rot-guard.md` | Rewrite R4 (lines 59–75) to the two admitted mechanisms, the `candle-kernels` Git patch and the fork registry pins, mirroring the new `cargo` description in `pinned-override-rot.json`. The `cudarc` 0.19.7 clause there is stale since Slice 31 and goes too |
| `scripts/slice135_pilot.py` | Unchanged. Its `SOURCE_PATHS` still names `third_party`, but the S02-L campaign freezes the helper's SHA-256, and `REQUIRED_SOURCE_PATHS` (`SOURCE_PATHS[:3]`) does not include it, so the missing directory is harmless |
| `scripts/security/gitleaks-current.toml` and `check-gitleaks-current-config.py` | Delete the two "Vendored SQLite" allowlists in both, together (the checker requires the config to equal its expected list). They name only files that no longer exist |
| `.gitattributes` | Delete the `third_party/libsqlite3-sys-0.38.1/** -whitespace` entry and its comment |
| `.markdownlint-cli2.jsonc`, `scripts/agent-lint-md.sh` | Delete the two vendored README/Upgrade excludes |

Unchanged: `scripts/perf-experiments/collect-host-spec.sh`,
`run-slice71-ac013-cell.sh` and `run_slice71b.py` read the version from
`Cargo.lock` and keep working. Historical evidence (build logs and
snapshots under `dev/plans/0.8.27/`, `dev/plans/runs/`,
`experiments/configs/release-0825-slice71b-probe.Cargo.lock`, and the
Slice 31 and Slice 40 records) is left as written.

### 2.4 The Windows verbatim-path guard

Today (`connection_runtime.rs`):

- `sqlite_local_verbatim_path_requires_patched_vfs(path, sqlite_version, patched_vfs)`
  (lines 129–146, `cfg(windows)`) is true for a `\\?\C:\…` path that
  `sqlite_runtime_path` cannot simplify, when `sqlite_version < 3_053_003`
  and the marker is absent;
- `ensure_sqlite_local_verbatim_path_safe` (148–167) also refuses a
  trailing dot or space, then calls the predicate with
  `rusqlite::version_number()` and `sqlite_local_verbatim_vfs_is_patched()`;
- `sqlite_local_verbatim_vfs_is_patched` (169–174) reads the
  `FATHOMDB_WIN_VERBATIM_SHM_SAFE` compile option.

The marker exists only in the vendored amalgamation. After § 2.3 no
build can carry it, so the third argument is always false and the guard
is already a version check in effect. The rework makes that the code:

```rust
/// First SQLite release whose Windows VFS keeps a local `\\?\C:\` path off
/// the shared UNC lock handle (check-in 246f46614f). Older releases can
/// strand WAL read locks on such a path.
pub(crate) const SQLITE_LOCAL_VERBATIM_WAL_FIX: i32 = 3_053_003;

/// Whether a database path must be refused because the linked SQLite would
/// treat a local verbatim path as UNC. `simplifiable` is true when the
/// engine hands SQLite a simplified non-verbatim form instead.
pub(crate) fn local_verbatim_path_needs_newer_sqlite(
    path_text: &str,
    simplifiable: bool,
    sqlite_version: i32,
) -> bool
```

- The predicate is portable (no `cfg`): it reads only the text, a flag
  and a number, so its test runs on Linux and the RED is auditable
  there. The byte test of the `\\?\C:\` prefix moves into it unchanged.
- `ensure_sqlite_local_verbatim_path_safe` (still `cfg(windows)`)
  computes `simplifiable = sqlite_runtime_path(path) != path` and passes
  `rusqlite::version_number()`. The error message is unchanged.
- Deleted: `sqlite_local_verbatim_path_requires_patched_vfs`,
  `sqlite_local_verbatim_vfs_is_patched`, and every mention of the
  marker.
- The comment in `sqlite_runtime_path` (lines 115–116, "special and long
  paths retain their verbatim form for the bundled VFS correction") is
  reworded: they keep their verbatim form, which SQLite 3.53.3 and later
  handle as a local path.
- The threshold stays 3.53.3. It is the fix's release, and 3.54.0 and
  3.54.1 contain the fix (§ 2.1). Workspace and published builds link
  3.54.x, so the check refuses nothing there; it still protects a
  consumer who links an older system SQLite (for example through
  `LIBSQLITE3_SYS_USE_PKG_CONFIG`).
- `lib.rs` Windows tests: `unpatched_sqlite_refuses_unsimplifiable_local_verbatim_paths`
  (858–877) moves to the new predicate and loses its marker assertions
  (the `true` third argument). `local_canonical_path_does_not_strand_idle_wal_readers`
  (915) loses its early return: it now asserts
  `rusqlite::version_number() >= SQLITE_LOCAL_VERBATIM_WAL_FIX` and always
  runs, because a silent skip would hide a regression to an older
  SQLite. Both edits are forced by the deleted functions.

### 2.5 sqlite-vec 0.1.9 on SQLite 3.54.1

`sqlite-vec` 0.1.9 compiles `sqlite-vec.c` with `SQLITE_CORE` against its
own copy of `sqlite3.h` (3.45.3) and links to whichever SQLite the binary
carries; the engine registers it with `sqlite3_auto_extension`. That is
the arrangement on 3.53.2 today; it relies only on the stable C API. In
the probe, `cargo test -p fathomdb-schema -p fathomdb-engine --features
test-hooks,operator` ran 1,637 passing tests on 3.54.1, including the
vector suites (`corpus_vector`, `rebuild_vec0`, `slice40_vector_pool`,
`tc76_vec0_long_metadata_delete` and the rest) and AC40-95 to AC40-97.
Three tests failed, all parallel-run flakes that fail the same way at the
base:

- `slice135_persistent_sqlite_full_preserves_state_until_cap_lifts`
  (Unit A's defect);
- `evidence_search_races_are_snapshot_atomic_or_wholly_refused` and
  `evidence_linearizes_at_sidecar_and_resolver_return_seams` in
  `slice50_evidence.rs` (`EvidenceUnavailable`): 6 of 15 parallel runs
  of that binary fail at the base and 3 of 15 on the probe; 5 of 5
  serial runs pass at the base. Not in this slice's scope; recorded in
  status.md for the coordinator.

### 2.6 TDD: RED and GREEN commits

One RED commit per behaviour, each failing at its own sha for the stated
reason, with "RED evidence:" in its body; then its GREEN. Test-plan § 3
has the commands. Order:

| # | Commit | Fails at its sha because | GREEN |
| --- | --- | --- | --- |
| B-RED-1 | New engine test target `tests/slice42_sqlite_bump.rs` with `bundled_sqlite_is_3_54_1_or_later`: `rusqlite::version_number() >= 3_054_001`, with the version in the message | The linked SQLite is 3.53.2 (3053002) | B-GREEN-1: manifests, lockfile, removal of both `libsqlite3-sys` patch lines, the § 2.2 migration |
| B-RED-2 | Two new cases in `test_check_pinned_override_rot.sh`: (a) a root `libsqlite3-sys` path patch fails as "unsupported Cargo override/git source Cargo.toml:patch.crates-io.libsqlite3-sys"; (b) a tree without the SQLite patch passes (it replaces the "missing approved SQLite patch" expectation). `test_sdk_rust_patch_policy.py` asserts that the driver's `libsqlite3-sys` patch is refused | (a) the checker admits the patch (rc 0); (b) the checker requires it; the pytest assertion fails because the guard admits the driver patch | B-GREEN-2: the checker and JSON (§ 2.3), deletion of `third_party/`, the gitleaks, `.gitattributes` and markdown-lint entries, `pinned-override-rot-guard.md`, and deletion of the obsolete SQLite fixtures and cases in the shell test |
| B-RED-3 | Portable unit test `local_verbatim_path_needs_newer_sqlite_decides_by_version_alone` in `connection_runtime.rs`: true for an unsimplifiable `\\?\C:\` path at 3_053_002; false at 3_053_003 and 3_054_001; false when simplifiable; false for `\\server\share\…` and `\\?\UNC\…`; and `SQLITE_LOCAL_VERBATIM_WAL_FIX == 3_053_003` | The function and constant do not exist; the evidence quotes the E0425 lines | B-GREEN-3: § 2.4 |

The test script stops at its first failing case, so B-RED-2's evidence
quotes the script's failure for one case and the checker's output run
directly on the other case's fixture. It names exactly those two cases
and the pytest assertion.

Between B-GREEN-1 and B-GREEN-2 the live pinned-override check fails
("missing approved SQLite patch"): the bump removes the patch it
requires. `scripts/agent-lint.sh` (line 101) and `ci.yml` (line 1387)
both run that check. B-GREEN-1's body quotes the checker's failure line
verbatim, and status.md records that sha for bisectors. B-GREEN-2 closes
it. Moving the checker first does not avoid this: a checker that refuses
the patch fails on a tree that still has it.

The per-connection cache for published consumers (REQ42-B3) has no new
committed test. In the workspace, AC40-95 to AC40-97 already pin it. For
consumers the evidence is the scratch-consumer run at the base (RED) and
at the final tree (GREEN), AC42-11.

A docs-only commit carries § 2.7. Unit A's commit (§ 1.6) can go before
or after.

### 2.7 Records

- `dev/notes/fathomdb-quality-testing-roadmap.md` line 38: replace the
  "targeted Windows SQLite VFS patch" sentence with one saying the
  checkout uses the registry `libsqlite3-sys` 0.39.1 (SQLite 3.54.1),
  and that the Windows verbatim-path WAL tests stay as regression
  coverage for the upstream fix.
- `dev/adr/ADR-0.8.28-sqlite-per-connection-page-cache.md`: a dated
  "Exit (0.8.28 Slice 42)" section. The decision's planned exit happened:
  the pin reached a release without the flag, the vendored crate is gone,
  and published crates and `cargo install` get per-connection caches.
  The `blast_radius` front matter is updated. The decision itself does
  not change, so this is an addendum, not a successor ADR.
- `dev/interfaces/rust.md` (page-cache paragraph, lines 60–73) and
  `dev/interfaces/rust-sdk.md` (lines 465–478): every build, including
  `cargo install` and registry consumers, has per-connection caches from
  0.8.28; drop the `LIBSQLITE3_FLAGS` advice except for 0.8.27.
- `docs/install/rust.md` (lines 42–50): from 0.8.28 no flag is needed;
  one sentence keeps the override for installing 0.8.27.
- `CHANGELOG.md` 0.8.28: under "Changed", "Updated the SQLite stack to
  `rusqlite` 0.41 (SQLite 3.54.1)"; in the Slice 40 page-cache entry,
  replace the registry caveat with "Since Slice 42 this also holds for
  `cargo install` and the published Rust crates".
- `dev/adr/ADR-0.6.0-decision-index.md` row 64 (line 168): replace
  "published Rust crates documented (`LIBSQLITE3_FLAGS`), not mitigated"
  with "published Rust crates get it from `libsqlite3-sys` 0.39"; status
  "accepted; exit completed 0.8.28 Slice 42".
- `dev/deps/rusqlite.md`: the "Version pin" line becomes `0.41.1`, features
  `bundled, fallible_uint` (plus `hooks` and `trace` under the engine's
  `test-hooks`).
- `dev/plans/release-state-0.8.28.json` (line 16, the Slice 42 evidence)
  and `dev/plans/plan-0.8.28.md` (line 184): "both exit conditions in
  `third_party/libsqlite3-sys-0.38.1/FATHOMDB-PATCH.md`" becomes "the exit
  conditions in the retired patch note (git history at `59079d5c5`)".
  After the JSON edit, `./scripts/check-release-state-views.sh --write`,
  and the regenerated views go in the same commit.
- `dev/plans/plan-0.8.28.md` § Slice 42 is kept in step with this file.

### 2.8 Verification beyond the unit tests

Test-plan § 4 has the runs: the full gate; the scratch consumer
(AC42-11); the Windows VM (AC42-13); macOS through the CI matrix in
final verification; and the `AGENT_LONG` perf baseline (AC42-16).

## 3. Open questions

- **Q-A1 (owner), ruled 2026-10-16.** Option 2 exactly as § 1.4. The
  gap on macOS and Windows, where the check becomes `None == None`, is
  accepted.
- **Q-B1, Q-B2, R-INV, O2.** Superseded with the old Unit B on
  2026-10-31.
- No new owner question. The choices in this design (manifest
  requirement `"0.41"`, keeping 3.53.3 as the guard threshold, a portable
  predicate, an ADR addendum rather than a successor) are within the
  re-scope and go to the design review.
