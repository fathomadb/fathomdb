---
title: ADR-0.8.28-sqlite-per-connection-page-cache
date: 2026-10-16
target_release: 0.8.28
desc: The bundled SQLite is built without SQLITE_ENABLE_MEMORY_MANAGEMENT, so each connection has its own page cache and no global page-cache mutex; successor note to the page-cache premise of ADR-0.7.0-ac020-architectural-lever
blast_radius: third_party/libsqlite3-sys-0.38.1/build.rs and its vendored-tree digest pins; every artifact built from this workspace; published Rust crates only through LIBSQLITE3_FLAGS
status: proposed; implementation authorized by owner ruling 2026-10-16 (Slice 40 Unit I, ledger seq 308); acceptance recorded when the Unit I gate passes and the owner confirms; conditional approval by owner ruling 2026-10-16 ("If Unit I's gate passes, I approve the ADR"), to be recorded as accepted with the gate receipts once the gate is reported passed
---

# ADR-0.8.28 — one SQLite page cache per connection

## Context

The vendored `libsqlite3-sys` build script passes
`-DSQLITE_ENABLE_MEMORY_MANAGEMENT`. In SQLite 3.53.2 that flag makes
`pcache1Init` put every connection in one global page-cache group, guarded
by `SQLITE_MUTEX_STATIC_LRU`. Slice 40 Unit I found that one heavy read lets
its reader take every other connection's pages, and the robbed readers do
not regrow until the engine is reopened. Later reads run about 2.5× slower
([requirements § 1.7](../plans/0.8.28/features/slice-40/requirements.md)).

`ADR-0.7.0-ac020-architectural-lever.md` (superseded on its decision by
`ADR-0.8.25-absolute-read-performance-successor`) stated that `pcache1`
mutex acquires on the hit path are something "canonical SQLite cannot
eliminate without a custom page-cache allocator", and ranked a
`SQLITE_CONFIG_PCACHE2` allocator as its option 1. That premise holds only
under the build flag. Without it, a thread-safe build with the default
configuration gives each connection its own group with a null mutex
(`pcache1Create`), so the hit path takes no global page-cache lock. The
6.29% page-cache share that the 0.7.0 performance notes measured
(`dev/notes/performance-whitepaper-notes.md` § 12.2, § 12.4) was measured
under the flag.

Upstream: the flag entered rusqlite's bundled build in rusqlite PR #197 (merged 2016-12-31), copied from System.Data.SQLite's
compile-option list with no rusqlite-specific reason. Upstream removed it
on master in PR #1900 (commit
`f31f7324a82894be13e5de1665cb9af45dc5f869`, merged 2026-09-04: it "forces
the unified page cache … nothing in rusqlite itself needs it"). No
`libsqlite3-sys` release on crates.io carries it yet (0.38.2 predates it).
Third parties report the same global-mutex cost (BrokkAi/bifrost #21,
cognica-io/uqa-engine #348).

## Decision

- Workspace builds of the bundled SQLite drop
  `SQLITE_ENABLE_MEMORY_MANAGEMENT`. No other compile flag changes, and the
  amalgamation is not touched.
- Each connection keeps SQLite's default `cache_size` (2,000 KiB). There is
  no page-cache knob. The ceiling (the sum of per-connection limits) is
  unchanged; resident page memory can rise toward it.
- Published Rust crates resolve the registry `libsqlite3-sys` and keep the
  flag. They are documented, not mitigated:
  `LIBSQLITE3_FLAGS=-USQLITE_ENABLE_MEMORY_MANAGEMENT` gives the workspace
  behaviour. They get the fix without the override from the first
  `libsqlite3-sys` release containing `f31f7324`.
- The vendored removal is temporary: it is dropped when FathomDB's
  `libsqlite3-sys` pin reaches a release that contains `f31f7324`.
- On the 0.7.0 ADR's option 1: the global page-cache mutex is removed by
  this build change, not by a custom allocator. A `SQLITE_CONFIG_PCACHE2`
  allocator is not needed for that purpose.

## Consequences

- `sqlite3_release_memory()` becomes a no-op. FathomDB never calls it.
- The soft-heap-limit malloc-alarm retry is gone. FathomDB sets no heap
  limit, and its default `Performance` mode turns memory statistics off.
- Any later `libsqlite3-sys` bump must keep the flag out of the bundled
  build. The Unit I test `bundled_sqlite_keeps_a_page_cache_per_connection`
  fails if it returns.
- No claim is made here about AC-081 read throughput. The Unit I gate
  measures the change; a throughput claim needs its own measurement.

## References

- [Slice 40 design § 2h](../plans/0.8.28/features/slice-40/design.md)
- `ADR-0.7.0-ac020-architectural-lever.md` (addendum 2026-10-16)
- `ADR-0.8.25-sqlite-runtime-configuration.md` (unchanged: the page-cache
  mode is decided at compile time, not by `sqlite3_config`)
