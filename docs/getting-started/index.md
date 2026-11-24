# Getting Started

Native Python and Node artifacts cover Linux x86_64/glibc, Linux
AArch64/glibc, macOS x64/arm64, and Windows x64. See the
[compatibility matrix](../compatibility/index.md) for CPU/CUDA and Jetson routes.

## Where to go

- [0.8.30 release notes](../release-notes/0.8.30.md) (published) and
  [0.8.31 release notes](../release-notes/0.8.31.md) (pre-publication target) —
  changes, compatibility boundaries, and schema migration guidance.

- [Quickstart](quickstart.md) — install, open, write, search,
  counters, close, exit. ~5 minutes.
- [Install — Python](../install/python.md)
- [Install — TypeScript / Node.js](../install/typescript.md)
- [Install — Rust](../install/rust.md)

## Release 0.8.31 surface (pre-publication target; newest published release is 0.8.30)

The Python, TypeScript, and Rust application SDKs use equivalent idiomatic
names and the same governed operation membership. The separate Rust facade
exposes lower-level inherent methods and re-exported types:

- **Core** — `Engine.open`, `write`, `search`, `close`,
  `admin.configure`.
- **Model lifetime** — standalone `release_models` / `releaseModels` releases
  CLS and cross-encoder caches; engine-owned providers close with their engine.
- **Retrieval** — `search` (hybrid FTS5 + vector, RRF-fused, optional
  cross-encoder rerank, graph arm, and opt-in agent `hints`), `search_text_only`,
  `rerank`, `embed`.
- **Reads** — `read.get`, `read.get_many`, `read.list`,
  `read.collection`, `read.operational_collections`,
  `read.operational_collection`, `read.mutations`,
  `read.crossed_boundary_since`, `read.projections`, projection-generation and
  mutation readiness, frozen canonical pagination, and operational-state reads.
- **Provenance and dependencies** — versioned source provenance, immutable
  source dependencies, keyed dependency-closure status, and bounded dependency
  tracing.
- **Atomic actuation** — bounded caller-decided batches with idempotent terminal
  receipts.
- **Frozen retrieval and evidence** — authenticated frozen contexts, frozen
  search, single (`resolve_evidence`) and bounded batch
  (`resolve_evidence_many`, `1..=50` with per-slot outcomes) exact source
  evidence resolution.
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
  safe-export, verify-embedder, trace, dumps, `orphan-provenance`),
  `fathomdb recover --accept-data-loss` (truncate-wal, rebuild-vec0,
  rebuild-projections, complete-erasures, excise-source, excise-record), and
  `fathomdb migrate --backup-dir <dir> <db_path>` (offline Schema 37-to-38
  upgrade). In the 0.8.31 line the CLI is also packaged via `fathomdb-cli` /
  `pip install 'fathomdb[cli]'` while keeping base `fathomdb` script-free.
- Local-first storage on SQLite (FTS5 + `sqlite-vec`), on-disk schema
  version **38** (0.8.31 target; published 0.8.30 uses schema 37). Public opens
  create a fresh schema-38 database or accept an existing schema-38 database;
  they never auto-migrate. Existing committed schema-37 stores upgrade offline
  via `fathomdb migrate --backup-dir <dir> <db_path>`.
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
- **Caller-supplied engine embedders** are unsupported before 1.x in every SDK;
  production engine admission selects the pinned built-in default or none.
- **No automatic upgrades on `Engine.open`.** A non-empty database must already
  have the compiled schema version (38 in 0.8.31); upgrade committed schema-37
  stores offline with `fathomdb migrate --backup-dir <dir> <db_path>` — see
  [compatibility](../compatibility/index.md) and [CLI reference](../reference/cli.md).
