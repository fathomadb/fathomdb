# fathomdb-engine

The core of **FathomDB** — a local-first, embedded retrieval engine for application and agent
workloads. Storage, schema admission/bootstrap, ingest, projection maintenance and hybrid query live here.

> Rust applications sharing the Python/TypeScript surface should use
> [`fathomdb-sdk`](https://crates.io/crates/fathomdb-sdk). The
> [`fathomdb`](https://crates.io/crates/fathomdb) crate is the lower-level facade.
> Depend on `fathomdb-engine` directly for lower-level features or to build a binding.

## Status: pre-1.0, beta

The 0.8.x line is under active development. The public surface can change between minor releases,
and scale/stability guarantees are staged for 0.9.x and later.

## What it is

A single-file SQLite database (bundled `rusqlite`, plus the `sqlite-vec` extension) with an engine
process model layered on top:

- **Canonical rows are the source of truth.** Callers write nodes and edges; the engine owns every
  derived structure.
- **Projections are engine-owned and declarative.** `configure_projections` diffs a caller's
  declaration against the durable registry and backfills the difference. FTS5 and filterable
  projections build in the same transaction; vector and rankable targets are deferred to a
  background scheduler.
- **Retrieval is hybrid.** FTS5 lexical search and `sqlite-vec` dense retrieval are fused, with
  optional cross-encoder reranking, an optional graph-expansion arm, and a typed soft-fallback
  report when a branch cannot contribute.
- **Provenance is mandatory.** Every canonical row carries a `SourceId`, which is what makes
  `erase_source` able to erase it — including its FTS, vector and secondary-index shadows — at rest.
- **Records have a lifecycle.** `transition` and `purge` move and hard-erase governed nodes;
  `ReadView` selects existence and world-time validity for the read verbs.

Public open in 0.8.30 creates missing/empty databases at schema 37 and accepts
only that exact schema for nonempty stores. It rejects incompatible versions;
there is no automatic migration. Low-level migration helpers in `fathomdb-schema`
serve internal/test/tooling uses, not an application upgrade contract.

## Install

```bash
cargo add fathomdb-engine
```

## Cargo features

| Feature | Effect |
| --- | --- |
| `operator` | Un-gates the operator / recovery seam (`rebuild_*`, `excise_source`, `dump_*`, `trace_source_ref`, `truncate_wal`, `verify_embedder`, `check_integrity`, `safe_export`, `recompute_mean`). A gate, not a behaviour switch. |
| `default-embedder` | Lets the engine materialise its pinned default embedder at open time. Weights are fetched on first use and cached under `~/.cache/fathomdb/embedders/`. Without the feature, asking for the default embedder returns a typed error rather than touching the network. |
| `default-reranker` | Enables the built-in cross-encoder reranker. |
| `embed-cuda` / `embed-metal` | GPU acceleration for embedding (opt in at build time, select at runtime with `FATHOMDB_EMBED_DEVICE`). |
| `rerank-cuda` / `rerank-metal` | GPU acceleration for the reranker (`FATHOMDB_RERANK_DEVICE`). |

Defaults are empty: a plain `cargo add fathomdb-engine` pulls no model runtime and performs no
network access.

## Related crates

| Crate | Role |
| --- | --- |
| [`fathomdb-sdk`](https://crates.io/crates/fathomdb-sdk) | Rust application SDK, peer of Python and TypeScript |
| [`fathomdb`](https://crates.io/crates/fathomdb) | Lower-level engine facade |
| [`fathomdb-schema`](https://crates.io/crates/fathomdb-schema) | Versioned migration registry |
| [`fathomdb-query`](https://crates.io/crates/fathomdb-query) | Query compilation and the JSON filter AST |
| [`fathomdb-embedder`](https://crates.io/crates/fathomdb-embedder) | Built-in embedder runtimes |
| [`fathomdb-embedder-api`](https://crates.io/crates/fathomdb-embedder-api) | The `Embedder` trait contract |

## License

MIT. See the `LICENSE` file shipped in this crate.

Source, issues and full documentation: <https://github.com/fathomadb/fathomdb>
