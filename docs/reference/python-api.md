# Python API

Module: `fathomdb`. Authoritative spec:
[`dev/interfaces/python.md`](https://github.com/fathomadb/fathomdb/blob/release/0.8.31/dev/interfaces/python.md).

> **Documentation target: 0.8.32 (pre-publish, pre-1.0 beta); newest published release: 0.8.31.**

## Representative top-level imports

The package exports the core symbols below plus the versioned 0.8.25 carrier
families summarized in [0.8.25 data-plane additions](#0825-data-plane-additions).

```python
from fathomdb import (
    DenseReadiness,
    EndpointValidationMode,
    Engine,
    EngineConfig,
    Filter,
    IdSpace,
    NodeRecord,
    OpStoreRow,
    ProjectionDelta,
    ProjectionRole,
    ProjectionRuntimeStatus,
    ProjectionRuntimeStatusEntry,
    ProjectionRuntimeUnavailabilityReason,
    ProjectionSpec,
    ProjectionStatusDenseReadiness,
    ReadView,
    ScopePolicy,
    ScopeViolationError,
    SearchFilter,
    SearchHit,
    SearchResult,
    SoftFallback,
    SoftFallbackBranch,
    WriteOptions,
    WriteReceipt,
    CounterSnapshot,
    admin,
    errors,
    graph,
    read,
)
```

## `release_models() -> None`

`from fathomdb import release_models` releases the process module-level CLS
embedder and cross-encoder caches before returning. Empty and repeated calls
succeed without loading weights or initializing CUDA. Loading, cache waits,
inference, or another release causes immediate `OverloadedError`; neither cache
changes on refusal. Python releases the GIL for native model work and teardown.
Healthy later calls reload on demand. Engine-owned Mean providers remain open;
terminal CUDA generation loss still requires process restart. See
[model lifetime and examples](../embedder.md#release-module-models).

## `Engine`

### `Engine.open(path, *, config=None, use_default_embedder=False, **engine_config) -> Engine`

Open or create a FathomDB database at `path`. Returns the engine
handle.

- `path` (`str`) — path to the SQLite DB file.
- `config` (`EngineConfig | None`) — pre-built config object.
- `use_default_embedder` (`bool`, default `False`) — `True` enables the pinned
  `fathomdb-bge-small-en-v1.5` embedder. First use can download weights into
  `~/.cache/fathomdb/embedders/`. `False` opens without an embedder; vector
  writes and `engine.embed` raise `EmbedderNotConfiguredError`. See the
  [embedder guide](../embedder.md) for runtime policy and typed failures.
- `**engine_config` — per-knob keyword arguments (see
  [config](config.md)), including `scope_policy: ScopePolicy | None = None`
  (defaults to `ScopePolicy.unscoped()` for single-scope/single-agent
  databases; opt into `ScopePolicy` for multi-agent stores). Mutually exclusive
  with `config`.

Raises `EngineError` subclasses on failure: `DatabaseLockedError`,
`DatabaseNotFoundError` (when `create_if_missing=False` on a missing or zero-length path),
`CorruptionError`, `IncompatibleSchemaVersionError`,
`MigrationError`, `EmbedderIdentityMismatchError`,
`EmbedderDimensionMismatchError`. See [errors](errors.md).

The structured open report is available after open via
`engine.open_report()` (below).

### `engine.open_report() -> OpenReport`

The structured open report defined in `dev/design/engine.md`:
`schema_version_before`, `schema_version_after`, `migration_steps`,
embedder identity confirmation, open-stage data, `stop_words_sha256`,
capability booleans (`reranker_compiled`, `cuda_compiled`, `rerank_available`),
and the embedder telemetry fields (`embedder_download_ms`, `embedder_events`,
`embedder_mean_centering_required`, `embedder_mean_vec_pinned`).

Numerical mismatch or inability to verify fails open without an Engine; there
is no dense-disabled report mode. See [compatibility errors](errors.md) and
[provider verification](../embedder.md#cross-backend-discipline).

### `engine.write(batch=None) -> WriteReceipt`

Write a batch of canonical rows synchronously through the primary writer
connection; return its committed receipt.

- `batch` (`list[Any] | None`) — non-empty list of caller-shaped canonical
  or operational rows. Passing `None` or an empty list (`[]`) raises
  `WriteValidationError`.

Every **node** item accepts these keys:

| Key | Type | Required | Meaning |
| --- | ---- | -------- | ------- |
| `kind` | `str` | **yes** | record kind |
| `body` | `str` | **yes** | record body |
| `source_id` | `str` | **yes** | provenance — see below |
| `logical_id` | `str` | no | governed cross-re-ingestion identity |
| `state` | `str` | no | create-time existence state: `"active"` (default) or `"pending"` |
| `reason` | `str` | no | advisory cause for `state`; stored verbatim, never interpreted |
| `valid_from` | `int` | no | world-time window, INCLUSIVE lower bound, epoch **seconds** UTC |
| `valid_until` | `int` | no | world-time window, EXCLUSIVE upper bound, epoch **seconds** UTC |
| `properties` | `dict[str, Any]` | no | optional inline JSON object of structured properties outside `body` |

An **edge** item takes `kind`, `from`, `to`, the same mandatory
`source_id`, an optional `logical_id`, optional inline `properties`
(`dict[str, Any] | None`), and the temporal pair
`t_valid` / `t_invalid` (`int | None`, epoch **seconds** UTC; `None`
means "still valid").

**`source_id` is MANDATORY (0.8.20).** `erase_source` addresses rows
*by* `source_id`, so a row written without one is reachable by no
erasure call. A missing, empty, whitespace-only or **reserved**
(`_`-prefixed) value raises `WriteValidationError`. Treat it as a
public identifier: use an opaque document or tenant id, never personal
data — see [Erasure](../operations/erasure.md).

**`logical_id`** — supplying it makes the write a transaction-time
**supersession** of the prior active version of that `logical_id`: the
prior version is tombstoned and the new version becomes active
(invalidate-not-delete). Active-row identity is scoped to `logical_id`
alone, so re-ingesting the same `logical_id` with a different `kind`
supersedes (it does not create a second active row). Omitting it is a
plain insert with a NULL `logical_id`, which never collides with other
NULL rows.

**Validity window** — half-open `[valid_from, valid_until)`. Omitting
both binds NULL/NULL (unbounded, the pre-0.8.20 default). Both bounds
present with `valid_from >= valid_until` is unsatisfiable and raises
`WriteValidationError`; validation runs before any insert, so the whole
batch is rejected. A one-sided window is never refused. A non-integer
bound raises `WriteValidationError` and is never coerced (`bool` is
rejected explicitly).

```python
receipt = engine.write([
    {"kind": "note", "body": "hello", "source_id": "doc-42"},
    {"kind": "note", "body": "governed", "source_id": "doc-42",
     "logical_id": "note:hello"},
    {"kind": "entity", "body": "Acme Corp", "source_id": "doc-42",
     "logical_id": "acme"},
    {"kind": "mentions", "from": "note:hello", "to": "acme",
     "source_id": "doc-42", "t_valid": 1_546_300_800, "t_invalid": None},
])
```

- Atomicity and endpoint integrity: a batch executes in a single SQLite
  transaction even when items carry distinct `source_id` values. Every edge in
  the batch (including edges superseded later in the same batch) must reference
  active, non-superseded endpoint nodes (`state == "active"` and
  `superseded_at IS NULL`); same-batch forward references and active self-loops
  are valid. Missing, superseded-only, `pending`, or `deleted` endpoints raise
  `WriteValidationError` and roll back the entire batch atomically.
- Returns: `WriteReceipt(cursor: int, row_cursors: tuple[int, ...],
  dangling_edge_endpoints: int)`. `cursor` advances monotonically across
  writes (the batch high-water cursor); `row_cursors` are the per-row
  `write_cursor`s, 1:1 with the input batch order;
  `dangling_edge_endpoints` is `0` on every committed strict batch — see
  [`WriteReceipt`](#writereceipt).

### `engine.write_with_options(batch=None, options=None) -> WriteReceipt`

Write a batch under explicit `WriteOptions(endpoint_validation=...)` (`0.8.32`):

- `options` (`WriteOptions | None`, default `None`) — `None` or
  `WriteOptions(endpoint_validation="strict")` enforces strict active-endpoint
  validation identical to `engine.write`. Passing a non-`WriteOptions` value or
  non-`str` mode raises `TypeError`; an unknown mode literal raises `ValueError`.
- Opting into `WriteOptions(endpoint_validation="bulk_non_strict")` admits
  out-of-order same-scope edges and reports the exact count of unresolved
  endpoints across surviving active edges written by the batch in
  `WriteReceipt.dangling_edge_endpoints` (`0`, `1`, or `2` per edge; `0` for
  node-only batches). Subsequent same-scope batches supplying the missing nodes
  resolve traversal automatically without rewriting edges.
- Cross-scope active endpoints and read-only `ScopePolicy` handles still roll
  back atomically with `ScopeViolationError`, and all other batch invariants
  (validity windows, typed `properties`, mandatory `source_id`, `state`,
  provenance, triggers, erasure) remain strict. Verify post-import completeness
  read-only with `fathomdb doctor data-plane-integrity --check edge_endpoint_integrity`
  or `fathomdb doctor check-integrity`.

### `engine.search(query, filter=None, *, rerank_depth=0, use_graph_arm=False, alpha=None, pool_n=None, explain=False, view=None, limit=10, snippet=None, graph_seed="graph_fts", graph_seed_k=10, graph_edge_kinds=None, graph_edge_predicates=None, hints=False) -> SearchResult`

Run hybrid retrieval (FTS5 + vector) for `query`, ranked by **G9 RRF fusion**,
with optional cross-encoder reranking (0.8.1 R1) and optional graph-BFS third
arm (0.8.1 R3).

- `query` (`str`).
- `filter` ([`SearchFilter`](#searchfilter) | `Filter` | `None`) — optional closed
  metadata filter. `Filter` accepts the unified metadata terms and lowers to
  `SearchFilter`; a `Json` term raises `InvalidFilterError` because search must
  not apply a JSON predicate after vector candidate selection. `None` (or an
  all-`None` filter) is the unfiltered path.
- `rerank_depth` (`int`, default `0`) — 0.8.1 R1 opt-in. `0` (default) uses the
  identity / soft-fallback path: byte-identical to the pre-0.8.1 fused order.
  `N > 0` applies a cross-encoder (TinyBERT-L-2, ≈4 MB; latency depends on
  the selected CPU/CUDA runtime and workload)
  over the top-N fused hits using score-blend (α=0.3 × CE + 0.7 × RRF-norm).
  Must be a non-negative integer; negative values raise `ValueError`. In the
  default build (no `default-reranker` feature), depth > 0 returns the identity
  order (model absent → soft-fallback).
- `use_graph_arm` (`bool`, default `False`) — opt-in graph-BFS third RRF arm
  (`RRF_WEIGHT_GRAPH = 1.0`, depth ≤ 3, cap 50) over active, temporally valid
  canonical edges. Depth-0 seed selection is controlled by `graph_seed`
  (`"graph_fts"` default, `"fused_top_k"`, or `"both"`). Default `False`
  produces byte-identical two-arm results with zero added graph SQL. Must be a
  `bool`; non-bool raises `TypeError`.
- `graph_seed` (`GraphSeedMode`, default `"graph_fts"`) — selects depth-0 graph
  seeds when `use_graph_arm=True`: `"graph_fts"` (edge-body and entity-node
  FTS), `"fused_top_k"` (top-`graph_seed_k` canonical-node hits from the
  two-arm fused/reranked ranking, enabling body-less property-graph edge
  traversal), or `"both"` (deterministic min-hop merge with `(fused_top_k,
  edge_fts, entity_fts)` tie-breaking). Available on `search`, `search_frozen`,
  and `EvidenceSearchRequestV1` (not `search_text_only`).
- `graph_seed_k` (`int`, default `10`) — maximum unique canonical-node seeds
  taken under `"fused_top_k"` or `"both"` (`1..=100`; outside range raises
  `InvalidArgumentError`).
- `graph_edge_kinds` (`Sequence[str] | None`, default `None`) — optional
  edge-kind allowlist for graph-arm seeding and traversal.
- `graph_edge_predicates` (`Sequence[PropertyPredicate] | None`, default `None`)
  — optional closed typed scalar predicates (implicit `AND`, at most `32`)
  evaluated against inline edge `properties`.
- `alpha` (`float | None`, default `None`) — 0.8.5 (EXP-0) CE-blend weight,
  clamped to `[0, 1]` in the engine. `None` ⇒ `0.3` — the **C6 factoid-guard**
  default that prevents a high-CE-but-wrong candidate from displacing a
  BM25-correct factoid. Only effective when `rerank_depth > 0` and the CE model
  is loaded.
- `pool_n` (`int | None`, default `None`) — 0.8.5 (EXP-0) reranked-pool size,
  clamped to the hit count. `None` ⇒ `rerank_depth` (preserves the prior
  pool == depth semantics). Note `rerank_depth == 0` is still the identity gate,
  so `rerank_depth=0, pool_n=10` does **not** rerank.
- `explain` (`bool`, default `False`) — `True` includes retrieval explanation
  data in `SearchResult.explanation` (including `PerHitExplain.graph_seed_source`
  and `PerHitExplain.graph_hop_depth` for `"graph_arm"` hits); non-boolean
  values raise `TypeError`.
- `view` (`ReadView | None`, default `None`) — controls validity at query time
  or an explicit `valid_as_of` instant. `include_out_of_window=True` widens
  validity windows. `include_superseded` and `include_inactive` are independent
  opt-ins; validity, dependency and mandatory edge-expiry restrictions still
  apply. Incomplete historical dense coverage uses the defined lexical fallback
  rather than silently returning an incomplete dense answer.
- `limit` (`int`, default `10`) — maximum ranked hits returned. It must be in
  `1..=100`; zero, negative values, and values above 100 raise
  `InvalidArgumentError` and are never silently clamped.
- `hints` (`bool`, default `False`) — opt-in agent search hints populated on
  `SearchResult.hints` without altering hit rankings (zero overhead when
  `False`; non-`bool` raises `TypeError`).
- Returns: `SearchResult(projection_cursor: int, soft_fallback:
  SoftFallback | None, results: list[SearchHit], hints: list[Hint] | None)`. Each
  [`SearchHit`](#searchhit) carries the matched record's `id`, `kind`,
  `body`, optional inline `properties`, the **RRF-fused** `score`, the `branch` that produced it
  (`"graph_arm"` for nodes surfaced only via graph traversal), and `ce_score`
  (the per-candidate CE score for in-pool reranked hits, `None` otherwise).

> **Ranking is RRF (behavior-compat event).** Results are ordered by Reciprocal
> Rank Fusion (`Σ w/(30 + rank)`, text weight 3, vector weight 1) of the vector and text branches — a body the
> two branches agree on ranks above one only a single branch found. This is the
> deliberate, documented 0.8.0 ranking change; pre-0.8.0 union-dedup ordering is
> not retained. See [hybrid search guide](../guides/hybrid-search-filtering.md).

### `engine.search_text_only(query, view=None, *, limit=10, filter=None) -> SearchResult`

Run direct FTS retrieval without embedding, vector retrieval, CE reranking, or
graph expansion. Matching node- and edge-body candidates are deterministically
body-deduplicated and ranked before `limit` is applied. The node candidate input
is fixed at 100, so for the same immutable selection, query, filter, and effective
validity time, a smaller accepted limit returns the ordered prefix of a larger
one. Compare `view=ReadView(valid_as_of=...)` calls only at the same explicit
instant; an omitted `valid_as_of` resolves per call. This guarantee does not
extend to hybrid `engine.search`.

`filter` is keyword-only and accepts `SearchFilter | None`. Existing positional
`view` calls remain valid. Predicates apply to snapshot-local node/edge body
FTS before candidate caps, deduplication, ranking and limit. Attributes are
node-scoped ordered AND equality over declared filterable projections. Kind/
source type and available attributes work without a provider. Status/time require
existing vector metadata: projected status is the empty-string sentinel and
`created_after` / `createdAfter` is inclusive projection time. Missing metadata
does not match. This is body FTS, with no vector, reranker or graph work. See
[filter guide](../guides/hybrid-search-filtering.md).

### `engine.search_projected_text(query, name, filter=None, *, view=None, limit=10) -> SearchResult`

Search exactly one declared `SEARCHABLE` property-FTS projection. `limit` has
the same `1..=100` validation and default of 10; metadata and validity filters
are applied before retained hits consume that result budget.

### `engine.embed(text: str) -> list[float]`

Embed `text` with the engine's pinned default embedder
(`fathomdb-bge-small-en-v1.5`) and return the raw vector. Read-path
primitive for callers that need vectors under the engine's **own**
embedder identity (e.g. coverage-index clustering) rather than a
parallel, possibly-divergent embedder. Raises
`EmbedderNotConfiguredError` if the engine was opened without an
embedder (`use_default_embedder=False`). Mirrored in TS as
`engine.embed(text)`.

### `engine.transition(logical_id, to_state, reason=None) -> None`

**0.8.19.** Move a **governed** node between existence states, per the
engine-enforced legal-transition table:

| From | To | Effect |
| ---- | -- | ------ |
| `pending` | `active` | promote (clears `reason`) |
| `pending` | `deleted` | **rejected** |
| `active` | `deleted` | soft-delete (sets `reason`) |
| `deleted` | `active` | undelete (clears `reason`) |

`reason` is advisory and never interpreted by the engine. Keys on the
bare `logical_id` — the `logical` (`l:`) id space only; a `content`
(`h:`) or `passage` (`p:`) id raises `NotLifecycleAddressableError`. An
illegal move (a `purged`/`pending` target, a self-loop, an absent node)
raises `IllegalTransitionError` carrying `from_state`, `to_state` and
`legal`.

### `engine.purge(logical_id: str) -> None`

**0.8.19.** Irreversibly hard-erase a governed node across every
row-owned target — all versions, its FTS/vector shadows, and its
touching edges (cascade-removed).

**Deleted-first:** legal only from `deleted`, otherwise
`IllegalTransitionError`. **Idempotent:** purging an absent or
already-purged id is a no-op success. A non-`l:` id raises
`NotLifecycleAddressableError`.

`purge` addresses one **governed** node. For anonymous content — rows
written with no `logical_id` — use `erase_source` below.

### `engine.erase_source(source_id: str) -> EraseReport`

**0.8.20.** Erase every canonical row carrying `source_id`, together with its
row-owned projections (FTS5, `vec0`, `search_index_v2`), then finish the erasure
at rest — redact the erased ids from the telemetry sink and truncate the WAL.

The **companion to `purge`, not a duplicate of it.** `purge` addresses a
*governed* node by `logical_id`; `erase_source` addresses *anonymous* content —
rows written with no `logical_id`, which `purge` cannot reach at all. Together
they make every canonical row erasable from the SDK alone, with no CLI on
`PATH`.

Idempotent: erasing an absent or already-erased source is a zero-count success,
so an interrupted erasure obligation can be retried without a pre-check.

Raises `WriteValidationError` for an empty, whitespace-only, or **reserved**
(`_`-prefixed) `source_id`. The engine's reserved namespace (`_engine:*`
substrate and the `_legacy:pre-0.8.20` migration cohort) is reachable only
through `fathomdb recover --excise-source`. Raises `ErasureIncompleteError`
(carrying `stage` and `detail`) rather than reporting success if the erasure
could not be completed at rest.
The same source identity accepted by a write can be erased, including one with
an embedded NUL; the call matches the full identity exactly.

Returns an `EraseReport` with `source_ref`, `nodes_excised`, `edges_excised`,
and `projections_invalidated`. Mirrored in TS as `engine.eraseSource(sourceId)`.

See [Erasure](../operations/erasure.md) for what this does and does **not**
guarantee, and for the non-PII `source_id` rule.

### `engine.close() -> None`

Release SQLite handles, join the writer thread, drain the scheduler,
release the on-disk lock. Idempotent.

### `engine.drain(*, timeout_s=0) -> None`

Block until in-flight writes drain or `timeout_s` elapses. Argument
unit is **seconds** (TS counterpart uses milliseconds).

### `engine.counters() -> CounterSnapshot`

Snapshot of engine-internal counters. See
[`CounterSnapshot`](#countersnapshot) below.

### `engine.set_profiling(*, enabled: bool) -> None`

Toggle per-operation profiling.

### `engine.set_slow_threshold_ms(*, value: int) -> None`

Set the slow-query threshold for profiling event emission.

### `engine.attach_logging_subscriber(logger) -> None`

Bind engine events into a Python `logging.Logger`. Engine events are
mapped to `logging.LogRecord` with the stable `fathomdb` payload. The logger
is caller-owned. Delivery uses a bounded queue: a slow logger can lose
records, and a `dropped_records` warning reports the count when it catches
up. Delivery and phase pairing are preserved below capacity while the logger
is live and functioning. Logger failures do not fail database calls. Logger handlers cannot
reenter FathomDB database operations. This helper does not emit heartbeats.

### Properties

- `engine.path` (`str`) — DB path supplied to `open`.
- `engine.config` (`EngineConfig`) — requested configuration captured at open;
  omitted fields remain omitted even when the engine uses their defaults.

## 0.8.25 data-plane additions

The additions below are available in the published 0.8.25 wheel. Request and
response objects use `schema_version=1`; new counters, cursors, generations,
and boundaries that cross the SDK boundary use canonical unsigned decimal
strings rather than JSON numbers.

### Versioned provenance

A node or edge write may include a closed `provenance` mapping. Python uses
snake-case fields: `artifact_revision_id`, `source_version_id`,
`source_revision_id`, `source_locator`, and `canonical_source_hash`.
`source_locator` is either whole-body or an exact UTF-8 byte span. Malformed or
unknown fields raise `ProvenanceError(reason, field_path)` before database
access. Existing writes without the versioned mapping remain valid.

The public carrier family is `CanonicalHash`, `WholeBodySourceLocator`,
`Utf8BytesSourceLocator`, `SourceLocator`, `CanonicalWriteProvenanceV1`,
`DerivedWriteProvenanceV1`, and `WriteProvenanceV1`.

### Dependencies, actuation, and closure

| Method | Result | Contract |
| ------ | ------ | -------- |
| `engine.register_source_dependency(request)` | `SourceDependencyV1` | Register or exactly replay one immutable source-to-derived relation. |
| `engine.dependencies_for_source(request)` | `DependencyListV1` | Return at most 100 dependencies in stable order. |
| `engine.dependency_for_derived(request)` | `SourceDependencyV1 \| None` | Resolve the single source dependency for a derived revision. |
| `engine.actuate(request)` | `ActuationReceiptV1` | Atomically apply 1–128 caller-decided operations with idempotent replay. |
| `engine.read_dependency_closure(request)` | `ClosureStatusV1 \| None` | Read one Engine-minted closure operation; there is no public list or resume verb. |

Dependency requests use `SourceDependencyRegistrationV1`,
`DependencySourceLookupV1`, and `DependencyDerivedLookupV1`. Actuation uses
`ActuationBatchV1` / `ActuationOperationV1`; its operation set is canonical
node put, derived node put, derived edge put, dependency registration, and
lifecycle transition. Derived edges require both endpoints in the final active
state of the complete batch, even when their node puts occur later.
Domain refusals are terminal receipts while malformed requests raise
`ActuationError`. Closure reads use `ClosureLookupV1`, `ClosureStatusV1`, and
`ClosureProofV1`.

### Frozen reads, pages, evidence, and trace

| Method | Result | Contract |
| ------ | ------ | -------- |
| `engine.freeze_read_context(context)` | `FrozenReadContextV1` | Authenticate a resolved `ReadContextV1` for this database. |
| `engine.search_frozen(query, context, **options)` | `SearchResult` | Search without weakening the frozen validity or eligibility policy. |
| `engine.search_expand_frozen(query, context, depth, *, limit=10)` | `SearchExpandResult` | Search and graph-expand in one frozen read context. |
| `engine.search_with_evidence(request)` | `EvidenceSearchResultV1` | Return normal hits plus one positional evidence reference per hit. |
| `engine.resolve_evidence(request)` | `ResolvedEvidenceV1` | Resolve exact canonical bytes and the selected UTF-8 span. |
| `engine.resolve_graph_evidence(request)` | `ResolvedGraphEvidenceV1` | Resolve an exact target or terminal edge disclosed by frozen graph expansion. |
| `engine.trace_dependency(request)` | `DependencyTraceResultV1` | Perform a bounded reciprocal dependency trace under a frozen context. |
| `read.canonical_page(engine, kind, context, page)` | `PageV1[NodeRecord]` | Read a stable page of canonical logical nodes. |
| `read.operational_state(engine, collection, record_key, context=None)` | `OperationalStateRecordV1 \| None` | Point-read a registered `latest_state` collection. |
| `read.operational_state_page(engine, collection, context, page)` | `PageV1[OperationalStateRecordV1]` | Read a stable operational-state page. |

For `search_frozen`, omitted `pool_n` now uses `rerank_depth`, matching ordinary
search and the other SDKs. Previously the Python frozen default was zero, so
this can change ranking when a caller sets a positive rerank depth.

`PageRequestV1` limits are 1 through 250 and continuations are opaque
`PageCursor` values. Frozen-context failures raise `FrozenReadError`; page,
evidence, and trace failures raise `PageError`, `EvidenceError`, and
`DependencyTraceError`, each with stable `reason` and `field_path`.

Set `explain=True` on `search_frozen`, or `include_explanation=True` on
`search_with_evidence`, to receive a validated non-empty correlation identity.
See [Frozen evidence](../guides/frozen-evidence.md) for the grounded retrieval
workflow and retry rules.

### Projection generation and constrained graph expansion

- `read.projection_generation_status(engine) -> ProjectionGenerationStatusV1`
  reports the serving projection generation and exact readiness without
  scheduling work.
- `read.mutation_projection_status(engine, request) ->
  MutationProjectionStatusV1` reads completion for one pending cursor already
  named by an actuation receipt.
- `graph.expand(engine, request) -> GraphExpandResultV1` accepts query or
  explicit seeds plus current or frozen read-context carriers. It returns
  deterministic ordered seeds and targets, complete work accounting,
  degradation codes, an optional compact explanation, and—when
  `include_evidence=True` under frozen authority—a positional exact-evidence
  sidecar.

The principal graph carriers are `GraphExpandRequestV1` (including optional
`edge_predicates` and `target_predicates`, each at most `32` `PropertyPredicate`
entries evaluated strictly against inline `properties`, bitemporal
`[t_valid, t_invalid)` edge gating, and optional mutually exclusive
knowledge-time bounds `as_of_write_cursor: str | None = None` — canonical
unsigned decimal `u64` string — and `as_of_epoch_ms: int | None = None` —
signed 64-bit integer epoch milliseconds), `GraphExpandResultV1`,
`GraphQuerySeedV1`, `GraphExplicitSeedV1`, `GraphReadContextV1`,
`GraphTargetV1` (with `properties` and `origin.terminal_edge_properties`), and
`GraphExpansionExplanationV1`. Historical `as_of` expansion composes
orthogonally with `ReadView.valid_as_of` (`ReadView` remains world-time-only),
resolves `as_of_epoch_ms` against `_fathomdb_commit_timestamps_ms_idx` (cursor
`"0"` returns empty query seeds and `GraphExpansionError("graph_seed_unavailable")`
for explicit seeds), evaluates declared `Filterable` keys inline on superseded
revisions, respects current-state `Engine.transition` and physical erasure /
closure barriers, and supports historical frozen evidence
(`artifact_lifecycle.superseded is True` when later superseded). Invalid graph
requests raise `GraphExpansionError` (including reasons
`"graph_context_invalid"`, `"graph_edge_predicates_invalid"`, and
`"graph_target_predicates_invalid"`); invalid projection-generation reads raise
`ProjectionGenerationError`.

## `admin.configure`

```python
from fathomdb import admin

receipt = admin.configure(engine, name="my-schema", body=schema_json)
```

`admin.configure(engine, *, name: str, body: str) -> WriteReceipt`.

Submit an admin schema configuration. The writer thread applies
it; the returned cursor places the apply in the global write order.

### `admin.configure_runtime(*, sqlite_mode) -> RuntimeConfiguration`

Select process-wide SQLite startup behavior before the first Engine opens:

```python
from fathomdb import admin

runtime = admin.configure_runtime(sqlite_mode="performance")
assert runtime.sqlite_mode == "performance"
```

The closed modes are `"performance"` and `"diagnostics"`. With no explicit
call, the first Engine open selects performance. Repeating the effective mode
is idempotent even after Engine open. A conflicting choice or a first request
after SQLite was initialized elsewhere raises `RuntimeConfigurationError`;
changing mode requires a process restart. This startup control is not a
database permission or a governed application command.

## `read.*` — governed read verbs (including 0.8.22 Slice 22)

```python
from fathomdb import read
```

The retrieval verbs below use the engine's **ReaderWorkerPool DEFERRED-tx
snapshot path**, preserving single-writer isolation. `read.projections` and
`read.projection_status` are different: they are pure introspection queries
through the ordinarily opened engine and may briefly take its connection lock.
They do not configure, write, or schedule work, but do not promise a separately
opened read-only SQLite mode.

### `read.get(engine, logical_id: str) -> NodeRecord | None`

Active-only point lookup by `logical_id` (active = `superseded_at IS NULL`). A
superseded version is never returned. A missing or superseded id returns `None`
— a **normal absence, not an exception** (a typed `NotFound` is a later-slice
concern).

### `read.get_many(engine, logical_ids: list[str]) -> list[NodeRecord | None]`

Batched point lookup. Returns one slot per requested id, **in request order**;
a missing/superseded id is `None` in its slot (partial result, never
all-or-nothing). `read.get` delegates to `read.get_many`.

### `read.collection(engine, collection, *, after_id=None, limit) -> list[OpStoreRow]`

Paginated op-store read-back over `operational_mutations` for `collection`,
**`ORDER BY id`**. `limit` is **mandatory** (the engine clamps it to a ~1M cap,
so no call yields an unbounded read); `after_id` is the exclusive cursor for the
next page.

### `read.mutations(engine, collection, *, after_id=None, limit) -> list[OpStoreRow]`

Mutation-log-oriented alias surface over the **same** op-store read-back as
`read.collection` (identical args + semantics).

### `read.operational_collections(engine) -> list[OperationalCollectionInfo]`

List all declared operational collections and their metadata (name, kind,
retention/policy details) in stable order.

### `read.operational_collection(engine, name: str) -> OperationalCollectionInfo | None`

Inspect one declared operational collection by `name`; returns `None` when no
collection with that name is registered. Conflicting `kind` re-registration on
write/configuration raises `WriteValidationError`.

### `read.visibility_state(engine) -> VisibilityStateV1`

Return the database's content-free visibility snapshot (`schema_version`,
`database_id`, `generation`, `nonce`, `canonical_write_boundary`,
`projection_cursor`) on one deferred reader transaction without acquiring the
writer lock. Raises `ClosingError` after close.

### `read.list(engine, kind, predicates=None, *, limit=100) -> list[NodeRecord]`

*(G4 / Slice 35)* List **active** `canonical_nodes` of the given `kind`
(`superseded_at IS NULL`), optionally filtered by a list of closed
`Predicate` dicts (AND-combined), up to `limit` rows (default 100).

Each predicate dict has the shape:

```python
{"type": "eq"|"gt"|"gte"|"lt"|"lte", "path": str, "value": str | int | bool}
```

`path` must be from the engine allowlist: `$.status`, `$.priority`,
`$.tags`, `$.kind`, `$.created_at`. A non-allowlisted path raises
`InvalidFilterError` (never a panic). Values are **always bound as
parameterized SQL** — never interpolated (injection-safe per ADR
D-F4). An empty `predicates` (or `None`) is the unfiltered path.

```python
from fathomdb import Engine, read
from fathomdb.errors import InvalidFilterError

engine = Engine.open("my.db")
# All active task nodes:
tasks = read.list(engine, "task")
# Filtered: open tasks with priority > 5:
open_high = read.list(engine, "task", predicates=[
    {"type": "eq",  "path": "$.status",   "value": "open"},
    {"type": "gt",  "path": "$.priority", "value": 5},
])
```

### Projection registry and derived readiness

`engine.configure_projections(specs, drop=None) -> ProjectionDelta` declares
the durable projection registry keyed by `(entity_class, name)`. It is
idempotent: omitting a live declaration does not delete it, while an explicit
destructive change or removal requires a strictly typed
`ProjectionTarget(name=..., entity_class=...)` in `drop` (bare strings raise
`TypeError`). `ProjectionDelta.built`, `.dropped`, and `.deferred` return
`list[ProjectionTarget]`. `ProjectionSpec` supports `entity_class` (`"node"`
default or `"edge"` projection targeting) and `vector_kinds` (opt-in custom node
kinds enrolled in `_fathomdb_vector_kinds` alongside the 7 built-in defaults;
reserved `"edge_fact"` is rejected). `read.projections(engine) ->
list[ProjectionSpec]` returns those durable declarations in `(name, entity_class)` order.

For an effective vector declaration, the returned
`ProjectionSpec.vector_dense_readiness: DenseReadiness | None` is engine-set
read metadata, never part of configuration. Supplying a valid readiness value
with `vector=True` to `configure_projections` is accepted but inert, so a result
from `read.projections` can be configured again as a no-op when its provider
selectors are still valid for production admission. Historical caller selector
strings are readable but refused on new configuration. An invalid spelling
or a readiness with `vector=False` is rejected. With no usable dense runtime
the engine reports `"unavailable"`; numerical verification refusal returns no
Engine and is not an attached readiness state.
With a usable runtime it reports `"embedding"` while eligible shared work is
outstanding; after `engine.drain(...)` completes and no further work is issued,
it reports `"ready"`. `DenseReadiness` is exactly `"unavailable" |
"embedding" | "ready"`.

### `read.projection_status(engine) -> ProjectionRuntimeStatus`

Read the current projection-runtime status without configuring projections or
changing the registry, storage, scheduler, or work queue. It is a status facade,
not a decorated `ProjectionSpec` and not a per-projection completion report.
It may briefly take the ordinarily opened engine connection lock; it is not a
ReaderWorkerPool request and does not promise a separate read-only SQLite
connection.

The frozen result has these fields:

```python
ProjectionRuntimeStatus(
    runtime_embedder_available: bool,
    runtime_unavailability_reason: (
        "none" | "no_runtime"
    ),
    projections: tuple[ProjectionRuntimeStatusEntry, ...],
    vector_unsupported_kinds: tuple[str, ...],
    vector_coverage: StoredHeadCoverageV1,
)
ProjectionRuntimeStatusEntry(
    name: str,
    dense_readiness: "not_declared" | "unavailable" | "embedding" | "ready",
    entity_class: "node" | "edge" = "node",
)
```

`"none"` is returned exactly when `runtime_embedder_available` is true.
Entries are sorted by name. `"not_declared"` means the declaration has no
effective vector arm (it needs both `searchable` and a vector sub-object), so a
legacy non-searchable vector sub-object remains `"not_declared"`. The other
readiness values are corpus-wide shared-pipeline facts and can repeat across
effective vector declarations. `vector_unsupported_kinds` is sorted and
deduplicated; it is `()` when no effective vector arm exists.

## `graph.*` — graph traversal (Slice 20 / G5 + G6)

```python
from fathomdb import graph
```

The `graph.*` namespace exposes bounded BFS traversal and hybrid
search-plus-expansion. Its reads ride the same **ReaderWorkerPool DEFERRED-tx
snapshot path** as the retrieval verbs in `read.*`; projection introspection
uses the ordinarily opened engine connection instead.

### `graph.neighbors(engine, logical_id, depth, direction="both") -> list[NodeRecord]`

G5 — bounded BFS from `logical_id` over `canonical_edges`.

- `logical_id` (`str`) — the root node's stable identity.
- `depth` (`int`) — hop limit; **must be 1, 2, or 3**.
  Depth > 3 raises `InvalidArgumentError`.
- `direction` (`str`) — edge direction to follow: `"outgoing"` (from→to),
  `"incoming"` (to→from), or `"both"`.

Returns up to **50** `NodeRecord`s reachable within `depth` hops
(root excluded). Edges with `t_invalid` in the past are silently skipped
(valid-time filter). Returns `[]` when the root has no reachable neighbors.

Raises `InvalidArgumentError` for depth > 3 or an unrecognised direction.

### `graph.search_expand(engine, query, depth, *, source_type=None, kind=None, created_after=None, status=None, search_limit=10, view=None) -> SearchExpandResult`

G6 — FTS/vector search (G1) followed by bounded BFS expansion.

- `query` (`str`) — free-text or embedding query (same as `engine.search`).
- `depth` (`int`) — BFS hop limit for expansion; 0 skips expansion.
  Depth > 3 raises `InvalidArgumentError`.
- Optional filter kwargs match `engine.search` semantics.
- `search_limit` (`int`, default `10`) — maximum initial ranked `search_hits`,
  in `1..=100`. It does not change the 50-per-root graph expansion cap.
- `view` (`ReadView | None`, default `None`) — optional validity/lifecycle
  `ReadView` applied consistently to both search and expansion (`search_expand_frozen`
  uses the view bound inside `FrozenReadContextV1` instead).

Returns a `SearchExpandResult`. Nodes that appear in both the search hit set
and the traversal reach appear **only** in `search_hits` (deduplication:
search score takes priority).

## Data shapes

### `WriteReceipt`

```python
@dataclass(frozen=True)
class WriteReceipt:
    cursor: int                       # batch high-water write_cursor
    row_cursors: tuple[int, ...]      # G0 — per-row write_cursor, 1:1 with the batch
    dangling_edge_endpoints: int      # 0 under strict mode; truthful count under bulk_non_strict
```

`row_cursors` is the `write_cursor`-as-row-id identity carrier (G0 /
Slice 15): for an N-row batch it is `(cursor - N + 1, …, cursor)`.

`dangling_edge_endpoints` reports the count of unresolved edge endpoints across
surviving active edges (`superseded_at IS NULL`) written by the batch:

- Under `"strict"` mode (`engine.write` and default `engine.write_with_options`),
  missing, superseded-only, `pending`, `deleted`, or cross-scope endpoints roll
  back atomically with `WriteValidationError`, so committed receipts always
  report `0`.
- Under opt-in `"bulk_non_strict"` mode (`engine.write_with_options(...,
  WriteOptions(endpoint_validation="bulk_non_strict"))`), `from` and `to` are
  probed independently against active same-scope canonical nodes, so each
  surviving active edge contributes `0`, `1`, or `2` (and node-only batches
  report `0`). Cross-scope active endpoints still fail atomically with
  `ScopeViolationError`.

### `WriteOptions` and `EndpointValidationMode`

```python
EndpointValidationMode = Literal["strict", "bulk_non_strict"]

@dataclass(frozen=True)
class WriteOptions:
    endpoint_validation: EndpointValidationMode = "strict"
```

### `NodeRecord`

```python
@dataclass(frozen=True)
class NodeRecord:
    logical_id: str
    kind: str
    body: str
    write_cursor: int   # engine-internal positional cursor — NOT SearchHit.id
    properties: dict[str, Any] | None = None
```

Returned by `read.get` / `read.get_many` for an **active** canonical node
(`superseded_at IS NULL`). Mirrors the TypeScript `NodeRecord`.

`logical_id` is the caller-facing identity here; `write_cursor` is the engine's
positional book-keeping value, reassigned on re-projection and **not** the same
carrier as [`SearchHit.id`](#searchhit) (which is a typed `IdSpace`).

### `OpStoreRow`

```python
@dataclass(frozen=True)
class OpStoreRow:
    id: int               # operational_mutations PK + the after_id cursor key
    collection: str
    record_key: str
    op_kind: str          # always "append"
    payload: str          # the stored payload_json
    schema_id: str | None
    write_cursor: int
```

Returned by `read.collection` / `read.mutations`. `id` is the after-id cursor
key. Mirrors the TypeScript `OpStoreRow`.

### `SearchResult`

```python
@dataclass(frozen=True)
class SearchResult:
    projection_cursor: int
    soft_fallback: SoftFallback | None = None
    results: list[SearchHit] = []
    hints: list[Hint] | None = None
```

### `SearchHit`

```python
@dataclass(frozen=True)
class IdSpace:
    space: str       # "logical" | "content" | "passage"
    value: str       # the BARE id (id-space prefix stripped)

@dataclass(frozen=True)
class SearchHit:
    id: IdSpace      # typed, non-null, id-space-total hit identity
    kind: str
    body: str
    score: float     # G9 RRF-fused relevance (Σ w/(30+rank), text w=3, vector w=1); higher = better
    branch: SoftFallbackBranch  # Literal["vector", "text", "text_edge", "graph_arm"]
    source_id: str | None = None  # provenance (`erase_source` arg); set on EVERY hit (TC-31)
    ce_score: float | None = None  # 0.8.5 CE score (sigmoid logit) for in-pool reranked hits
    properties: dict[str, Any] | None = None  # optional inline JSON properties
```

> **BREAKING since 0.8.9.** `SearchHit.id` was an `int` row cursor. It is now
> the typed `IdSpace` above — the **permanent** caller-facing identity, not an
> interim carrier. `space` is `"logical"` for governed rows (prefix `l:`),
> `"content"` for doc-seeded rows (`h:`), `"passage"` for synthetic passages
> (`p:`); `value` is the bare id with that prefix stripped, and
> `f"{prefix}{value}"` reproduces the pre-0.8.19 `stable_id` byte-for-byte. It
> is stable across sessions and re-ingest, and never participates in ranking.
> The engine's positional `write_cursor` is internal book-keeping and is **not
> surfaced by the bindings**. Only `logical`-space ids are lifecycle-addressable
> by `transition` / `purge`.

`score` is the **G9 RRF-fused** relevance (higher = more relevant), optionally
recency-reweighted. Raw `vec_distance_l2` (vector) and `bm25()` (text) are fused
on **rank**, never compared raw (they are not comparable). `branch` tags which
branch produced the representative hit (vector-first when both surface a body).
`ce_score` (0.8.5 / EXP-0) is the per-candidate cross-encoder score
(`sigmoid(ce_logit)`) for hits inside the reranked pool, `None` otherwise.

`source_id` carries the hit's source-document provenance — the identifier
`engine.erase_source` consumes. **TC-31 (0.8.20): it is populated on
every hit path**, not just the graph arm, so a caller can always resolve a hit
back to the document it came from. Node hits (text/vector) carry the node's own
`source_id`; edge hits (edge-FTS, vector edge-fact) carry the edge's own;
graph-arm hits carry the traversed edge's. It is `None` only when the stored row
genuinely has NULL provenance — written before 0.8.20, or a governed row spared
by the step-21 backfill. `source_id` never participates in ranking.

### `SearchFilter`

```python
@dataclass(frozen=True)
class SearchFilter:
    source_type: str | None = None
    kind: str | None = None
    created_after: int | None = None   # created_at >= bound (unix seconds)
    status: str | None = None
    attributes: tuple[tuple[str, str], ...] = ()
    property_predicates: tuple[PropertyPredicate, ...] = ()
```

G10 — a **closed** metadata filter (not an open DSL) for `engine.search`. Each
present field constrains the vector branch in a single phase-1 KNN statement and
constrains the text branch by the same metadata; `None`/all-`None` is the
unfiltered path (byte-identical to the pre-filter query). `status` filters the
vec0 `status` column, which ships an **empty-string sentinel only** (no real
population source yet — vec0 TEXT metadata is not NULL-able), so a
`status="open"`-style filter prunes every row until a population slice lands.
`attributes` is ordered AND equality over declared `filterable` projections;
its values are canonical text, so projected string `"1"` and number `1` both
match `"1"`. `property_predicates` adds closed typed predicates (`eq`, `ne`,
`lt`, `lte`, `gt`, `gte`) over materialized properties.

### `SoftFallback`

```python
@dataclass(frozen=True)
class SoftFallback:
    branch: SoftFallbackBranch  # Literal["vector", "text", "text_edge", "graph_arm"]
```

`branch` indicates which non-essential branch could not contribute.
Total request failure is not expressed via this carrier.

### `CounterSnapshot`

```python
@dataclass(frozen=True)
class CounterSnapshot:
    queries: int = 0
    writes: int = 0
    write_rows: int = 0
    admin_ops: int = 0
    cache_hit: int = 0
    cache_miss: int = 0
```

### `ExpandedNode`

```python
@dataclass(frozen=True)
class ExpandedNode:
    node: NodeRecord      # the reachable node
    hop_count: int        # BFS distance from the nearest search-hit root
```

Returned in `SearchExpandResult.expanded`. Only nodes NOT already in
`search_hits` appear here.

### `SearchExpandResult`

```python
@dataclass(frozen=True)
class SearchExpandResult:
    search_hits: list[SearchHit]     # original RRF-scored search results
    expanded: list[ExpandedNode]     # nodes reachable by traversal, not in search_hits
    all_logical_ids: list[str]       # deduplicated union of both sets
```

Returned by `graph.search_expand`. `all_logical_ids` contains the
`logical_id` strings for every node in both `search_hits` and `expanded`.

## Errors

`fathomdb.errors` exports `EngineError` (the catch-all base) plus **47**
concrete leaf classes below it (`48` shared error classes total), including
`ScopeViolationError` (`0.8.32`). See [errors reference](errors.md) for the
full matrix and recovery-hint codes.

The lifecycle / erasure verbs raise `IllegalTransitionError`,
`NotLifecycleAddressableError`, `ErasureIncompleteError` and
`WriteValidationError`; `configure_projections` raises
`ProjectionDestructiveError` and `WriteValidationError`.

## Embedder device (GPU)

There is **no Python API** for selecting the embedder device — it is chosen by a
artifact capability plus the `FATHOMDB_EMBED_DEVICE` environment variable
(`auto` · `cpu` · `cuda:N`), resolved when the engine opens. Unset is `auto` only on a CUDA-capable artifact; a CPU-only artifact reports
`cuda_not_compiled`. `auto` records a typed CPU result when CUDA is
unavailable; forced `cuda:N` fails rather than falling back. End users of a
CUDA-capable package do not rebuild to switch between CPU and GPU. See
[Default Embedder → GPU acceleration](../embedder.md#gpu-acceleration-opt-in).

`FATHOMDB_RERANK_DEVICE` independently selects TinyBERT CE with the same
`auto` · `cpu` · `cuda:N` grammar. Both model paths may select the same GPU;
this does not make SQLite retrieval, FTS, fusion, graph, or storage GPU work,
and FathomDB provides no GPU reservation or hard memory cap.

## Explicit erasure maintenance

The persisted policy and owned maintenance window are available in all three SDKs.
Python uses `begin_erasure_maintenance`, `queue_physical_erasure`,
`physical_erasure_status`, `withdraw_queued_erasure` and
`complete_physical_erasure`; TypeScript uses camelCase counterparts. Queueing
leaves existing logical visibility unchanged. A window excludes ordinary work;
activated incomplete debt retains durable write fences and original-sink/WAL
requirements. Acquisition wait is not a cleanup timeout. Handle close does not
forgive debt. See [the complete maintenance example](../operations/erasure.md#schedule-an-explicit-maintenance-window)
for typed outcomes, original-sink reattachment and retry rules.

## Stored-head reporting

Projection runtime status includes required stored mapped-head coverage in
`vector_coverage`. Available
values report exact known/unknown/truncated heads, known/unrepresented bytes,
unattributed heads and exact unreduced known-only shares; unavailable values
name cache_cold or state_changed without numeric facts. The population includes
retained mapped history, independently of current eligibility or physical health.
Only already-required committed full validation seeds the cache; ordinary status
reads do not force a scan or refresh. Counts are exact Python ints in Python and
canonical decimal strings in TypeScript; profile dimension remains numeric.

An opt-in search snippet's scored_bytes (Python) / scoredBytes (TypeScript)
is the exact stored head endpoint of that returned record's actual original
vector participation. Known zero stays zero; unknown/nonparticipant makes no
extent claim. Standalone snippet selection and text-only search have no extent
lookup. This does not change ranking, snippet anchors, frozen eligibility or
body-byte coordinates, including role tags.

## Batch evidence resolution, search hints, and OpenReport capabilities

- **`engine.resolve_evidence_many(request: EvidenceResolveManyRequestV1) -> list[EvidenceResolveManyItemV1]`:**
  Resolves `1..=50` (`EVIDENCE_RESOLVE_MANY_MAX = 50`) opaque evidence
  references under one `FrozenReadContextV1` in a single read snapshot. Enforces
  `schema_version == 1` (`EvidenceError("unsupported_schema_version")`),
  list/tuple of `str` (`TypeError`), no NUL bytes (`WriteValidationError`), and
  raw count `1..=50` (`InvalidArgumentError`). Preserves duplicate order and
  arity; returns per-slot `EvidenceResolveManyItemV1(status="resolved", resolved=...)`
  or `EvidenceResolveManyItemV1(status="refused", reason="evidence_unavailable", field_path="/evidenceRefs/{i}")`
  while collapsing frozen-context drift into whole-call
  `EvidenceError("evidence_unavailable")`.
- **Opt-in search hints (`hints: bool = False`):** Accepted as a keyword-only
  parameter on `engine.search`, `engine.search_frozen`, and
  `EvidenceSearchRequestV1` (non-`bool` values raise `TypeError`). When `False`
  (default), `SearchResult.hints` is `None` with zero overhead; when `True`,
  `SearchResult.hints` contains `list[Hint]` (`action`, `reason`, `mode`,
  `limit`, `arm`, `cause`).
- **`OpenReport` capability booleans:** `engine.open_report()` exposes
  `reranker_compiled: bool`, `cuda_compiled: bool`, and
  `rerank_available: bool` with zero network I/O and zero GPU allocation during
  `Engine.open`.

## Caller scope isolation (0.8.32, Schema 39)

By default a FathomDB database is **single-scope / single-agent** (`scope = ""`,
`ScopePolicy.unscoped()`). Multi-agent or shared-store deployments opt into
engine-enforced isolation by constructing a frozen `ScopePolicy`:

- `ScopePolicy.unscoped()` — single-scope default (`scope = ""`).
- `ScopePolicy.single(scope: str)` — read and write one scope token.
- `ScopePolicy.read_only(read_scopes)` — read-only across `1..=32` scopes (any
  write/mutation/admin verb raises `ScopeViolationError`).
- `ScopePolicy.read_write(read_scopes, write_scope)` — read `read_scopes`,
  write `write_scope` (must be an element of `read_scopes`).

Pass via `EngineConfig(scope_policy=...)` or `Engine.open(path, scope_policy=...)`.
Tokens must be non-empty UTF-8 `<= 128` bytes without leading/trailing
whitespace, ASCII control/`NUL`/`\x1e`, or `"*"`. `FrozenReadContextV1` binds
the handle's policy digest (cross-policy reuse raises `FrozenReadError`);
cross-scope edge endpoints raise `WriteValidationError` and cross-scope
evidence tokens raise `EvidenceError("evidence_unavailable")` (D-01
nondisclosure).

## See also

- [Quickstart](../getting-started/quickstart.md)
- [Config knobs](config.md)
- [Errors](errors.md)
- [Erasure](../operations/erasure.md)
- Locked spec: [`dev/interfaces/python.md`](https://github.com/fathomadb/fathomdb/blob/release/0.8.31/dev/interfaces/python.md)
