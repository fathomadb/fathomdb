# Install — Rust

Rust application code uses `fathomdb-sdk`, the peer of the Python and
TypeScript SDKs. The lower-level facade and operator CLI remain separate
consumption routes:

- **`fathomdb-sdk`** (0.8.28) — the
  [Rust SDK](../reference/rust-sdk.md) with the Python/TypeScript surface.
- **`fathomdb` facade crate** — re-exports the runtime verbs from
  `fathomdb-engine` for downstream Rust libraries and applications.
- **`fathomdb-cli` operator CLI** — `fathomdb doctor` and
  `fathomdb recover` verbs. Operator-only; does **not**
  ship `search` / `get` / `list` query verbs.

> **Release 0.8.28, pre-1.0 beta.**

## Requirements

- Rust **1.95.0** or newer (declared MSRV; CI pins 1.95.0).
- A supported Rust target, C/C++ build tools and the normal platform linker.
  `rusqlite` bundles SQLite; `sqlite-vec` is a pinned Rust dependency. No system
  SQLite headers or separately installed extension are required.
- Python/npm artifacts cover Linux x86_64 and Linux AArch64 glibc, macOS
  x64/arm64 and Windows x64. Rust crates compile from source; CUDA builds have
  the toolkit/library requirements in the [embedder guide](../embedder.md).

## Install the lower-level facade and CLI

Lower-level library:

```bash
cargo add fathomdb@=0.8.28
```

CLI:

```bash
cargo install fathomdb-cli --version 0.8.28
```

From 0.8.28, source crates use `rusqlite` 0.41.1 / `libsqlite3-sys` 0.39.1
(SQLite 3.54.1) without `SQLITE_ENABLE_MEMORY_MANAGEMENT`, so page caches are
per connection without a caller flag. Pin the application SDK and CLI to the
matching release; source consumers must also keep sibling crate versions and
required repository patches aligned.

For the immutable 0.8.28 data-plane inspection route, pin the exact version and
verify the discovered binary:

```bash
cargo install fathomdb-cli --version '=0.8.28' --locked
fathomdb --version
```

Public 0.8.28 opens bootstrap a missing or zero-length path directly at schema
36 and accept a non-empty database only when its committed schema is exactly
36. There is no public automatic-upgrade route.

## Install the application SDK

For the application surface shared with Python and TypeScript:

```bash
cargo add fathomdb-sdk@=0.8.28
```

Use this complete program in `src/main.rs`, then run `cargo run`:

```rust
use fathomdb_sdk::{Engine, OpenOptions, SearchOptions};

fn main() -> fathomdb_sdk::Result<()> {
    let engine = Engine::open("./sdk-example.sqlite", OpenOptions::default())?;
    let hits = engine.search("hello", SearchOptions::default())?;
    assert!(hits.results.is_empty());
    engine.close()?;
    Ok(())
}
```

See the [Rust SDK reference](../reference/rust-sdk.md) for operations and
option defaults. To build from source, use a trusted
`release/0.8.28` checkout beside the application:

```bash
cargo add fathomdb-sdk --path ../fathomdb/src/rust/crates/fathomdb-sdk
```

The lower-level example below deliberately imports `fathomdb`.

## Lower-level facade feature flags

These flags describe the `fathomdb` facade. The application SDK has no
`operator` feature or raw SQL route. It enables no features by default; opt
into `default-embedder` and `default-reranker` for those model runtimes. See
the [SDK feature reference](../reference/rust-sdk.md#features).

- **default (no features)** — the *governed application surface*: `Engine::open`
  / `write` / `search` / `close`, the `read_*` and `graph_*` verbs, the
  lifecycle verbs (`transition`, `purge`, `erase_source`) and the projection
  registry. No recovery-named method and no raw-SQL method resolves.
- **`operator`** — un-gates the operator/recovery seam (`rebuild_*`,
  `excise_source`, `dump_*`, `check_integrity`, `safe_export`, …) and the
  report types those methods return. `fathomdb-cli` enables it. Gating, not
  deletion: engine behaviour is identical either way.
- **`default-embedder`** — compiles in the in-process `bge-small-en-v1.5`
  embedder (opt in per engine at `open`). See the
  [Default Embedder guide](../embedder.md).

## Verify

```rust
use fathomdb::{Engine, PreparedWrite, SourceId};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // `Engine::open` returns an `OpenedEngine { engine, report }`.
    let opened = Engine::open("./hello.fdb")?;
    let engine = opened.engine;

    engine.write(&[PreparedWrite::Node {
        kind: "note".into(),
        body: "hello".into(),
        // `source_id` is MANDATORY since 0.8.20. The `SourceId` newtype makes
        // an un-provenanced write a COMPILE error, not a runtime rejection.
        source_id: SourceId::new("demo-source")?,
        logical_id: None,
        state: Default::default(),
        reason: None,
        valid_from: None,
        valid_until: None,
    }])?;

    engine.search("hello")?;
    engine.close()?;
    println!("ok");
    Ok(())
}
```

`PreparedWrite` is `#[non_exhaustive]`: new variants can land in a micro
release, so `match` on it with a `_` arm.

For the CLI:

```bash
fathomdb doctor data-plane-integrity --json ./store.sqlite
```

Stop all FathomDB processes using the store first. The command requires the
existing product lock and an exact database-schema match, refuses non-empty
WAL/rollback-journal sidecars, and performs no migration or repair. See the
[integrity inspection procedure](../operations/integrity-inspection.md).

## See also

- [Reference — Rust SDK](../reference/rust-sdk.md)
- [Reference — Rust API](../reference/rust-api.md)
- [Reference — CLI](../reference/cli.md)
- [Reference — errors](../reference/errors.md)
- Rust API docs are published at `docs.rs/fathomdb`. For the current source,
  run `cargo doc --open`, or read
  [`src/rust/crates/fathomdb/`](https://github.com/fathomadb/fathomdb/tree/main/src/rust/crates/fathomdb).
