---
title: FathomDB 0.8.27 Slice 132 — dedicated Rust SDK design
status: REVIEWED
target_release: 0.8.27
planning_baseline: 316ac4769
---

# Slice 132 — dedicated Rust SDK design

## Boundary

The new workspace crate `src/rust/crates/fathomdb-sdk` (package
`fathomdb-sdk`, library `fathomdb_sdk`) is the Rust peer of the Python
`fathomdb` package and the TypeScript `fathomdb` package. It depends
non-optionally on `fathomdb-engine`, `fathomdb-schema`, `fathomdb-embedder`,
and `fathomdb-embedder-api`. The last three own types that appear in
`OpenReport` and the open errors, and the embedder crates own CLS embedding.
Its public `lib.rs` is a closed export list:

- **Root:** `Engine`, `EngineConfig`, `OpenOptions`, the option structs below,
  `Error`, `ErrorKind`, `Result`, `rerank`, `embed_batch_cls`,
  `RerankPassage`, `RerankOptions`, `RerankResult`, and the selected DTO
  re-exports.
- **Modules:** `read`, `graph`, `admin`.

The SDK `Engine` owns a private `fathomdb_engine::Engine`, the `OpenReport`,
the effective `EngineConfig`, and the current subscriber attachment. It has
no `Deref`, no `inner()`, no `From<core::Engine>`, and no public core field.
The SDK never re-exports `fathomdb_engine::Engine`, `OpenedEngine`,
`EmbedderChoice`, the `Embedder` trait, operator types, or test hooks.

Adapters are thin: they validate only what the Python and TypeScript bindings
validate before calling the core, then call the same core function that both
native bindings call (listed below). Database semantics stay in the engine.
The existing `fathomdb` crate is unchanged. It stays the lower-level engine
facade governed by the BIND-RUST allowlist.

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
| `engine.erase_source` | `erase_source(&self, source_id: &str) -> Result<EraseReport>` | `erase_source` |
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
| `rerank` | `rerank(query: &str, passages: &[RerankPassage], rerank_depth: usize, options: RerankOptions) -> Result<Vec<RerankResult>>`. Takes `RerankPassage { id: u64, body: String, score: f64 }` and returns `RerankResult { id: u64, score: f64, ce_score: Option<f64> }`. A non-finite `alpha` is `InvalidArgument`, matching TypeScript. A non-finite passage `score` or a core `Err(String)` is `WriteValidation`, following Python and napi. This is a deliberate exception to rule 6, because the TypeScript wrapper's `RangeError` is a host-side check. | `fathomdb_engine::rerank_passages` |
| — | `embed_batch_cls(texts: &[&str]) -> Result<Vec<Vec<f32>>>`. With `default-embedder`: empty input returns `[]`; the embedder is a process singleton cached only on success; a load failure is `EmbedderNotConfigured`, and an embed failure is `Embedder`. Without the feature, every call, including one with empty input, returns `EmbedderNotConfigured`. | `CandleBgeEmbedder::new()?.with_pooling(Pooling::Cls)` then `Embedder::embed_batch` |

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
     TypeScript export, under their core names

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
}
pub type Result<T> = std::result::Result<T, Error>;
```

`Error` implements `Display`, `std::error::Error` (with `source`), and `From`
for the three core error types. The core error enums are re-exported so
callers can still read typed payloads such as `holder_pid` and `legal`.
`Sdk` holds errors raised by the SDK itself or by a component outside the
engine, such as a rerank `Err(String)` or a CLS embedder failure.

`ErrorKind` is a fieldless `Copy` enum with 42 variants:

- one for each of the 41 non-base error classes Python and TypeScript share,
  with the `Error` suffix removed: 39 true leaves plus the parents `Vector` and
  `Embedder`;
- `Engine` for the base-class fallback.

`ErrorKind::ALL` lists all 42 variants in the order the Python binding
declares the classes: base first, then `RuntimeConfiguration`, `Storage`, and
so on through `ProjectionDestructive`.

`ErrorKind::parent()` returns:

- `Embedder` for `EmbedDevicePolicy`, `RerankerDevicePolicy`,
  `EmbedderNotConfigured`, and `EmbedderRequired`;
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
  explicitly.

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

## Governance and checks

- **Operation map.** `src/conformance/governed-operation-parity.json` gains a
  `rust` endpoint on every operation:
  - Rust locators reuse the existing set (`engine_static`, `engine_instance`,
    `read`, `graph`, `admin`, `package`).
  - Spellings equal the Python spellings.
  - The signed allowlist and its pin are unchanged.
- **Checker.** `scripts/check-sdk-surface-parity.py` adds `rust` to
  `BINDINGS`. It also gains `--rust-crate <dir>`, which observes the Rust
  surface from source. The observation covers:
  - `pub fn` items in `impl Engine` in the crate's `engine` module;
  - `pub fn` items in `read.rs`, `graph.rs`, and `admin.rs`;
  - root `pub fn` items re-exported by `lib.rs`.

  The scan covers every `impl Engine` block under `src/`, ignores
  `pub(crate)` items and `#[cfg(test)]` modules, and maps `open` to
  `engine_static`. It applies its own `RUST_NON_COMMAND` exclusion
  constant:
  - the shared non-command members listed above;
  - `open_report`, `config`, and `attach_subscriber`;
  - `admin::configure_runtime` and `embed_batch_cls`.

  The scan matches `pub [const|async|unsafe] fn name`. It also refuses
  type-level routes to the core that a function scan cannot see:
  - an `impl <Trait> for Engine` other than `Drop` or `Debug`, which rules out
    `Deref`, `AsRef`, `From`, and `Borrow`;
  - a public field on the SDK `Engine`;
  - a glob `pub use fathomdb_engine::*`, or a re-export naming the core
    `Engine`, `OpenedEngine`, or `EmbedderChoice`;
  - a `pub mod` in `lib.rs` other than `read`, `graph`, `admin`, or an empty
    doc module.

  Until Slice 150's rustdoc probe, re-exported data types are pinned only by
  the consumer test.
  `scripts/tests/test_check_sdk_surface_parity.py` adds `rust` endpoints to
  its `COMPANION` fixture. It also gains a real-crate case expecting
  `rust 44/44`, plus missing, extra, and renamed Rust mutation cases. That
  test already runs in the `agent-test.sh` fast tier.
- **Absence proofs.** `compile_fail` doctests on a doc-hidden module, as in
  the `fathomdb` facade, prove that each of these does not resolve:
  - `fathomdb_sdk::EmbedderChoice`
  - `Engine::open_with_choice`
  - `engine.read_get(..)`
  - `engine.check_integrity()`
  - `engine.execute_for_test(..)`
  - `fathomdb_sdk::OpenedEngine`
- **Hidden-surface probe.** Deferred to Slice 150, which owns release
  surface capture and needs a new `ROWS` entry for it.

## Release wiring

The new crate must be added to each list below:

- **Workspace and dependencies:** workspace `members`, plus a
  `workspace.dependencies` entry (path + version).
- **Version axes (Axis W):** `scripts/set-version.sh` (both lists),
  `scripts/tests/test_set_version.sh`, and
  `scripts/tests/test_release_version_surfaces.sh`.
- **Publication lists:**
  - `scripts/verify-release-gates.sh` `PUBLISHABLE_CRATES`;
  - `scripts/release/cargo-publish-if-new.sh` dependent-crate case, plus its
    header comment;
  - a `release.yml` publish and wait step after `fathomdb`.

  It is not added to `local-dry-run.sh` `LEAVES`, which holds only crates
  without workspace dependencies.
- **Licensing:** a byte-identical `LICENSE` in the crate root. Update the crate
  count in `scripts/check-license-consistency.sh`'s comment.
- **Test matrix:** `scripts/test-feature-matrix.toml`, regenerated with
  `scripts/lib/test_targets.py --write-matrix`.
- **Docs:**
  - a crate README;
  - a `docs/` page for the Rust SDK, with a `mkdocs.yml` nav entry;
  - an entry in `CHANGELOG.md`;
  - cross-references in `dev/design/bindings.md` and
    `dev/interfaces/README.md`.

## Verification design

The behavior tests (AC27-132D) open real databases in temporary directories
and use no mock. They assert the documented Python/TypeScript values directly.
Feature-dependent tests run without features. A test needing the real
embedder or reranker is marked `#[ignore]` with its reason, or is gated on the
feature, following the engine's existing pattern. The development loop is:

1. `cargo test -p fathomdb-sdk`
2. the checker test
3. `cargo clippy -p fathomdb-sdk --all-targets -D warnings`
4. strict `./scripts/agent-verify.sh` once, on the candidate
