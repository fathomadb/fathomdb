# Bundled SQLite Windows WAL path correction

This is the published `libsqlite3-sys` 0.38.1 crate, copied from the Cargo
registry package with its optional SQLCipher files intact. The only change to
its source is the `winIsUNCPath` function and adjacent comment in
`sqlite3/sqlite3.c`. It preserves bundled SQLite version 3.53.2 and its normal
linkage on every platform.

SQLite 3.53.2 treats the Win32 long DOS-device form `\\?\C:\...` as a UNC
path and selects its shared WAL lock handle. SQLite 3.53.3 excludes the local
drive form while retaining normal `\\server\share` and `\\?\UNC\...` paths.
The function change here follows the [official 3.53.3 Windows VFS source](https://raw.githubusercontent.com/sqlite/sqlite/version-3.53.3/src/os_win.c)
and the [SQLite change record](https://sqlite.org/src/timeline?from=version-3.53.0&to=version-3.53.3&to2=branch-3.53&y=ci)
for check-in `246f46614f`. Compare the
[original 3.53.2 Windows VFS source](https://raw.githubusercontent.com/sqlite/sqlite/version-3.53.2/src/os_win.c).

Original registry amalgamation SHA-256:
`0a409f1633283fa31a9126b11fbfd64a1991c5d30defad07e5745d4667f5e23d`.
Patched amalgamation SHA-256:
`153d0e416b3beb5ccbd8447ef46c12462ac988295f0ef3e4af88475eba23b36f`.
Vendored crate tree SHA-256 (all files except this note, with sorted relative
paths and each file's SHA-256 bytes):
`5081187f42871172080ee096b7692937ad41723db8995496388dcdaa26a94bec`.
The Windows Engine test `local_canonical_path_does_not_strand_idle_wal_readers`
exercises the correction using the path that Engine passes to SQLite.
