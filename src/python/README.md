# FathomDB

Python SDK for **FathomDB** — a local-first, embedded retrieval engine for application and agent
workloads, built on SQLite.

A FathomDB database is a single file on your disk and runs in your process without a server.
Plain open uses no model network access. Explicitly enabling the pinned default embedder may
fetch verified weights on a cold cache, as described below.

## Status: pre-1.0, beta

The 0.8.x line is under active development (published release: 0.8.31). The
public surface can change between minor releases, and scale/stability guarantees
are staged for 0.9.x and later.

## Install

```bash
pip install fathomdb==0.8.31
```

Requires Python 3.10 or newer. Wheels are `abi3`, so one wheel per platform serves every supported
interpreter. The package is a PyO3 binding to the FathomDB Rust engine and has no Python
dependencies of its own. Starting in `0.8.31`, the optional `fathomdb[cli]`
extra (`pip install 'fathomdb[cli]==0.8.31'`) installs the lockstep
`fathomdb-cli` binary wheel alongside the SDK while keeping the base `fathomdb`
wheel free of console scripts.

## Quick start

```python
from fathomdb import Engine

engine = Engine.open("./app.sqlite")

engine.write([
    {
        "kind": "doc",
        "body": "the quick brown fox",
        # Provenance is MANDATORY on every canonical row: source_id is the
        # axis erase_source() erases on, so a row without one is unerasable.
        "source_id": "import-2026-07",
    }
])

result = engine.search("brown fox")
for hit in result.results:
    print(hit.id, hit.kind)

engine.close()
```

## What is on the surface

- **Engine lifecycle** — `Engine.open` (plus `0.8.32` target
  `create_if_missing=False` / `DatabaseNotFoundError` and opt-in `ScopePolicy`
  caller isolation defaulting to `ScopePolicy.unscoped()`), `engine.close`,
  `engine.drain`, `engine.counters()`, `engine.open_report()` (including
  `reranker_compiled`, `cuda_compiled`, and `rerank_available` capability
  booleans), and a logging-subscriber seam.
- **Writes** — `engine.write(batch)` (plus `0.8.32` target opt-in
  `engine.write_with_options(batch, WriteOptions.bulk_non_strict())` and
  `ActuationOperationV1.put_latest_state`) of canonical nodes
  (`{"kind", "body", "source_id", "logical_id"?, "state"?, "reason"?, "valid_from"?,
  "valid_until"?, "properties"?}`) and edges (`{"edge": {"kind", "from", "to", "source_id", "properties"?}}`).
- **Search** — `engine.search(query, ...)` fuses FTS5 lexical retrieval with dense vector
  retrieval, with optional cross-encoder reranking, JSON/property filters, an opt-in explain
  sidecar, and opt-in agent search `hints` (`SearchResult.hints`). A
  `soft_fallback` field reports when a branch could not contribute, rather than degrading silently.
- **Frozen reads and evidence** — `engine.freeze_read_context`, `engine.search_frozen`,
  `engine.search_with_evidence`, `engine.resolve_evidence`, and bounded batch
  `engine.resolve_evidence_many` (`1..=50` refs with per-slot partial-result outcomes).
- **Reads** — the `fathomdb.read` module: `read.get`, `read.get_many`, `read.list`,
  `read.collection`, `read.operational_collections`, `read.operational_collection`,
  `read.mutations`, `read.visibility_state` (`0.8.32` target),
  `read.projections`, `read.crossed_boundary_since`. Each takes a `ReadView`
  selecting existence state and world-time validity where applicable.
- **Graph** — `fathomdb.graph.neighbors`, `fathomdb.graph.search_expand` for bounded BFS
  expansion from search hits, and `fathomdb.graph.expand` with predicates and optional
  historical `as_of_write_cursor` / `as_of_epoch_ms` coordinates.
- **Record lifecycle** — `engine.transition`, `engine.purge`, `engine.erase_source`.
- **Projections** — `engine.configure_projections`, declaring which fields are filterable,
  searchable or rankable. The engine owns every derived index; you never maintain one by hand.
- **Typed errors** — a single-rooted hierarchy under `fathomdb.errors.EngineError` with typed
  payload attributes, so you branch on `DatabaseLockedError` or `EmbedderNotConfiguredError`
  instead of parsing message strings.

## Embeddings

Vector search needs vectors. `Engine.open(path, use_default_embedder=True)` opts into the engine's
pinned default embedder; on first use the weights are downloaded and cached under
`~/.cache/fathomdb/embedders/`. The default is `False` — a plain `Engine.open` performs no network
access at all, and vector writes then raise `EmbedderNotConfiguredError` rather than quietly doing
nothing.

## Release cached module models

`from fathomdb import release_models` exposes `release_models()`, which drops
both standalone CLS and cross-encoder caches before success. Call it after
admitted model work finishes. Loading, cache waits, inference or another release
raises `OverloadedError` immediately and clears neither cache. Empty/repeated
release constructs nothing; later healthy calls reload on demand. Engine-owned
Mean providers close separately with their Engine. Terminal CUDA loss still
requires process restart. See [model lifetime and examples](https://github.com/fathomadb/fathomdb/blob/release/0.8.31/docs/embedder.md#release-module-models).

## Erasing data

Every canonical row carries a `source_id`, and that is deliberate: it is what makes the row
erasable. Under the default autonomous policy, `engine.erase_source(source_id)` removes every row written under that provenance together
with its full-text, vector and secondary-index shadows, and finishes the erasure at rest.
`engine.purge(logical_id)` does the same for a single governed record while preserving actuation
receipts for surviving sibling entities that share the same `source_id` bucket (`0.8.32` target).
Both are idempotent, so an interrupted erasure obligation can simply be retried.

A store configured with `physical_erasure_policy="explicit_maintenance"`
requires an exclusive maintenance window for physical erasure; direct
erase/purge outside that window raises `MaintenanceError`. Queueing an erasure
intent does not delete or hide records. Completion is certified by the
maintenance result, while incomplete cleanup retains its durable fences. See
the [maintenance guide](https://github.com/fathomadb/fathomdb/blob/release/0.8.31/docs/operations/erasure.md).

## Other SDKs

The same engine is available as a Node.js package (`npm install fathomdb`). The Rust
application peer is `fathomdb-sdk` (import `fathomdb_sdk`), available in
0.8.31; use the
[Rust SDK installation guide](https://github.com/fathomadb/fathomdb/blob/release/0.8.31/docs/install/rust.md).
The [`fathomdb` crate](https://crates.io/crates/fathomdb) remains the
separate lower-level engine facade.

## Building from source

From an owned checkout, select its declared developer tools and create its own `.venv`:

```bash
bash scripts/setup-python-owned.sh --provision
```

The helper installs a locked, noneditable wheel built from this source tree, with the declared
production features and owned interpreter. Its receipt records source, wheel and native-module
identity. Linked worktrees need separate environments; never borrow or repoint another checkout's
binding. The explicit test-hooks gate separately builds its candidate wheel for tests.

## License

MIT. See the `LICENSE` file shipped in this distribution.

Source, issues and full documentation: <https://github.com/fathomadb/fathomdb>
