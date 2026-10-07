# Rust SDK

Crate: `fathomdb-sdk` (import `fathomdb_sdk`), introduced in 0.8.27 and not
yet published. It has the same surface as the Python and TypeScript SDKs:

- an `Engine` with the same operations;
- the `read`, `graph`, and `admin` namespaces;
- the standalone `rerank` and `embed_batch_cls` operations;
- the same option defaults and error classes.

The repository check that verifies Python and TypeScript membership also
verifies the Rust SDK: 44 of 44 governed operations.

## Shape

```rust
use fathomdb_sdk::{admin, graph, read, Engine, ErrorKind, NeighborsOptions, OpenOptions,
    SearchOptions};

let engine = Engine::open("./app.sqlite", OpenOptions::default())?;
let hits = engine.search("brown fox", SearchOptions { rerank_depth: 3, ..Default::default() })?;
let note = read::get(&engine, "note:1", None)?;
let near = graph::neighbors(&engine, "note:1", 1, NeighborsOptions::default())?;
admin::configure(&engine, "settings", r#"{"type":"object"}"#)?;
match engine.close() {
    Err(err) if err.kind() == ErrorKind::Closing => {}
    other => other?,
}
```

## Differences from Python and TypeScript

The Rust SDK differs from the other two only in Rust idiom:

- **Naming.** Names are `snake_case`, as in Python.
- **Calls.** Calls are synchronous and return `Result`. TypeScript returns
  Promises.
- **Optional arguments.** They become option structs (`SearchOptions`,
  `FrozenSearchOptions`, `ListOptions`, `NeighborsOptions`, `RerankOptions`,
  ...). Each struct's `Default` equals the Python/TypeScript defaults. For
  example, `limit` is 10, `alpha` is 0.3, and `pool_n` defaults to
  `rerank_depth`.
- **Errors.** There is one `Error` type. `err.kind()` returns an `ErrorKind`
  named after the shared error class: `ErrorKind::Closing` is `ClosingError`,
  for example. Typed payloads stay available through the wrapped engine
  errors.
- **Divergences.** Where Python and TypeScript differ, Rust follows
  TypeScript:
  - `drain` takes milliseconds.
  - `graph::search_expand` takes a `SearchFilter`.
  - Frozen search defaults `pool_n` to `rerank_depth`.

Custom embedder injection, the recovery seam, raw SQL, and test hooks are not
part of the SDK. The lower-level [`fathomdb` crate](rust-api.md) remains the
engine facade.

## Features

No feature is enabled by default. `default-embedder` matches the shipped
Python wheel and npm package. Without it, `use_default_embedder`,
`Engine::embed`, and `embed_batch_cls` return typed embedder errors.
`default-reranker` enables cross-encoder reranking. The `embed-cuda`,
`rerank-cuda`, `embed-metal`, and `rerank-metal` features select GPU builds.
