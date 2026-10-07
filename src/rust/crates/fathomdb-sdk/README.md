# fathomdb-sdk

The Rust application SDK for **FathomDB**, a local-first, embedded retrieval
engine for application and agent workloads, built on SQLite.

`fathomdb-sdk` has the same surface as the Python and TypeScript `fathomdb`
packages:

- an `Engine` with the governed engine operations;
- the `read`, `graph`, and `admin` namespaces;
- the standalone `rerank` and `embed_batch_cls` operations;
- the same option defaults and the same error classes.

If you are writing a Rust application against FathomDB, depend on this crate.

## Status: pre-1.0, beta

The 0.8.x line is under active development. The public surface can change
between minor releases.

## Install

```bash
cargo add fathomdb-sdk
# Match the shipped Python wheel and npm package (pinned default embedder):
cargo add fathomdb-sdk --features default-embedder
```

## Example

```rust
use fathomdb_sdk::{graph, read, Engine, InitialState, NeighborsOptions, OpenOptions,
    PreparedWrite, SearchOptions, SourceId};

let engine = Engine::open("./app.sqlite", OpenOptions::default())?;
engine.write(&[PreparedWrite::Node {
    kind: "note".into(),
    body: "the quick brown fox".into(),
    // Provenance is mandatory: `erase_source` erases by `source_id`.
    source_id: SourceId::new("import-2026-10").expect("valid source id"),
    logical_id: Some("note:1".into()),
    state: InitialState::Active,
    reason: None,
    valid_from: None,
    valid_until: None,
}])?;

let note = read::get(&engine, "note:1", None)?;
let hits = engine.search("brown fox", SearchOptions::default())?;
let near = graph::neighbors(&engine, "note:1", 1, NeighborsOptions::default())?;
engine.close()?;
# Ok::<(), fathomdb_sdk::Error>(())
```

## How it maps to Python and TypeScript

| Python / TypeScript | Rust |
| --- | --- |
| `engine.search(query, rerank_depth=3)` / `engine.search(query, undefined, 3)` | `engine.search(query, SearchOptions { rerank_depth: 3, ..Default::default() })` |
| `read.get(engine, id)` / `read.get(engine, id)` | `read::get(&engine, id, None)` |
| `except fathomdb.errors.ClosingError` / `instanceof ClosingError` | `err.kind() == ErrorKind::Closing` |
| Promise-returning calls (TypeScript) | Synchronous `Result` |

Optional arguments become option structs whose `Default` equals the
Python/TypeScript defaults. Every error has an `ErrorKind` named after the
shared error class. The full contract is `dev/interfaces/rust-sdk.md` in the
repository.

## Features

| Feature | Effect |
| --- | --- |
| `default-embedder` | Pinned BGE embedder for `use_default_embedder`, `Engine::embed` and `embed_batch_cls`. Without it, those calls return typed `Embedder` / `EmbedderNotConfigured` errors. |
| `default-reranker` | Cross-encoder reranking. Without it, `rerank` and `rerank_depth` take the identity path. |
| `embed-cuda`, `rerank-cuda`, `embed-metal`, `rerank-metal` | GPU builds of the above. |

No feature is enabled by default.

## Not in this crate

Custom embedder injection, the operator/recovery seam (`fathomdb doctor` /
`recover` in `fathomdb-cli`), raw SQL, and test hooks are not reachable here.
The lower-level `fathomdb` crate remains available as the engine facade.
