# Rust SDK

Crate: `fathomdb-sdk` (documentation target `0.8.32`, pre-publish; newest
published crates.io release `0.8.31`; import `fathomdb_sdk`). It has the same
surface as the Python
and TypeScript SDKs:

- an `Engine` with the same operations;
- the `read`, `graph`, and `admin` namespaces;
- the standalone `rerank`, `embed_batch_cls`, and `release_models` operations;
- the same option defaults and error categories, represented by one Rust
  `Error` type and its `ErrorKind`.

The repository check that verifies Python and TypeScript membership also
verifies the Rust SDK against the current canonical operation map. See the
[generated SDK index](../generated/sdk-operation-map.md).

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
        properties: None,
    }])?;

    let note = read::get(&engine, "note:1", None)?;
    let hits = engine.search_text_only("brown fox", TextSearchOptions::default())?;
    println!("note: {note:?}; matches: {}", hits.results.len());
    engine.close()?;
    Ok(())
}
```

## Filtered direct body FTS and maintenance

`TextSearchOptions` carries `view`, `limit` and `filter: Option<SearchFilter>`.
`Engine::search_text_only` applies that filter before candidate caps,
deduplication, ranking and limit. The same-filter/same-view prefix rule and
metadata limits are shared with [Python and TypeScript](../guides/hybrid-search-filtering.md).
`SearchFilter.attributes` is node-scoped ordered AND equality; status/time need
stored vector metadata and do not become available merely by opening without a
provider.

`EngineConfig::physical_erasure_policy` selects the persisted policy. The SDK
exposes typed queue/status/withdraw outcomes and an owned `ErasureMaintenance`
window from `begin_erasure_maintenance`; activated cleanup returns typed complete
or incomplete results. See [maintenance semantics](../operations/erasure.md#schedule-an-explicit-maintenance-window).
No completion deadline or immediate-revocation promise is implied.

## Explicit module model release

`fathomdb_sdk::release_models() -> Result<()>` releases the standalone CLS and
cross-encoder caches before success. Empty and repeated calls succeed without
loading models or initializing CUDA. Concurrent loading, cache waits, inference,
or another release returns `ErrorKind::Overloaded` immediately and leaves both
caches unchanged. Healthy subsequent calls can reload. Engine-owned Mean
providers remain attached to their engines. Terminal CUDA loss remains terminal
until process restart. See [the lifecycle guide](../embedder.md#release-module-models).

Downstream exhaustive matches on `RerankerDevicePolicyError` must handle the
new `ModelBusy` variant (`kind() == "overloaded"`, no ordinal). Fallible reranker
paths map it to overload; infallible `rerank_fused` retains unchanged-input
fallback.

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
On AArch64 Linux, `embed-cuda` and `rerank-cuda` also build the Jetson private
CUDA memory pool (they imply `tegra-pool`; naming it is redundant). A Rust
program reaches the pool with no init call of its own, and turns it off with
`EngineConfig { cuda_pool: Some(CudaPoolMode::Off), .. }` on
`OpenOptions::config`. The three GPU pool settings, and the rule that one pool
configuration holds per process, are in
[configuration](config.md#engine-configuration).

## Stored-head reporting

`read::projection_status` includes `StoredHeadCoverageV1`, with exact u64/u128
mapped-head counts/bytes and unreduced known-only fractions, or explicit
cache_cold/state_changed availability without numeric facts. The stored profile
and numeric dimension describe retained mapped heads, not current eligibility
or physical-vector health. Status reads reuse only committed matching captures.
Opt-in `SnippetV1.scored_bytes: Option<u32>` describes this exact returned
record's original contributing stored head extent. `Some(0)` is known zero;
`None` makes no extent claim. Pure `snippet_for` and text-only selection return
`None` without extent reads. No governed operation, ranking change or refresh API
is added.

## Inline properties, custom vector kinds, and operational discovery

- **Inline `properties`:** `PreparedWrite::Node`, `PreparedWrite::Edge`,
  `PreparedWrite::ProvenancedNode`, and `PreparedWrite::ProvenancedEdge` accept
  optional inline JSON `properties`, surfaced on `NodeRecord` and `SearchHit`
  and indexed for declared filterable/searchable projections.
- **Typed `SearchFilter.property_predicates`:** `SearchFilter` accepts
  `property_predicates` (`eq`, `ne`, `lt`, `lte`, `gt`, `gte`) in addition to
  `attributes`.
- **`ProjectionSpec` targeting and custom vector kinds:** `ProjectionSpec`
  adds `entity_class` (`Node` or `Edge`) and `vector_kinds` (opt-in custom node
  kinds enrolled in `_fathomdb_vector_kinds` alongside the 7 built-in defaults;
  reserved `"edge_fact"` is rejected).
- **Operational collection discovery:** `read::operational_collections(&engine)`
  lists declared operational collections and
  `read::operational_collection(&engine, name)` inspects one collection's
  metadata (conflicting `kind` re-registration returns
  `ErrorKind::WriteValidation`).

## Strict endpoint integrity, traversal predicates, and opt-in fused seeding

- **Strict atomic write integrity:** `Engine::write` executes the full batch
  atomically in one transaction even across mixed `source_id` values, verifying
  that every edge (including edges superseded later in the same batch) points
  at active, non-superseded nodes (`state = 'active'`, `superseded_at IS NULL`).
  Violations return `ErrorKind::WriteValidation` with zero rows committed;
  committed batches always report `WriteReceipt.dangling_edge_endpoints == 0`.
  Same-batch forward references and active self-loops succeed. `Engine::actuate`
  retains `ActuationOutcomeV1::Refused` (`ReferenceUnavailable`).
- **Bounded `graph::expand` predicates and historical reconstruction:**
  Construct `GraphExpandRequestV1` via `GraphExpandRequestV1::new(...)`
  (`#[non_exhaustive]`) and optional fluent builders `.with_edge_predicates(...)`
  / `.with_target_predicates(...)` (at most 32 closed inline `PropertyPredicate`
  entries each, implicit `AND`; invalid predicates return
  `GraphEdgePredicatesInvalid` or `GraphTargetPredicatesInvalid`) plus mutually
  exclusive knowledge-time builders `.with_as_of_write_cursor(u64)` and
  `.with_as_of_epoch_ms(i64)` (specifying both refuses with
  `GraphContextInvalid` at `/asOfEpochMs`). Default traversal enforces
  current-time `[t_valid, t_invalid)` gating via indexed SQL pushdown;
  historical `as_of` mode composes orthogonally with `ReadView.valid_as_of`,
  resolves `as_of_epoch_ms` against `_fathomdb_commit_timestamps_ms_idx`,
  respects current-state `Engine::transition` and physical erasure /
  dependency-closure barriers, and supports frozen historical evidence
  (`artifact_lifecycle.superseded == true`).
- **Opt-in `use_graph_arm` seeding and provenance:** `SearchOptions`,
  `FrozenSearchOptions`, and `EvidenceSearchRequestV1::new(...)` accept
  `graph_seed: GraphSeedMode` (`GraphFts` default, `FusedTopK`, `Both`),
  `graph_seed_k` (`1..=100`, default `10`), `graph_edge_kinds`, and
  `graph_edge_predicates`. `PerHitExplain` exposes
  `graph_seed_source: Option<GraphArmSeedSource>` and
  `graph_hop_depth: Option<u32>`, and `EvidenceGraphOriginV1` exposes
  `seed_source: GraphArmSeedSource`. `TextSearchOptions` does not carry
  graph-arm seeding fields.

## Batch evidence resolution, search hints, and OpenReport capabilities

- **`Engine::resolve_evidence_many(&self, &EvidenceResolveManyRequestV1) -> Result<Vec<EvidenceResolveManyItemV1>>`:**
  Resolves `1..=50` (`EVIDENCE_RESOLVE_MANY_MAX = 50`) evidence references under
  one `FrozenReadContextV1` in a single read snapshot. Rejects
  `schema_version != 1` (`ErrorKind::Evidence`, `UnsupportedSchemaVersion`) and
  raw lengths `0` or `> 50` (`ErrorKind::InvalidArgument`), preserves duplicate
  order and arity, returns per-slot `EvidenceResolveManyItemV1::Resolved` or
  nondisclosing `EvidenceResolveManyItemV1::Refused` (`"/evidenceRefs/{i}"`),
  and collapses frozen-context drift into whole-call `EvidenceUnavailable`.
- **Opt-in search hints (`hints: bool`):** `SearchOptions.hints`,
  `FrozenSearchOptions.hints`, and `EvidenceSearchRequestV1::with_hints(bool)`
  default to `false` (`SearchResult.hints == None`). Setting `true` populates
  `SearchResult.hints: Option<Vec<Hint>>` (`Hint`, `HintAction`, `HintReason`,
  `HintMode`, `HintArm`, `HintCause`) without altering hit rankings.
- **`OpenReport` capability booleans:** `OpenReport` exposes
  `reranker_compiled`, `cuda_compiled`, and `rerank_available` with zero
  network I/O and zero GPU allocation during `Engine::open`. Offline `38 -> 39`
  and chained `37 -> 38 -> 39` migrations remain CLI-only (`fathomdb migrate`).

## Caller scope isolation and `search_expand` view (0.8.32, Schema 39)

- **Single-scope default:** By default a FathomDB database is single-scope /
  single-agent (`ScopePolicy::Unscoped`, storing `scope = ""`). Multi-agent or
  shared-store applications opt into caller isolation at open via
  `OpenOptions::with_scope_policy(...)` or `EngineConfig::with_scope_policy(...)`
  (`engine.config().scope_policy` reflects the open-time policy).
- **`ScopePolicy` exports:** `ScopePolicy` (`Unscoped`,
  `Scoped { read_scopes, write_scope }`, `single`, `read_only`, `read_write`,
  `is_unscoped`, `read_scopes`, `write_scope`), `ScopePolicyError`,
  `MAX_READ_SCOPES` (`32`), and `MAX_SCOPE_TOKEN_BYTES` (`128`). Invalid
  policies fail before file/lock open with `ErrorKind::InvalidArgument`.
- **`ErrorKind::ScopeViolation`:** Added to `ErrorKind::ALL` (`48` total variants
  = base `ErrorKind::Engine` + `47` leaf classes; `.name() == "ScopeViolationError"`),
  returned on read-only handle mutation attempts while preserving D-01
  nondisclosure on cross-scope endpoint/evidence lookups (`WriteValidation` /
  `ReferenceUnavailable` / `EvidenceUnavailable`).
- **`SearchExpandOptions.view`:** `SearchExpandOptions` gains `pub view:
  Option<ReadView>` and `.with_view(ReadView)`. `graph::search_expand` applies
  `options.view`; `Engine::search_expand_frozen` rejects `options.view.is_some()`
  with `ErrorKind::InvalidArgument` (`FrozenReadContextV1` fixes its view).

## Opt-in bulk write mode (`WriteOptions`, `Engine::write_with_options`, 0.8.32)

- **Strict defaults preserved:** `Engine::write(&self, batch: &[PreparedWrite])`
  and `Engine::actuate` remain strictly endpoint-validated
  (`WriteReceipt.dangling_edge_endpoints == 0` on every commit).
- **Exports:** `EndpointValidationMode::{Strict, BulkNonStrict}` (`Default::default() == Strict`)
  and `#[non_exhaustive] WriteOptions` (`WriteOptions::strict()`,
  `WriteOptions::bulk_non_strict()`, `WriteOptions::with_endpoint_validation(...)`).
- **`Engine::write_with_options(&self, batch: &[PreparedWrite], options: &WriteOptions) -> Result<WriteReceipt>`:**
  Under `WriteOptions::bulk_non_strict()`, out-of-order same-scope edges are
  admitted and `WriteReceipt.dangling_edge_endpoints` reports the exact
  unresolved endpoint count (`0`, `1`, or `2` per surviving active edge written
  in the batch; `0` for node-only batches). Subsequent same-scope batches
  providing the missing nodes resolve graph traversal without rewriting edges.
  Cross-scope active endpoints and read-only handles fail atomically with
  `ErrorKind::ScopeViolation`, and all other batch invariants remain strict.
  After bulk loading, verify graph completeness read-only with
  `fathomdb doctor data-plane-integrity --check edge_endpoint_integrity` or
  `fathomdb doctor check-integrity`.
