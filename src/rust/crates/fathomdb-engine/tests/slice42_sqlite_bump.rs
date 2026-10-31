//! 0.8.28 Slice 42 — the engine links the registry SQLite 3.54.1 or later.
//!
//! `libsqlite3-sys` 0.39.1 bundles SQLite 3.54.1, which keeps a page cache
//! per connection and carries the Windows local-verbatim-path WAL fix of
//! 3.53.3, so the repository no longer vendors a patched 0.38.1.

/// AC42-07: the linked SQLite is 3.54.1 or later.
#[test]
fn bundled_sqlite_is_3_54_1_or_later() {
    let version = rusqlite::version_number();
    assert!(
        version >= 3_054_001,
        "linked SQLite {version} ({}) is older than 3.54.1",
        rusqlite::version()
    );
}
