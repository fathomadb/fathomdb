---
title: Retrieval Subsystem Design
date: 2026-11-26
target_release: 0.8.32
desc: Current lexical, vector, fusion, reranking, graph, frozen-view, explanation, and evidence pipeline
blast_radius: fathomdb-query; fathomdb-engine retrieval/frozen/evidence/graph paths; SDK search and graph interfaces
status: ACTIVE
---

# Retrieval design

This file owns how current retrieval mechanisms compose. Public request,
response, and error spellings remain in `dev/interfaces/`; exact result-limit
rules remain in [`retrieval-result-limits.md`](retrieval-result-limits.md); the
shared vector representation remains owned by [`vector.md`](vector.md).

## Proposed current/history isolation

The [post-POC production design](current-history-search.md) proposes isolated
current statistical corpora, explicit corrected-history/prior-belief semantics,
and writer-owned validity-boundary refresh. It is not implemented or latency
qualified. Its [follow-up POC proposal](../experiments/current-history-search/next-poc-plan.md)
closes schema compatibility, cheap certification and historical resource/backend
questions before production adoption. The live mechanisms below remain the
current contract.

## Pipeline and authority order

Retrieval is a typed, bounded data-plane operation:

1. validate query, result bound, filter, view or frozen context, explanation,
   evidence, graph, and reranker options before candidate execution;
2. compile text through the safe `fathomdb-query` grammar rather than passing
   caller text directly to FTS5;
3. apply declared projection roles, lifecycle, valid-time, source, attribute,
   and frozen eligibility before each branch's candidate cutoff;
4. collect lexical and vector candidates, hydrate canonical state, and fuse
   contributing arms deterministically;
5. apply only requested optional graph, reweight, or cross-encoder mechanisms;
   and
6. truncate to the public limit, then finalize opt-in explanation/evidence from
   the same authorized result snapshot.

Eligibility-before-truncation is load-bearing. A hidden, expired, deleted,
superseded, ineligible, or wrong-generation row cannot consume a bounded slot
and then disappear after ranking. Current and frozen paths share that rule.

## Current .28 selection and query compilation

Direct node/edge body FTS applies optional SearchFilter eligibility in its
snapshot-local SQL before candidate caps, per-record credit suppression, ranking and limit.
Fusion identity follows [the accepted record-version decision](../adr/ADR-0.8.29-retrieval-record-identity.md); public IdSpace is unchanged.
A same-selection/query/filter/effective-time comparison preserves the ordered
prefix as the accepted limit grows. This does not promise hybrid prefix order.
Node attributes use declared filterable projections; a nonempty attribute
filter excludes edge hits rather than ignoring its predicates. Status/time require existing vector metadata, with empty-string status
and inclusive projection time; missing rows fail to match. Kind/source type and
available attributes work without a provider.

Independent include_superseded/include_inactive flags widen lifecycle visibility
without weakening validity, dependency or mandatory edge-expiry rules. Historical
property predicates use canonical rows; incomplete historical dense coverage
uses lexical fallback. Dense direct lifecycle/validity eligibility is compiled
inside vec0 before its cap, replacing the old whole-store eligibility scan.

Query stop words are fixed at open, matched exactly before stemming, and
identified by their effective hash. Context pack is a stateless SDK renderer;
full_ids overrides automatic tiers, scalar item-text budgets exclude rendering
overhead. Neither adds a retrieval/model round or changes stored body bytes.
Details remain in the interfaces and public context/filter guides.

## Candidate arms

The live hit-provenance enum has four values:

- `Vector` — eligible node candidates from the shared dense table;
- `Text` — canonical node-body or declared projected-text FTS candidates;
- `TextEdge` — edge-body text or vector-projected edge-fact candidates; and
- `GraphArm` — nodes contributed by the optional bounded search graph arm.

## Soft-fallback signal

The soft-fallback record is narrower than hit provenance. It reports only a
nonessential `Vector` or `Text` branch that could not contribute; total request
failure remains a typed error. `TextEdge` and `GraphArm` identify returned hit
origins and are not new soft-fallback categories.

### Lexical collection

Node and edge text uses maintained FTS tables. Declared nested-source projected
text participates only through the registered projection grammar. Direct
`search_text_only*` requests preserve the stable prefix contract: the unfiltered
node-only fast path reads the hidden-rank stream through the complete score
group crossing its fixed candidate boundary, restores stable
`(bm25 score, write_cursor)` order, and then applies the public limit. Filtered
FTS-only `search_frozen` and `search_with_evidence` push `search_index.kind = ?`
before joining `canonical_nodes`, use FTS5 `ORDER BY rank` streaming /
`collect_complete_fusion_rank_boundary` for deterministic `(bm25 score, write_cursor)`
prefixes (never applying an `ORDER BY`-less `LIMIT` on `defer_full_hydration`),
cap `fts_only_limit` when `_fathomdb_vector_kinds` is empty strictly when
`!use_graph_arm && !recency_enabled && !importance_enabled` (respecting
`rerank_depth` pool bounds when `rerank_depth > 0`), and defer
`build_importance_confidence_maps` until after `truncate(final_limit)` whenever
`!importance_enabled` (preserving `EngineError::Storage` propagation on row
decode failure for all surviving hits `<= final_limit`). Unscoped single-store
detection (`store_single_unscoped`) uses `SEARCH`-only `EXISTS(... WHERE scope < '' LIMIT 1) OR EXISTS(... WHERE scope > '' LIMIT 1)`
range probes gated behind cheap booleans; `detect_vector_soft_fallback`
short-circuits in `O(1)` via `EXISTS(SELECT 1 FROM _fathomdb_vector_kinds LIMIT 1)`;
`historical_dense_incomplete` short-circuits immediately when
`query_vector.is_none()`; and `DEFERRED` snapshot pinning in both
`frozen_read::mint_inner` and `graph_expand/execution.rs` (`Current` branch)
reads `_fathomdb_read_visibility_state WHERE singleton = 1` in `O(1)` instead
of scanning `canonical_nodes`. Edge-bearing workspaces, unsupported shapes, and
statement/stream-row failures use the full stable-sort path. A row decoding
failure on that path propagates `EngineError::Storage`; it must not return a
partial result. The optimization does not change public ordering.

### Vector collection and fusion

Vector retrieval uses the one schema-owned `vector_default` virtual table plus
its authority sidecars. Phase 1 uses the binary shortlist; phase 2 exact-f32
distance reranks that shortlist. Kind and declared filter attributes constrain
the candidate statement before its limit.

Contributing lexical, vector, and optional graph arms are combined by weighted
reciprocal-rank fusion. There is no public `fusion_mode` switch. Vector-first
tiebreaking and stable cursor ordering make equal-score output deterministic.
Recency and importance/confidence reweights are separate, default-off
mechanisms; they do not redefine the base fusion contract.
Absent stored importance or confidence remains neutral. When ranking or an
opt-in explanation reads those values, a statement or row-decoding failure
returns `EngineError::Storage` rather than treating the failed lookup as absent.

Numerical compatibility failure refuses open with no Engine. An explicit
subsequent no-provider open can supply text-only access if otherwise admissible;
there is no attached refused Engine or automatic retry. Device selection for the embedder and cross-encoder is
independent; a reranker device result never attests the embedding or SQLite
candidate path.

## Cross-encoder reranking

Search may request bounded cross-encoder reranking of its fused pool. The model
contributes nullable `ce_score` values and the configured blend only inside the
reranked pool; depth zero, empty input, or the feature-off/model-unavailable
identity path preserves input ordering and scores. Validation and forced-device
errors occur before or at the owned reranker boundary and do not silently fall
back from forced CUDA.

Standalone `rerank` is a live governed package operation in Python and
TypeScript. It accepts caller passages and reuses the same bounded reranker
mechanism, but it does not query SQLite and does not imply that ordinary search
always reranks. The executable operation map and binding surface oracle are
governed by `src/conformance/governed-operation-parity.json` and
`src/conformance/governed-surface-allowlist.json`.

## Current and frozen reads

`ReadView` owns current visibility and valid-time policy. A frozen read context
authenticates the database, effective view, validity instant, canonical
high-water boundary, dependency generation, projection generation, and request
eligibility envelope. Reproduction succeeds only while those facts remain
available and coherent; tamper, foreign database, or state drift returns the
typed frozen-read outcome rather than silently reading current state.

On Schema `39+` databases ([ADR-0.8.32 `#103`](../adr/ADR-0.8.32-frozen-read-additive-stability-and-evidence-eligibility.md)),
`FrozenReadBinding` binds only the visibility partitions intersected by
`scope.read_scopes()` (`ScopePolicy::Unscoped` resolves uniformly to `[""]`),
`ReadContextV1.eligibility`, and the caller's active `ReadSurface`
(`OperationalState`, `CanonicalPage`, `Preflight`, `SearchNoGraph`,
`EvidenceNode`, `GraphOrEdge`, `EvidenceGraph`, `TraceDependency`) using
`_fathomdb_scope_visibility_watermarks` (`WITHOUT ROWID`),
`_fathomdb_read_visibility_state.(global_struct_gen, global_struct_nonce)`,
universal `serving_digest` (`SERVING_DOMAIN_V4` over `global_struct`, sorted
`current_generation`, and `(scope, 'scope_destructive')`), and `34 B`/`66 B`
trailing `0x53 0x39` surface tags (`492`–`556 B` total `fdbfr1` token vs.
`1,024 B` cap; Schema `< 39` retains legacy global encoding bit-for-bit).
`validate_snapshot` always returns the global monotonic
`load_visibility_generation` clock for `Engine::read_visibility_generation.fetch_max`.
`search_expand_frozen` (`expand_depth.is_some()`) routes through `GraphOrEdge`;
`resolve_evidence` and `resolve_evidence_many` perform pre-decode
`EvidenceNode` validation, post-decode `edge_any` (`projection_cursor`) checks
for `Edge`/`GraphArm` payloads (emitting per-slot `Refused(EvidenceUnavailable)`
in `resolve_evidence_many` so unaffected node-only slots resolve), and
post-resolve TOCTOU re-validation (`EvidenceGraph` when any slot resolved
`Edge`/`GraphArm`). Unrelated additive writes (disjoint canonical `kind`,
`operational_mutations` append-only logs, `operational_state` vs. canonical
surfaces, isolated tenant scopes including destructive mutations in other
scopes, and unrelated background `projection_worker` commits) preserve frozen
contexts, while same-partition writes, matching-kind edge-FTS inserts,
same-scope destructive/dependency mutations, global structural updates, and raw
`coalescing = 0` SQL fail closed. Eligibility, provenance, lifecycle/validity,
keyset order, `operational_state`, and graph topology are exact across
unrelated additive writes; FTS5 `bm25` raw scores reflect live corpus
`(N, avgdl, df)` statistics rather than frozen IDF snapshots (`ADR #103` §1.7).

Frozen pagination and operational-state reads use their own bound cursor
contracts. A frozen context is not a permanently held SQLite transaction or a
persisted lease. Each operation opens a reader transaction and revalidates the
authenticated boundary.

## Graph retrieval

Two bounded graph mechanisms are current:

- the optional search graph arm seeds from eligible retrieval results and adds
  `GraphArm` candidates before final fusion; and
- the governed `graph_expand` V1 operation accepts query or explicit seeds,
  direction, edge and target kinds, indexed predicates, current or frozen
  context, and explicit depth/work/result bounds.

Graph seed and target eligibility is applied before seed truncation and
expansion. Results are deterministic one-page targets with compact origin and
degradation state. General continuation, full paths, associative diffusion,
and semantic truth inference remain deferred.

## Explanation and evidence

Explanation is an opt-in sidecar finalized exactly once with a nonempty
engine-minted correlation identity. It describes returned results using the
live fusion/reranking, fallback, projection, graph, dependency, and lifecycle
facts; it does not run a parallel ranking implementation. Explanation-off
ranking and result identity remain unchanged.

For hits scored by `execute_vector_arm` (`vector_rank.is_some()`, including
vector-scored `TextEdge` `edge_fact` hits), `PerHitExplain` (`#[non_exhaustive]`,
extending [`0.8.8-explain-and-telemetry-adr.md`](0.8.8-explain-and-telemetry-adr.md)
§A.4 Q1 specifically for `vector_cosine` under `Gate1-D10` while keeping
[`EP-13-raw-arm-scores.md`](mcp-adapter/engine-proposals/EP-13-raw-arm-scores.md)
`vector_distance`/`text_bm25` deferred) carries additive
`vector_cosine: Option<f64>`, computed as finite `1.0 - vec_distance_cosine`
clamped to `[-1.0, 1.0]` over uncentered f32 vectors in the same search snapshot
(covering unit-normalized built-in embeddings as well as non-unit-norm
test/unnormalized vectors without restoring custom production provider admission
per `ADR-0.8.28`). Zero-norm, `NULL`, or `NaN` cosine distances and any hit with
`vector_rank == None` (single- or multi-arm) report `None` (never `0.0`).
`vec_distance_cosine` is not evaluated when `explain = false`, and `explain = true`
preserves L2 ordering. `SearchHit`, `fathomdb-mcp` `PerHitOut`, and the
HMAC-authenticated `EvidenceContributionV1` wire codec remain unchanged.

Snapshot projection-generation status needed by `search(..., explain=true)`,
`search_frozen(..., explain=true)`, `search_with_evidence(include_explanation=true)`,
and query-seeded `graph_expand` shares one unified snapshot boundary helper with
`read_projection_generation_status` and reuses the engine's scope-partitioned
`CachedProjectionGenerationStatus` cache across `Engine` and `ReaderWorkerPool`
with monotonic cache-epoch guarding and per-connection `PRAGMA data_version`
(`+ total_changes` on `Engine.connection`) invalidation, keeping steady-state
`status_in_snapshot` work `O(1)` under default lifecycle views
(`!include_superseded && !include_inactive`) while immediately observing
internal and external SQLite mutations, projection publication, `runtime_state`
changes, and valid-time membership boundaries. Inside `empty_physical_fast_path`,
the internal `canonical_nodes` aggregation (`node_sql`) is skipped whenever
`!(node_arm_declared || has_explicit_node_enrolment)` while preserving
`edge_count` and `edge_count != 0 && !edge_enrolled` corruption checks. When
`search_with_evidence` is invoked with `include_explanation = false`,
lightweight `EvidenceContributionV1` capture is decoupled from full
`explain = true`, skipping status-cache/snapshot checks, structural explain
hydration, and `vec_distance_cosine` SQL evaluation while producing
bit-identical `EvidenceContributionV1` payloads.

Compact ranked evidence is also opt-in and resolves under its authenticated
frozen eligibility envelope to exact source revision, locator, hash (computing
each `source_body` SHA-256 once per resolution rather than duplicating across
evidence and dependency verification), bytes or span, lifecycle, contribution,
and dependency facts. Under [ADR-0.8.32 `#103`](../adr/ADR-0.8.32-frozen-read-additive-stability-and-evidence-eligibility.md)
Part 2 (amending `ADR-0.8.25` `#48`), `SearchFilter.property_predicates` is
artifact-scoped and cleared on `source_filter` alongside `kind` and
`source_type` across `authorize_source_access`, `query_graph_evidence_batch`,
and `resolve_graph_evidence`, while access-bearing `attributes`,
`created_after`, `status`, `ReadView` validity/lifecycle, dependency closures,
and Schema 39 `ScopePredicate` remain strictly source-enforced. By contrast,
`trace_dependency` preserves `ADR-0.8.25 #49` symmetric endpoint visibility:
only its SQL candidate pre-queries omit `property_predicates`, while Rust
`visible_artifact` in `authorize_candidate_chain` and `include_candidate`
continues applying full `context.eligibility` (including `kind`, `source_type`,
and `property_predicates`) to both root and counterpart endpoints (intentionally
unchanged by `#269` Option A). On `GraphArm`-origin hits, `SearchHit.source_id` intentionally preserves the
traversed edge's legacy `source_id` while `ResolvedEvidenceV1.source_id`
resolves the returned node artifact's own canonical provenance source.
Authorization and nondisclosure precede detailed corruption diagnostics.

Bounded graph expansion has a distinct positional V1 evidence sidecar.
Frozen-only opaque references bind the selected target and winning terminal
edge revisions to the originating normalized request and disclosure envelope.
Resolution returns intrinsic artifact/source evidence; it never fabricates
rank, contribution, or a complete path. Exact behavior is owned by
[`ADR-0.8.26-exact-graph-artifact-evidence.md`](../adr/ADR-0.8.26-exact-graph-artifact-evidence.md).

## Ownership boundaries

- Bindings own language spelling and conversion precedence.
- Projection owners define generation/readiness and declared roles.
- The engine owner defines reader/writer topology and cursor semantics.
- The evidence and graph ADRs define opaque reference security and failure
  precedence.
- Semantic planning, synthesis, answer verification, and model/provider spend
  remain caller-owned.

Historical 0.6.0 stage and two-branch descriptions remain available in Git but
are not current authority.
