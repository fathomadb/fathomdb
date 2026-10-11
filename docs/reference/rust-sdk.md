# Rust SDK

Crate: `fathomdb-sdk` (import `fathomdb_sdk`), introduced in 0.8.27. It has the same surface as the Python and TypeScript SDKs:

- an `Engine` with the same operations;
- the `read`, `graph`, and `admin` namespaces;
- the standalone `rerank` and `embed_batch_cls` operations;
- the same option defaults and error categories, represented by one Rust
  `Error` type and its `ErrorKind`.

The repository check that verifies Python and TypeScript membership also
verifies the Rust SDK: 44 of 44 governed operations.

## Shape

```rust
use fathomdb_sdk::{read, Engine, InitialState, OpenOptions, PreparedWrite,
    SourceId, TextSearchOptions};

fn main() -> fathomdb_sdk::Result<()> {
    let engine = Engine::open("./app.sqlite", OpenOptions::default())?;
    engine.write(&[PreparedWrite::Node {
        kind: "note".into(),
        body: "the quick brown fox".into(),
        source_id: SourceId::new("demo-source").expect("valid source id"),
        logical_id: Some("note:1".into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }])?;

    let note = read::get(&engine, "note:1", None)?;
    let hits = engine.search_text_only("brown fox", TextSearchOptions::default())?;
    println!("note: {note:?}; matches: {}", hits.results.len());
    engine.close()?;
    Ok(())
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
