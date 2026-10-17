# FathomDB changes to the vendored libsqlite3-sys 0.38.1

## Bundled SQLite Windows WAL path correction

This is the published `libsqlite3-sys` 0.38.1 crate, copied from the Cargo
registry package with its optional SQLCipher files intact. The changes to
`sqlite3/sqlite3.c` are the `winIsUNCPath` function and adjacent comment, plus
a Windows-only `FATHOMDB_WIN_VERBATIM_SHM_SAFE` compile-option marker. It
preserves bundled SQLite version 3.53.2 and its normal linkage on every
platform. The marker lets the published engine fail closed on unsimplifiable
local verbatim paths when downstream Cargo resolves the unpatched registry
crate; it does not claim a different SQLite version.

SQLite 3.53.2 treats the Win32 long DOS-device form `\\?\C:\...` as a UNC
path and selects its shared WAL lock handle. SQLite 3.53.3 excludes the local
drive form while retaining normal `\\server\share` and `\\?\UNC\...` paths.
The function change here follows the [official 3.53.3 Windows VFS source](https://raw.githubusercontent.com/sqlite/sqlite/version-3.53.3/src/os_win.c)
and the [SQLite change record](https://sqlite.org/src/timeline?from=version-3.53.0&to=version-3.53.3&to2=branch-3.53&y=ci)
for check-in `246f46614f`. Compare the
[original 3.53.2 Windows VFS source](https://raw.githubusercontent.com/sqlite/sqlite/version-3.53.2/src/os_win.c).

## Per-connection page caches

`build.rs` no longer passes `-DSQLITE_ENABLE_MEMORY_MANAGEMENT` in the
bundled build. With that flag, SQLite's `pcache1Init` puts every
connection in one page-cache group (*"Always use a unified cache (mode-2)
if ENABLE_MEMORY_MANAGEMENT"*). A connection at its own `cache_size`
limit then recycles other connections' pages, and the robbed connections
stay near one page until they are reopened. Without the flag, each
connection has its own page cache, with no global page-cache mutex. No
other compile flag changes, and `sqlite3.c` is not touched by this
change. See `dev/plans/0.8.28/features/slice-40/design.md` § 2h and
`dev/adr/ADR-0.8.28-sqlite-per-connection-page-cache.md`.

rusqlite removed the flag upstream in
[PR #1900](https://github.com/rusqlite/rusqlite/pull/1900) (commit
`f31f7324a82894be13e5de1665cb9af45dc5f869`, merged 2026-09-04). No
crates.io `libsqlite3-sys` release contains it yet. **Exit condition:**
drop this change when FathomDB's pinned `libsqlite3-sys` release
contains that commit. The engine test
`bundled_sqlite_keeps_a_page_cache_per_connection` fails if a later
build turns the flag back on.

## Digests

Original registry amalgamation SHA-256:
`0a409f1633283fa31a9126b11fbfd64a1991c5d30defad07e5745d4667f5e23d`.
Patched amalgamation SHA-256:
`7e4799907bdc92cff8858db2c52f5a03e136d5989ead6e513c0ef64057baeb9e`.
Vendored crate tree SHA-256 (all files except this note, with sorted relative
paths and each file's SHA-256 bytes; covers both changes):
`ba21fdceb02dc7d7224a81d1b44e9c114c39d6a0ae0000c4f4b0b320c658fdeb`.
The Windows Engine test `local_canonical_path_does_not_strand_idle_wal_readers`
exercises the correction using the path that Engine passes to SQLite.
