# fathomdb-sdk

The Rust application SDK for **FathomDB**, a local-first, embedded retrieval
engine for application and agent workloads, built on SQLite.

`fathomdb-sdk` has the same surface as the Python and TypeScript `fathomdb`
packages:

- an `Engine` with the governed engine operations;
- the `read`, `graph`, and `admin` namespaces;
- the standalone `rerank` and `embed_batch_cls` operations;
- the same option defaults and error categories through `Error` / `ErrorKind`.

If you are writing a Rust application against FathomDB, depend on this crate.

## Status: pre-1.0, beta

This is the Rust application SDK for FathomDB 0.8.27. Public contracts may
change between micro releases.

## Install

```bash
cargo add fathomdb-sdk@=0.8.27
```

To enable the pinned default embedder, add `--features default-embedder` to
that command. Model use can download weights; the example below requires no
model feature. See the
[Rust install guide](https://github.com/fathomadb/fathomdb/blob/release/0.8.27/docs/install/rust.md).

## Example

Save this complete program as `src/main.rs`, then run `cargo run` in the
application project:

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

`source_id` is mandatory provenance: `erase_source` removes records by that
identity. See the [SDK reference](https://github.com/fathomadb/fathomdb/blob/release/0.8.27/docs/reference/rust-sdk.md)
for hybrid search, graph operations, and option defaults.

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
