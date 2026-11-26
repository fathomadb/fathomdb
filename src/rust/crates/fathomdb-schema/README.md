# fathomdb-schema

The on-disk schema and versioned migration registry for **FathomDB**, a local-first embedded
retrieval engine built on SQLite.

## Is this the crate you want?

Probably not, unless you are working *on* FathomDB rather than *with* it.
Applications should use [`fathomdb-sdk`](https://crates.io/crates/fathomdb-sdk),
the Rust peer of the Python and TypeScript SDKs. The
[`fathomdb`](https://crates.io/crates/fathomdb) crate remains the lower-level facade.

Public `Engine::open` on the `0.8.32` target line bootstraps a missing or
zero-length database at schema **39** (published `0.8.31` used schema **38**),
or opens a nonempty database whose committed `user_version` is exactly 39. It
rejects older, future and nonempty zero-version databases before product
mutation. It does not run an automatic upgrade or compatibility reader.
Committed schema 38 stores (and chained schema 37 stores) upgrade offline via
explicit `fathomdb migrate --backup-dir <dir> <db-path>` in `fathomdb-cli`
(`38 -> 39` and chained `37 -> 38 -> 39`).
See the [schema contract](https://github.com/fathomadb/fathomdb/blob/release/0.8.31/docs/compatibility/index.md#on-disk-schema).

This crate is published separately because it is a **leaf** — it depends only on `rusqlite` — and
because it is the useful thing to reach for when you need to reason about a FathomDB database file
without opening an engine: inspecting a database's schema version, checking whether a file is at a
version your build understands, or writing tooling over the canonical tables.

## Status: pre-1.0, beta

The 0.8.x line is under active development. The public surface can change between minor releases.
The **on-disk** format has its own schema version. The low-level migration
registry and helpers remain available for internal implementation, tests and
deliberate tooling; their presence is not a supported product-open migration
route. Public applications open a fresh or current-schema store; committed
Schema 37 and Schema 38 stores upgrade offline via `fathomdb migrate --backup-dir <dir> <db-path>`
in `fathomdb-cli` (`38 -> 39` and chained `37 -> 38 -> 39`). For stores older
than Schema 37 or downgrades, export with the matching old product and re-ingest
through the current public write surface.

## What it contains

- `SCHEMA_VERSION` — **39** on the `0.8.32` target line (**38** in published
  `0.8.31`), the current bootstrap version and exact nonempty-store version
  accepted by public open.
- `TABLE_MANIFEST` — the single-source table/shadow/trigger manifest (**36**
  tables including `_fathomdb_scope_visibility_watermarks`, **20**
  visibility-trigger tables / **60** triggers).
- `MIGRATIONS` — the ordered, append-only migration registry. Each entry is a named step; the
  registry is accretion-checked, so an already-released step cannot be edited in place.
- `migrate`, `migrate_with_steps`, `migrate_with_event_sink` — run pending migrations against a
  `rusqlite::Connection`, returning a `MigrationReport` (per-step names and timings) or a typed
  `MigrationError` / `MigrationFailureReport`. These explicit low-level calls
  mutate schema and are separate from public Engine-open admission.
- `bootstrap_steps` and `CANONICAL_TABLES` — the initial schema and the canonical table set.
- File-suffix constants (`SQLITE_SUFFIX`, `WAL_SUFFIX`, `LOCK_SUFFIX`, `JOURNAL_SUFFIX`) and
  `PRAGMA_USER_VERSION`, which together describe the on-disk footprint of one database.

## Install

```bash
cargo add fathomdb-schema
```

## License

MIT. See the `LICENSE` file shipped in this crate.

Source, issues and full documentation: <https://github.com/fathomadb/fathomdb>
