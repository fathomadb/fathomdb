---
title: Rust SDK interface (fathomdb-sdk)
date: 2026-10-06
target_release: 0.8.27
desc: Public contract of the dedicated fathomdb-sdk crate and its mapping to the Python and TypeScript SDKs
blast_radius: src/rust/crates/fathomdb-sdk; src/conformance/governed-operation-parity.json; scripts/check-sdk-surface-parity.py
status: locked
---

# Rust SDK interface (`fathomdb-sdk`)

Decision: [`ADR-0.8.27-rust-sdk-parity.md`](../adr/ADR-0.8.27-rust-sdk-parity.md).
`fathomdb-sdk` is the Rust peer of the Python and TypeScript SDKs described in
[`python.md`](python.md) and [`typescript.md`](typescript.md). Their operation
semantics, result shapes, and error payloads apply unchanged except where the
translation rules below say otherwise. The lower-level `fathomdb` facade keeps
its separate contract in [`rust.md`](rust.md).

## Boundary

`fathomdb_sdk` exports `Engine`, `EngineConfig`, `OpenOptions`, the option
structs, `SearchFilterArg`, `Error`, `ErrorKind`, `Result`, `rerank`,
`embed_batch_cls`, the rerank DTOs, the shared DTO re-exports, and the
modules `read`, `graph` and `admin`. The SDK `Engine` owns its core engine
privately; there is no `Deref`, accessor or conversion to
`fathomdb_engine::Engine`. Custom embedder injection (`EmbedderChoice`,
`open_with_choice*`), the operator/recovery seam, raw SQL, test hooks and the
Rust-only search overloads are not reachable.

The checker also refuses these routes to the core:

- `Engine` trait impls other than `Drop` and `Debug`;
- public `Engine` fields;
- glob or core-`Engine` re-exports of `fathomdb_engine`;
- extra public modules.

Operation membership is enforced by `scripts/check-sdk-surface-parity.py`
against the `rust` endpoints of `src/conformance/governed-operation-parity.json`
(48/48 live operations).

## Translation rules

These are the only allowed differences from the Python and TypeScript SDKs:

1. **Spelling.** Names use `snake_case`, matching Python spelling.
   TypeScript spells the same names in camelCase.
2. **Calls are synchronous** and return `Result<T, fathomdb_sdk::Error>`.
   TypeScript returns Promises. Python raises.
3. **Optional arguments.**
   - Optional keyword arguments become an options struct implementing
     `Default`. Its defaults equal the Python/TypeScript defaults.
   - An optional `view`/`context` argument in a namespace function is
     `Option<&T>`; `None` behaves like an omitted argument.
   - A required argument stays positional.
4. **Typed requests and results.** Requests and results use the core typed
   structs where Python/TypeScript use dicts or interfaces. The SDK
   re-exports those structs under the shared names. A core request struct
   with no constructor, such as `EvidenceSearchRequestV1`, has no implicit
   defaults in Rust. Callers set every field, and the interface document
   lists the Python/TypeScript defaults to use.
5. **Errors.** Python and TypeScript use one exception class per category.
   Rust uses one `Error` enum plus `Error::kind() -> ErrorKind`, with one
   `ErrorKind` variant per category. Where Python or TypeScript raises a
   host-language argument error (`ValueError`, `TypeError`, `RangeError`),
   Rust returns kind `InvalidArgument`. Examples are an out-of-range ranked
   `limit`, a non-finite `alpha`, and `predicates` together with `filter`.
6. **Divergences between Python and TypeScript.** Where they disagree, Rust
   follows TypeScript:
   - `drain` takes milliseconds.
   - `graph::search_expand` takes a `SearchFilter`.
   - `search_expand_frozen` takes the option `search_limit`.
   - `search_frozen` with no `pool_n` uses `rerank_depth`, which matches TypeScript and non-frozen `search` in both SDKs.
7. **String transport guard (AC-068a).** Both bindings reject an embedded NUL
   in a caller string with `WriteValidation` before calling the core, and the
   core does not check. The SDK applies the same check, using one private
   helper, to every string the Python binding checks:
   - `open` path, `query`, projection/search `name`, `kind`, `collection`,
     `record_key`, `logical_id`(s);
   - filter strings and attribute values, `Filter` term strings;
   - `sink_path`, `query_id`, `label_source`;
   - embed and CLS texts, the rerank query and passage bodies;
   - projection-spec strings;
   - every string field of a `PreparedWrite` except `source_id`, which
     AC-068a requires to keep an embedded NUL.

   It also checks `Predicate` paths and text values, including `Json` filter
   terms, and page cursors. Lone surrogates cannot occur in a Rust `&str`.
   Python checks the `ingest_with_extractor` document strings and the
   `consolidate_with_provider` axis strings, but napi does not; under rule 6
   the SDK follows TypeScript and leaves them to the core. Typed V1 request structs, which Python checks only as serialized
   JSON, are validated by the core's identity grammar.
8. **Members specific to one SDK are not reproduced:**
   - Python's `Engine.path`.
   - Python's per-knob `Engine.open` keyword arguments.
   - TypeScript's mapping helpers (`mapOpenReport`, etc.) and `_native`.
   - Panics are not caught. TypeScript's `FathomDbPanicError` and Python's
     `PanicException` have no Rust peer.

## Open and engine members

`OpenOptions { config: EngineConfig, use_default_embedder: bool }` derives
`Default`, which gives the default `EngineConfig` and `use_default_embedder =
false`. `EngineConfig` is the core struct, re-exported: its fields, ranges,
and defaults are already identical across the three SDKs.

The core-call column names a method on `fathomdb_engine::Engine` unless
another path is given. The Python and TypeScript bindings call the same core
function in every row.

| Canonical ID | `fathomdb_sdk` member | Core call |
| --- | --- | --- |
| `engine.open` | `Engine::open(path: impl AsRef<Path>, options: OpenOptions) -> Result<Engine>` | `open_with_choice_and_config(path, Default or None, config)` |
| — | `open_report(&self) -> &OpenReport`, `config(&self) -> &EngineConfig` (both SDKs expose `config`) | Stored at open: the requested config. |
| `engine.close` | `close(&self) -> Result<()>` (idempotent) | Mark closing and detach the subscriber, as napi does, then call `close`. |
| `engine.write` | `write(&self, batch: &[PreparedWrite]) -> Result<WriteReceipt>` | `write` |
| `engine.actuate` | `actuate(&self, request: ActuationBatchV1) -> Result<ActuationReceiptV1>` | `actuate` |
| `engine.register_source_dependency` | `register_source_dependency(&self, SourceDependencyRegistrationV1) -> Result<SourceDependencyV1>` | same |
| `engine.dependencies_for_source` | `dependencies_for_source(&self, DependencySourceLookupV1) -> Result<DependencyListV1>` | same |
| `engine.dependency_for_derived` | `dependency_for_derived(&self, DependencyDerivedLookupV1) -> Result<Option<SourceDependencyV1>>` | same |
| `engine.read_dependency_closure` | `read_dependency_closure(&self, ClosureLookupV1) -> Result<Option<ClosureStatusV1>>` | same |
| `engine.trace_dependency` | `trace_dependency(&self, DependencyTraceRequestV1) -> Result<DependencyTraceResultV1>` | same |
| `engine.transition` | `transition(&self, logical_id: &str, to_state: LifecycleState, reason: Option<&str>) -> Result<()>` | same |
| `engine.purge` | `purge(&self, logical_id: &str) -> Result<()>` | same |
| `engine.erase_source` | `erase_source(&self, source_id: &str) -> Result<EraseReport>` | `erase_source`; accepts every Engine-valid `SourceId`, including embedded NUL |
| `engine.configure_projections` | `configure_projections(&self, specs: &[ProjectionSpec], drop: &[String]) -> Result<ProjectionDelta>` | same |
| `engine.search` | `search(&self, query: &str, options: SearchOptions) -> Result<SearchResult>` | `search_reranked_view_with_limit` |
| `engine.search_text_only` | `search_text_only(&self, query: &str, options: TextSearchOptions) -> Result<SearchResult>` | `search_text_only_view_with_limit` |
| `engine.search_projected_text` | `search_projected_text(&self, query: &str, name: &str, options: ProjectedTextSearchOptions) -> Result<SearchResult>` | `search_projected_text_with_limit` |
| `engine.freeze_read_context` | `freeze_read_context(&self, context: &ReadContextV1) -> Result<FrozenReadContextV1>` | same |
| `engine.search_frozen` | `search_frozen(&self, query: &str, context: &FrozenReadContextV1, options: FrozenSearchOptions) -> Result<SearchResult>` | `validate_frozen_read_context_for_binding`, then `search_frozen` |
| `engine.search_expand_frozen` | `search_expand_frozen(&self, query: &str, context: &FrozenReadContextV1, depth: u32, options: SearchExpandOptions) -> Result<SearchExpandResult>` | Same validation, then `search_expand_frozen`. |
| `engine.search_with_evidence` | `search_with_evidence(&self, &EvidenceSearchRequestV1) -> Result<EvidenceSearchResultV1>` | same |
| `engine.resolve_evidence` | `resolve_evidence(&self, &EvidenceResolveRequestV1) -> Result<ResolvedEvidenceV1>` | same |
| `engine.resolve_graph_evidence` | `resolve_graph_evidence(&self, &GraphEvidenceResolveRequestV1) -> Result<ResolvedGraphEvidenceV1>` | same |
| `engine.ingest_with_extractor` | `ingest_with_extractor(&self, cmd: &[&str], documents: &[ExtractDocument]) -> Result<IngestWithExtractorReceipt>` | same |
| `engine.consolidate_with_provider` | `consolidate_with_provider(&self, cmd: &[&str], axes: &[ConsolidateAxis]) -> Result<ConsolidateReceipt>` | same |
| `engine.embed` | `embed(&self, text: &str) -> Result<Vec<f32>>` | `embed_text` |

Search through the engine's maintained FTS index returns `Storage` when a
persisted FTS row cannot be decoded. The rank-stream optimization may retry
through full stable sort, but a decoding failure there does not return a
partial successful result. See [retrieval](../design/retrieval.md).

Shared non-command members are reproduced with core semantics:

- `drain(timeout_ms: u64)`
- `dense_disabled`, `dense_disabled_reason`, `vector_equivalence_refusal_count`
- `enable_telemetry(sink_path)`, `last_telemetry_query_id`
- `record_feedback(query_id, relevant_ids, irrelevant_ids, label_source)`
- `counters`
- `set_profiling(enabled) -> Result<()>`
- `set_slow_threshold_ms(value) -> Result<()>`
- `attach_subscriber(subscriber: Arc<dyn Subscriber>) -> Result<()>`
  - It replaces any earlier attachment.
  - After `close` it fails with `Closing`.
  - `close` and `Drop` detach it.
  - This mirrors TypeScript `attachSubscriber`.

These members stay outside the operation map, as they are for Python and
TypeScript.

## Options structs

Each struct derives `Clone`, `Debug`, `PartialEq`, and `Default`.

Validation follows Python's order:

1. On frozen paths, the core's frozen-context check runs first, as in both
   bindings.
2. Option checks: a ranked-search `limit` outside `1..=100` or a non-finite
   `alpha` is refused with `InvalidArgument`. The core would also refuse the
   limit; checking it early keeps the order of errors.
3. The string guard.
4. The core call.

Two orders differ from Python, and both still report a typed error:

- `search_with_evidence` checks `limit` before the core's `schema_version`
  check.
- The page functions check strings before the core's `PageError` checks.

`graph::search_expand` checks `search_limit` first, as both SDKs do. Every
other check, including empty-string arguments (which Python rejects
host-side for `admin.configure` `name` and `graph.neighbors` `logical_id`,
but TypeScript does not), is left to the core.

`SearchOptions.filter` is a `SearchFilterArg`, because Python and TypeScript
both accept either filter form. It is
`enum SearchFilterArg { Search(SearchFilter), Unified(Filter) }`, with `From`
impls for both. `Unified` is lowered through the public
`Filter::to_search_filter()`, which refuses a `Json` term with
`InvalidFilter`.

| Struct | Fields and defaults |
| --- | --- |
| `SearchOptions` | `filter: Option<SearchFilterArg> = None`, `rerank_depth: usize = 0`, `use_graph_arm: bool = false`, `alpha: Option<f64>` (None → 0.3), `pool_n: Option<usize>` (None → `rerank_depth`), `explain: bool = false`, `view: ReadView = default`, `limit: usize = 10` |
| `TextSearchOptions` | `view: ReadView = default`, `limit: usize = 10` |
| `ProjectedTextSearchOptions` | `filter: Option<SearchFilter> = None`, `view: ReadView = default`, `limit: usize = 10` |
| `FrozenSearchOptions` | `rerank_depth = 0`, `use_graph_arm = false`, `alpha: Option<f64>` (None → 0.3), `pool_n: Option<usize>` (None → `rerank_depth`), `explain = false`, `limit = 10` |
| `SearchExpandOptions` | `search_limit: usize = 10` |
| `ListOptions` | `predicates: Vec<Predicate> = []`, `filter: Option<Filter> = None`, `limit: usize = 100`, `view: Option<ReadView> = None` |
| `NeighborsOptions` | `direction: TraversalDirection = Both`, `view: Option<ReadView> = None` |
| `RerankOptions` | `alpha: Option<f64>` (None → 0.3), `pool_n: Option<usize>` (None → `rerank_depth`) |

## `read`, `graph`, `admin`, and root operations

| Canonical ID | `fathomdb_sdk` function | Core call |
| --- | --- | --- |
| `read.get` | `read::get(engine: &Engine, logical_id: &str, view: Option<&ReadView>) -> Result<Option<NodeRecord>>` | `read_get` |
| `read.get_many` | `read::get_many(engine, logical_ids: &[String], view: Option<&ReadView>) -> Result<Vec<Option<NodeRecord>>>` | `read_get_many` |
| `read.collection` | `read::collection(engine, collection: &str, after_id: Option<i64>, limit: usize) -> Result<Vec<OpStoreRow>>` | `read_collection` |
| `read.mutations` | `read::mutations(engine, collection: &str, after_id: Option<i64>, limit: usize) -> Result<Vec<OpStoreRow>>` | The core call that both bindings use, `read_collection`. |
| `read.list` | `read::list(engine, kind: &str, options: ListOptions) -> Result<Vec<NodeRecord>>`. Non-empty `predicates` together with `filter` is `InvalidArgument`. | `read_list` or `read_list_filter` |
| `read.canonical_page` | `read::canonical_page(engine, kind, context: &FrozenReadContextV1, page: &PageRequestV1) -> Result<PageV1<NodeRecord>>` | same |
| `read.operational_state` | `read::operational_state(engine, collection, record_key, context: Option<&FrozenReadContextV1>) -> Result<Option<OperationalStateRecordV1>>` | same |
| `read.operational_state_page` | `read::operational_state_page(engine, collection, context, page) -> Result<PageV1<OperationalStateRecordV1>>` | same |
| `read.crossed_boundary_since` | `read::crossed_boundary_since(engine, since: i64, view: Option<&ReadView>) -> Result<Vec<BoundaryCrossing>>` | same |
| `read.projections` / `projection_status` / `projection_generation_status` / `embedding_readiness` | `read::<name>(engine)` | `read_<name>` |
| `read.mutation_projection_status` | `read::mutation_projection_status(engine, request: MutationProjectionStatusRequestV1)` | same |
| `graph.expand` | `graph::expand(engine, request: &GraphExpandRequestV1) -> Result<GraphExpandResultV1>` | `graph_expand` |
| `graph.neighbors` | `graph::neighbors(engine, logical_id: &str, depth: u32, options: NeighborsOptions) -> Result<Vec<NodeRecord>>` | `graph_neighbors` |
| `graph.search_expand` | `graph::search_expand(engine, query: &str, depth: u32, filter: Option<SearchFilter>, options: SearchExpandOptions) -> Result<SearchExpandResult>`. Non-empty `filter.attributes` is `InvalidArgument`, because neither SDK forwards attributes on this path. | `search_expand_with_limit` |
| `admin.configure` | `admin::configure(engine, name: &str, body: &str) -> Result<WriteReceipt>` | `write(&[PreparedWrite::AdminSchema { name, kind: "latest_state", schema_json: body, retention_json: "{}" }])` |
| — | `admin::configure_runtime(sqlite_mode: RuntimeSqliteMode) -> Result<RuntimeConfiguration>` | `fathomdb_engine::configure_runtime` |
| `rerank` | `rerank(query: &str, passages: &[RerankPassage], rerank_depth: usize, options: RerankOptions) -> Result<Vec<RerankResult>>`. Takes `RerankPassage { id: u64, body: String, score: f64 }` and returns `RerankResult { id: u64, score: f64, ce_score: Option<f64> }`. A non-finite `alpha` is `InvalidArgument`, matching TypeScript. A non-finite passage `score` is `WriteValidation`, following Python and napi; a reranker refusal from the core's `RerankPassagesError` keeps its typed kind (`RerankerDevicePolicy` or a CUDA pool kind). This is a deliberate exception to rule 6, because the TypeScript wrapper's `RangeError` is a host-side check. | `fathomdb_engine::rerank_passages` |
| `pack_context` | `pack_context(hits: &[ContextItem], policy: &ContextPolicy, context: &[ContextItem]) -> Result<ContextPack>`. Pure; no engine. Invalid input is `InvalidArgument` whose message names the field. See § Context pack. | `fathomdb_engine::pack_context` |
| `encode_round_body` | `encode_round_body(parts: &[ContextPart]) -> Result<String>`. | `fathomdb_engine::encode_round_body` |
| `decode_round_body` | `decode_round_body(body: &str) -> Result<Vec<ContextPart>>`. | `fathomdb_engine::decode_round_body` |
| `context_item_from_hit` | `context_item_from_hit(hit: &SearchHit, options: ContextHitOptions) -> Result<ContextItem>`. `ContextHitOptions { group, position, order_key, header }` defaults to all `None`. | `IdSpace::to_prefixed` plus `decode_round_body` |
| — | `embed_batch_cls(texts: &[&str]) -> Result<Vec<Vec<f32>>>`. With `default-embedder`: empty input returns `[]`; the embedder is a process singleton cached only on success. CUDA pool failures during load or embedding retain their typed CUDA kinds and details; other load failures are `EmbedderNotConfigured`, and other embedding failures are `Embedder`. Without the feature, every call, including one with empty input, returns `EmbedderNotConfigured`. | `CandleBgeEmbedder::new()?.with_pooling(Pooling::Cls)` then `Embedder::embed_batch` |

The namespace functions take `&Engine` first, as the Python and TypeScript
namespace functions take the engine.

## Types

The SDK root re-exports a concrete, closed list, assembled from three
sources:

1. **The facade list.** All of the `fathomdb` facade's non-operator,
   non-recovery re-exports (`fathomdb/src/lib.rs` unconditional block),
   except `Engine`, `OpenedEngine`, `Subscription`, and the functions
   `configure_runtime` and `encode_resolved_graph_evidence_v1`, none of
   which either SDK exposes.
   `ExciseReport` is re-exported as `EraseReport`, the name Python and
   TypeScript use.
2. **Names the facade omits but an SDK signature or public field needs:**
   - `SearchHit`, `IdSpace`, `IdSpaceKind`, `Filter`, `FilterTerm`
   - `OpStoreRow`, `ConsolidateAxis`, `ConsolidateReceipt`
   - the `lifecycle` items a subscriber implementation needs: `Subscriber`,
     `Event` (re-exported as `SubscriberEvent`, the TypeScript name),
     `Phase`, `EventSource`, `EventCategory`, `ProfileRecord`,
     `ProjectionStatus`, `SlowStatement`, and `StressFailureContext`
3. **Types from the schema and embedder crates:**
   - `MigrationStepReport` (schema)
   - `EmbedderIdentity`, and `EmbedderError` re-exported as
     `RuntimeEmbedderError` (the `EngineOpenError::Embedder` payload)
     (embedder-api)
   - `EmbedderEvent`, `DeviceResolution`, `RerankerDeviceResolution`,
     `EmbedDevicePolicyError`, and `RerankerDevicePolicyError` (embedder)
   - the CUDA device and witness types inside `OpenReport` that Python and
     TypeScript export, under their core names, including the allocator
     report types `CudaAllocatorReport`, `CudaAllocatorPath`,
     `CudaAllocatorReason`, `ReleaseThreshold` and `ModuleLoadInit`

Python/TypeScript inventory concepts that are enum variants or plain fields
in Rust are not separate types. Examples are the source-locator variants and
the TypeScript-only `*Options` interfaces, which the option structs above
replace. The table below records each mapping. The consumer test imports
every re-exported name, so a dropped re-export fails to compile.

| Python / TypeScript name | Rust SDK form |
| --- | --- |
| `EngineOpenOptions` (TS), `Engine.open` keyword arguments (Py) | `OpenOptions` |
| `SearchOptions` (TS view + `limit`), search keyword arguments (Py) | `SearchOptions`, `TextSearchOptions`, `ProjectedTextSearchOptions` |
| `FrozenSearchOptions`, `SearchExpandOptions` (TS) | Same-named structs. `search_limit` replaces Python's `limit`. |
| `ReadCollectionOptions` (TS) | Positional `after_id: Option<i64>, limit: usize` |
| `RerankPassage`, `RerankOptions`, `RerankResult` (TS); passage/result dicts (Py) | Same-named structs |
| `ExpandedNode` | `(NodeRecord, u32)` in `SearchExpandResult.expanded` |
| `WholeBodySourceLocator`, `Utf8BytesSourceLocator` | Variants of `SourceLocator` |
| `SubscriberCallback` (TS), logger adapter (Py) | `Arc<dyn Subscriber>` |
| `LifecycleState` (TS string union), state strings (Py) | `LifecycleState` enum |

An optional view takes one of three forms, each pinned by the surface test:

- `Option<&ReadView>`, positional, in `read` functions;
- `view: ReadView` with a default, in search options;
- `view: Option<ReadView>`, in `ListOptions` and `NeighborsOptions`.

## Errors

```rust
pub enum Error {
    Engine(EngineError),
    Open(EngineOpenError),
    RuntimeConfiguration(RuntimeConfigurationError),
    Sdk { kind: ErrorKind, message: String },
    Cuda(CudaErrorDetails),
}
pub type Result<T> = std::result::Result<T, Error>;

#[non_exhaustive]
pub enum CudaErrorDetails {
    PoolExhausted { ordinal: usize, max_size_bytes: u64, message: String },
    ContextLost {
        recorded_context_id: u64,
        current_context_id: Option<u64>,
        driver_error: String,
        operation: String,
    },
    PrivateBuildRefused { ordinal: usize, message: String },
}
impl Error { pub fn cuda_details(&self) -> Option<&CudaErrorDetails>; }
```

`Error` implements `Display`, `std::error::Error` (with `source`), and `From`
for the three core error types. The core error enums are re-exported so
callers can still read typed payloads such as `holder_pid` and `legal`.
`Sdk` holds errors raised by the SDK itself or by a component outside the
engine, such as invalid rerank input or a CLS embedder failure.

`Cuda` holds the three CUDA private-memory-pool kinds from every path (engine,
open, reranker, CLS embedder). `From<EngineError>` and `From<EngineOpenError>`
produce it for `CudaPoolExhausted`, `CudaContextLost` and
`CudaPrivateBuildRefused` (including a reranker or open-time embedder failure
of one of those kinds); `Error::cuda_details()` returns the payload, and
`None` for every other error. Context ids are `u64`.

`ErrorKind` is a fieldless `Copy` enum with 45 variants:

- one for each of the 44 non-base error classes Python and TypeScript share,
  with the `Error` suffix removed: 42 true leaves plus the parents `Vector` and
  `Embedder`;
- `Engine` for the base-class fallback.

`ErrorKind::ALL` lists all 45 variants in the order the Python binding
declares the classes: base first, then `RuntimeConfiguration`, `Storage`, and
so on through `ProjectionDestructive`, `CudaPoolExhausted`, `CudaContextLost`
and `CudaPrivateBuildRefused`.

`ErrorKind::parent()` returns:

- `Embedder` for `EmbedDevicePolicy`, `RerankerDevicePolicy`,
  `EmbedderNotConfigured`, `EmbedderRequired`, `CudaPoolExhausted`,
  `CudaContextLost`, and `CudaPrivateBuildRefused`;
- `Vector` for `KindNotVectorIndexed`;
- `None` otherwise.

`ErrorKind::name()` returns the shared class name, for example
`"StorageError"`. For the base, it returns Python's `"EngineError"`; the
TypeScript base is `FathomDbError`.

`Error::kind()` mirrors the Python mapping:

- **Core errors.** `engine_error_to_py`, `engine_open_error_to_py`, and
  `runtime_configuration_error_to_py` in `fathomdb-py/src/errors.rs`. The
  TypeScript mapping is equivalent.
- **Open errors:** `EngineConfiguration` → `InvalidArgument`; `Io` →
  `Storage`.
- **Core variants.** A core variant compiled only with `operator` →
  `ErrorKind::Engine`, as in Python, through an
  `#[allow(unreachable_patterns)]` catch-all. That arm would also absorb a
  future core variant, so a Rust test maps every non-operator core variant
  explicitly. The three CUDA core variants, and a `RerankerDevicePolicy` or
  open `Embedder` payload of a CUDA kind, have explicit arms ahead of it.

## Features

`default = []`, matching the `fathomdb` facade. A default feature would leak
into every workspace build through Cargo feature unification and change other
crates' tests. The crate forwards these features to the engine (and, for CLS,
to the embedder):

- `default-embedder`
- `default-reranker`
- `embed-cuda`
- `rerank-cuda`
- `embed-metal`
- `rerank-metal`

No feature forwards `operator`, `test-hooks`, or `tc5-benchmark`. The
behavior without features matches a wheel built without those features:

- **`use_default_embedder: true`** returns the core's typed `Embedder` open
  error.
- **`embed_batch_cls`** returns `EmbedderNotConfigured`.
- **`rerank`** takes the identity path.

The README states that `default-embedder` reproduces the shipped wheel and npm
behavior.

`tegra-pool` (0.8.28) is implied by this crate's `embed-cuda` and
`rerank-cuda`; naming it is accepted and redundant (0.8.28 Slice 31, owner
ruling Q37). It selects the Tegra private CUDA memory pool on aarch64 Linux
and compiles nothing on x86_64 (`dev/interfaces/rust.md` § CUDA private
memory pool; `dev/adr/ADR-0.8.28-tegra-private-cuda-pool.md`). It adds no
dependency: the pool primitive comes from `cudarc-fathomdb` through the
Candle fork crates on crates.io, so it also builds downstream of the
published crates.

An SDK process gets the private pool with no init call of its own (owner
rulings Q30 and Q36). It has no module-load hook; the decision accepts the
`cuInit` that the SDK's device probe makes when `Engine::open` (with
`use_default_embedder`), `embed_batch_cls` or `rerank` first uses CUDA. Its
allocator report says `module_load_init` `not_at_load` with `path` `private`
and `reason` `private_pool`. `FATHOMDB_CUDA_EARLY_INIT` has no effect here.
Two `fathomdb-embedder` constructors skip the probe and stay on the 0.8.27
allocator path, which is safe: the TC5 benchmark's
`CandleBgeEmbedder::new_from_local_asset_on_device_attested(.., ExplicitCandleDevice::Cuda(n))`
and `CandleBgeEmbedder::new_from_weights_with_device_resolution` with a
caller-built `DeviceResolution`. Neither is reachable from this crate.

The pool settings are `EngineConfig::cuda_pool` (`CudaPoolMode`),
`cuda_pool_max_size` (bytes, at least 32 MiB) and
`cuda_pool_release_threshold` (`ReleaseThreshold`) on `OpenOptions::config`,
read back through `Engine::config()`; the environment variables
`FATHOMDB_POOL_MODE`, `FATHOMDB_POOL_MAXSIZE` and
`FATHOMDB_POOL_RELEASE_THRESHOLD` apply only when no open sets the setting.
With threshold `max`, freed GPU memory stays reserved while any FathomDB
model is open in the process and is returned when the last one closes.

**One pool configuration per process.** The GPU pool settings are fixed once
per process: by the first open that passes any of them, or, if none does, by
the `FATHOMDB_POOL_*` environment variables (or their defaults) at the first
GPU use. A later open that sets one of them to a different value fails with
`ErrorKind::InvalidArgument` naming the setting and the value in force.
Closing every engine does not reset them. Opens that omit them never fail this
way. An open that sets any of them fixes the configuration on every platform,
including machines where the pool never engages (x86, discrete GPUs, CPU-only
engines). Whether an open that omits them fixes the configuration depends on
whether it builds a GPU device; set the same values on every open, or on none.
In practice: pick one set of values per process, or leave them unset; a test
binary that runs in one process must not open engines with different pool
settings. A `cuda_pool_max_size` larger than the GPU's memory cannot be
checked until the pool is created; the pool is then turned off, and the
allocator report's reason is `invalid_setting`.

The `cuda_allocator` field of `CudaDeviceInfo` is reachable through
`OpenReport`; this crate re-exports its report types
(`CudaAllocatorReport`, `CudaAllocatorPath`, `CudaAllocatorReason`,
`ReleaseThreshold`, `ModuleLoadInit`, `PoolSettingSource`) and the setting
types (`CudaPoolMode`, `CudaPoolSettingConflict`, `CudaPoolSettingValue`).
The
`cuda_libraries` field of `CudaDeviceInfo` (0.8.28 Slice 31) lists the CUDA
libraries the probe loaded; this crate re-exports `CudaLibraryReport`,
`CudaLibraryRefusal` and `CudaLibraryRefusalReason`. A refused library is the
device-policy error kind `cuda_library_unavailable`, carried by
`EmbedDevicePolicyError` / `RerankerDevicePolicyError` (`ErrorKind::EmbedDevicePolicy`
/ `ErrorKind::RerankerDevicePolicy`; `dev/interfaces/rust.md` § CUDA library
check and GPU floor). The
standalone `embed_batch_cls` and `rerank` models are process singletons:
on the private pool they hold their memory until the process exits.

## Request-struct defaults

Core request structs are constructed in full by Rust callers. Use the values
the Python and TypeScript SDKs default to: `EvidenceSearchRequestV1`
`{schema_version: 1, rerank_depth: 0, use_graph_arm: false, alpha: 0.3,
pool_n: 0, include_explanation: false, limit: 10}`;
`DependencyTraceRequestV1::new(..)` already applies the binding bounds
(`max_relations` 100, `max_work_units` 101) when `with_bounds` is not called.

## SQLite runtime configuration

SQLite page cache (0.8.28 Slice 40). Artifacts built from this repository
(the Python wheel, the npm addon and a CLI built from this workspace) build
the bundled SQLite without
`SQLITE_ENABLE_MEMORY_MANAGEMENT`, so each SQLite connection keeps its own
page cache of SQLite's default 2,000 KiB. A Rust build that resolves the
registry `libsqlite3-sys` keeps SQLite's shared page cache, where one large
read can slow later reads until the engine is reopened. rusqlite removed the
flag upstream (commit `f31f7324`); Rust consumers get the fix from the first
`libsqlite3-sys` release that contains it, and until then
`LIBSQLITE3_FLAGS=-USQLITE_ENABLE_MEMORY_MANAGEMENT` at build time gives the
same behaviour as the workspace build. `cargo install fathomdb-cli` is such a
registry build: it keeps the shared page cache until a `libsqlite3-sys` release
contains `f31f7324`, unless `LIBSQLITE3_FLAGS=-USQLITE_ENABLE_MEMORY_MANAGEMENT`
is set for the install.

## Search behaviour in 0.8.28 (Slice 40)

The signatures are unchanged; the behaviour of three search verbs changed.

- **Query compilation** (`Engine::search_text_only`, and the text branch of `Engine::search`):
  the query is lowercased, split on non-alphanumeric characters, de-duplicated
  in first-seen order and OR-joined. Tokens of every length are kept, so "C",
  "D3", "II" and the "1" of "IGF-1" are query terms. A fixed English stop list
  is dropped. It now also holds the fragments an apostrophe leaves (`s`, `t`,
  `ll`, `re`, `ve`, `m`) and `my`, `me`, `us`, `am`, `so`, `up`. A query made
  only of stop words falls back to its raw whitespace tokens.
- **Fusion** (`Engine::search_text_only`, `Engine::search`): each branch credits a body once, at
  the rank of its first occurrence, so several rows sharing a byte-identical
  body no longer outrank the best match. A body found by more than one branch
  still accumulates one term per branch:
  `score = Σ_branch w_branch / (30 + rank_branch)`, with `w` 3 for the text
  branch and 1 for the vector and graph branches.
- **`Engine::search_projected_text`**: the projection's FTS5 index is read in rank order and
  reading stops once `limit` rows and any rows tied with the last one are
  read, instead of sorting every match. Results and their order are unchanged:
  bm25 ascending, then write order.
  With an attribute filter whose smallest attribute cohort holds at most 256
  rows, the rank stream skips rows outside that cohort before reading them;
  results, scores and order are the same.
- **Dense retrieval in `Engine::search`**: the vector branch needs a declared
  `SEARCHABLE` projection with `vector: Some(..)` (`Engine::configure_projections`) before the rows are
  written; that declaration enrols the node kind for embedding. Opening with
  `OpenOptions { use_default_embedder: true, .. }` alone embeds nothing, and `Engine::search` then returns text hits
  only. The engine embeds each node's full body, not the projected attribute.
- **Vector pool in `Engine::search`**: the vector branch fuses at least its ten nearest
  surviving rows, whatever the limit. This changes only calls with a limit
  below 10; limits of 10 and above rank as before. For one database state, with
  rerank depth 0 and no graph arm, a search with limit `k` returns the first `k` results of the same
  search with limit `k2` for every `k < k2 <= 10`, with bit-identical scores. Above 10 this is not
  guaranteed.

## Context pack (0.8.28 Slice 41)

The behaviour, the types and the round-body convention are shared with
Python and TypeScript; [`python.md`](python.md) § Context pack is the full
contract. The Rust spellings are `ContextPart`, `ContextItem`,
`ContextPolicy` (`Default` = the product defaults), `ContextOrder::{Rank,
Chronological}`, `ContextPack`, `ContextPackEntry`, `ContextPartEntry`,
`ContextSource::{Hit, Context}` and `ContextTier::{Full, Compact, Omitted}`.
`position` is `Option<u64>`; counts are `usize`, so negative counts cannot
be expressed. Characters are Unicode scalar values (`str::chars`). The
shared conformance fixture is `src/conformance/context-pack-fixtures.json`.
