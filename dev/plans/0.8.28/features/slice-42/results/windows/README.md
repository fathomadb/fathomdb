# AC42-13 — Windows verbatim-path tests on the local VM

Host: libvirt VM `gh-runner-wonl-win11` (Windows 11, x86_64 MSVC), started for
this run on 2026-10-31 and shut down afterwards. Toolchain 1.95.0 (the
repository's `rust-toolchain.toml`). Each tree was transferred as a
`git archive` tarball under an owned `C:\temp\slice42-*` path, its SHA-256
checked on both sides, and the directory removed afterwards.

| Tree | Tarball SHA-256 |
| --- | --- |
| final `2af70d3bc` | `3215abc61866fa16d65ca0fe634c0ada0f3c531f99ce9d71fde5940828d5de18` |
| base `59079d5c5` (comparison only) | `f44404f6bb7091fc4b5240dbe5c7b6205d4830545d93c12ac2a9c77c9ca86444` |

## AC42-13 result (final tree): pass

`cargo test -p fathomdb-engine --lib -- <names> --nocapture`, 6 passed:

- `connection_runtime::tests::local_verbatim_path_needs_newer_sqlite_decides_by_version_alone`
  (at `2af70d3bc`; `ffae52b18` later moved it unchanged into the root `tests`
  module)
- `tests::sqlite_runtime_path_simplifies_only_safe_local_drive_paths`
- `tests::unpatched_sqlite_refuses_unsimplifiable_local_verbatim_paths`
- `tests::windows_trailing_component_cannot_change_database_identity`
- `tests::engine_open_refuses_trailing_dot_before_database_or_lock_creation`
- `tests::local_canonical_path_does_not_strand_idle_wal_readers`, which now
  always runs (no version skip) on SQLite 3.54.1

Also passing: `cargo test -p fathomdb-engine --test slice42_sqlite_bump` (SQLite
3.54.1 on Windows) and `cargo test -p fathomdb-engine --features
operator,test-hooks --test erasure_completeness
erasure_busy_cross_process_windows_yields_typed_diagnostic -- --exact`.

## Pre-existing Windows failures (not caused by Slice 42)

A wider `cargo test -p fathomdb-engine --features operator,test-hooks --lib`
on the final tree found three tests that fail or hang. Each was rerun alone
with `--exact` on both trees; all three behave the same at the base, with the
vendored SQLite 3.53.2:

| Test | Final `2af70d3bc` | Base `59079d5c5` |
| --- | --- | --- |
| `slice90_close_review_tests::failed_startup_after_timed_out_probe_preserves_error_and_cleans_sqlite_profiles` | FAIL at line 58, "injected post-probe failure was ignored" | same |
| `slice90_post_probe_real_error_tests::real_post_probe_visibility_error_uses_shared_unwind_and_retains_provider_only` | FAIL at line 69 | same |
| `tests::current_opener_holds_lock_before_admission_classification` | hangs (killed after 900 s) | same |

All three install a test hook keyed by the database path before
`Engine::open` and expect it to fire; the cause was not investigated in this
slice. They are reported to the coordinator for placement.
