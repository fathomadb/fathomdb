# Getting Started

> **Release 0.8.27, pre-1.0 beta.** Choose Python, TypeScript, or the Rust
> application SDK. Install the exact release version.

## Where to go

- [Quickstart](quickstart.md) — install, open, write, search,
  counters, close, exit. ~5 minutes.
- [Install — Python](../install/python.md)
- [Install — TypeScript / Node.js](../install/typescript.md)
- [Install — Rust](../install/rust.md)

## What ships in 0.8.27

The Python, TypeScript, and Rust application SDKs use equivalent idiomatic
names and the same governed operation membership. The separate Rust facade
exposes lower-level inherent methods and re-exported types:

- **Core** — `Engine.open`, `write`, `search`, `close`,
  `admin.configure`.
- **Retrieval** — `search` (hybrid FTS5 + vector, RRF-fused, optional
  cross-encoder rerank and graph arm), `search_text_only`, `rerank`,
  `embed`.
- **Reads** — `read.get`, `read.get_many`, `read.list`,
  `read.collection`, `read.mutations`, `read.crossed_boundary_since`,
  `read.projections`, projection-generation and mutation readiness,
  frozen canonical pagination, and operational-state reads.
- **Provenance and dependencies** — versioned source provenance, immutable
  source dependencies, keyed dependency-closure status, and bounded dependency
  tracing.
- **Atomic actuation** — bounded caller-decided batches with idempotent terminal
  receipts.
- **Frozen retrieval and evidence** — authenticated frozen contexts, frozen
  search, and opt-in exact source evidence resolution.
- **Graph** — `graph.neighbors`, `graph.search_expand`, and the constrained
  `graph.expand` API.
- **Lifecycle / erasure** — `transition`, `purge`, `erase_source`.
- **Projections** — `configure_projections` (declarative registry) +
  `read.projections`.
- **BYO-LLM** — `ingest_with_extractor`, `consolidate_with_provider`.
- Engine-attached instrumentation: `open_report`, `drain`, `counters`,
  `set_profiling`, `set_slow_threshold_ms`, host-logger / subscriber
  attach.
- Operator CLI: `fathomdb doctor` (integrity, bounded data-plane integrity,
  safe-export, verify-embedder, trace, dumps, `orphan-provenance`) and
  `fathomdb recover --accept-data-loss` (truncate-wal, rebuild-vec0,
  rebuild-projections, excise-source, excise-record).
- Local-first storage on SQLite (FTS5 + `sqlite-vec`), on-disk schema
  version **34**. Public opens create a fresh schema-34 database or accept an
  existing schema-34 database; they do not upgrade older non-empty databases.
- Two-axis versioning: workspace lockstep across the
  runtime/binding/CLI crates and the independently versioned
  `fathomdb-embedder-api` trait crate.

## Breaking since 0.8.9

- `SearchHit.id` is a typed `IdSpace` (`{space, value}`), no longer an
  integer row cursor. See [structured search hits](../guides/structured-search-hits.md).
- `source_id` is **mandatory** on every canonical node/edge write.
- Search results are ordered by **RRF fusion**, not the old
  union-dedup.
- An unsatisfiable `valid_from >= valid_until` window is refused with
  `WriteValidationError` (message-less), not `InvalidArgumentError`.
- A projection spec carrying `fts`/`vector` without the `searchable`
  role is refused.

The [CHANGELOG](https://github.com/fathomadb/fathomdb/blob/main/CHANGELOG.md)
is the authoritative list.

## Use the right SDK

Choose Python, TypeScript, or `fathomdb-sdk` for the high-level application
surface. Rust users can also consume the lower-level `fathomdb` facade crate. The
`fathomdb-cli` binary provides operator diagnostics and recovery.

## Known limitations

- Published performance evidence is workload- and environment-specific, not a
  general latency SLA; measure on the intended corpus and hardware. See
  [compatibility § performance posture](../compatibility/index.md).
- **Custom Python / TypeScript embedder implementations** are not
  exposed; the binding choice is the built-in default embedder or none.
- **No compatibility shims or public automatic upgrades.** A non-empty
  database must already have the compiled schema version (34) — see
  [compatibility](../compatibility/index.md).
