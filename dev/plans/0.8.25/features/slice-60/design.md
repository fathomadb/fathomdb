---
title: 0.8.25 Slice 60 — minimal constrained combined-expansion design
status: FIX1_AWAITING_REVIEW
design_version: 3
target_release: 0.8.25
depends_on: 55
architecture: dev/design/fathomdb-data-plane-architecture-v2.md
---

# Slice 60 design

## Authority, outcome, and boundary

This slice implements the retained subset of R25/AC25-60, Memex need 15, the
compact graph-origin portion of need 12, and A25-05/A25-06. It adds one
governed combined-expansion operation over the existing
`canonical_nodes`/`canonical_edges` substrate. It does not replace or silently
change `graph_neighbors`, `search_expand`, `search_expand_frozen`, default
`Engine::search`, or `use_graph_arm=false`.

The operation accepts query-derived or explicit logical-node seeds; applies a
current or authenticated frozen read context; honors incoming, outgoing, or
both direction; filters traversed edge kinds; filters returned target kinds;
and exhausts one bounded deterministic walk before returning a complete top-N
one-page result.

Rich continuation, full ordered path replay, diffusion/PPR, semantic routing,
ontology, relation inference, target predicates beyond exact kind, and the
rejected exact-anchor retrieval treatment remain outside 0.8.25. A compact
origin identifies the seed, predecessor, terminal edge kind, actual terminal
direction, and hop count; it is not full path evidence.

## Existing substrate and net-new work

The implementation extends the shipped graph path rather than adding another
graph engine.

- Existing: opaque logical identity, enriched temporal edges, endpoint indexes,
  bounded BFS helpers, direction support, `ReadContextV1`, authenticated
  `FrozenReadContextV1`, indexed eligibility, projection generations, compact
  structural explanation, and Rust/Python/TypeScript binding patterns.
- Net-new: the versioned constrained request/response/error types, one exact
  graph expansion implementation, native logical-node query seeding, exact edge
  and target kind constraints, deterministic origin selection, caller work
  accounting, and cross-SDK codecs.
- Unchanged: schema version 33. The existing
  `canonical_edges_from_id_idx`/`canonical_edges_to_id_idx` and active
  logical-node lookup are sufficient. No table, column, index, trigger,
  backfill, or migration is added.

Kind values are ontology-neutral, open, case-sensitive UTF-8 strings because
the current schema has no kind registry. A nonblank value is valid even when no
stored row currently uses it; that case yields no matching traversal or output,
not an “unknown kind” error. This design does not invent a registry.

## Exact public surface

### Rust

The default facade re-exports every type below. New public structs are
`#[non_exhaustive]` where external field construction would otherwise prevent
additive response evolution.

```rust
pub enum GraphSeedV1 {
    Query {
        schema_version: u32,
        text: String,
        ranked_limit: u32,
    },
    Explicit {
        schema_version: u32,
        logical_ids: Vec<IdSpace>,
    },
}

pub enum GraphReadContextV1 {
    Current {
        schema_version: u32,
        context: ReadContextV1,
    },
    Frozen {
        schema_version: u32,
        context: FrozenReadContextV1,
    },
}

pub struct GraphExpandRequestV1 {
    pub schema_version: u32,
    pub seed: GraphSeedV1,
    pub direction: TraversalDirection,
    pub edge_kinds: Vec<String>,
    pub target_kinds: Vec<String>,
    pub context: GraphReadContextV1,
    pub max_depth: u32,
    pub result_limit: u32,
    pub max_work_units: u64,
    pub include_explanation: bool,
}

pub struct ResolvedGraphSeedV1 {
    pub schema_version: u32,
    pub logical_id: String,
    pub seed_ordinal: u32,
    pub query_score: Option<f64>,
}

pub struct GraphOriginV1 {
    pub schema_version: u32,
    pub seed_logical_id: String,
    pub seed_ordinal: u32,
    pub predecessor_logical_id: String,
    pub target_logical_id: String,
    pub hop_count: u32,
    pub terminal_edge_kind: String,
    pub terminal_direction: TraversalDirection,
}

pub struct GraphTargetV1 {
    pub schema_version: u32,
    pub logical_id: String,
    pub kind: String,
    pub body: String,
    pub write_cursor: u64,
    pub origin: GraphOriginV1,
}

pub enum GraphExpansionDegradationCodeV1 {
    QuerySeedTextFallback,
    ProjectionLegacyUnverified,
    ProjectionProcessing,
    ProjectionBlocked,
    ProjectionDeferred,
    ProjectionDegraded,
}

pub struct GraphTargetExplanationV1 {
    pub schema_version: u32,
    pub target_index: u32,
    pub origin: GraphOriginV1,
    pub lifecycle_state: StructuralLifecycleStateV1,
    pub dependency_state: StructuralDependencyStateV1,
}

pub struct GraphExpansionExplanationV1 {
    pub schema_version: u32,
    pub correlation_id: String,
    pub seed_source: GraphSeedSourceV1,
    pub read_mode: GraphReadModeV1,
    pub projection_generation_id: Option<String>,
    pub projection_origin: GraphProjectionOriginV1,
    pub projection_readiness: GraphProjectionReadinessV1,
    pub degradation_codes: Vec<GraphExpansionDegradationCodeV1>,
    pub per_target: Vec<GraphTargetExplanationV1>,
}

pub struct GraphExpandResultV1 {
    pub schema_version: u32,
    pub seeds: Vec<ResolvedGraphSeedV1>,
    pub targets: Vec<GraphTargetV1>,
    pub complete: bool,
    pub work_units: u64,
    pub degradation_codes: Vec<GraphExpansionDegradationCodeV1>,
    pub explanation: Option<GraphExpansionExplanationV1>,
}

impl Engine {
    pub fn graph_expand(
        &self,
        request: &GraphExpandRequestV1,
    ) -> Result<GraphExpandResultV1, EngineError>;
}
```

`GraphSeedSourceV1` is the closed enum `query | explicit`.
`GraphReadModeV1` is `current | frozen`. `GraphProjectionOriginV1` is
`not_applicable | fresh | legacy_unverified | configuration | rebuild`.
`GraphProjectionReadinessV1` is
`not_applicable | ready | processing | blocked | deferred | degraded`.
`EngineError` gains `GraphExpansion(GraphExpansionErrorV1)` and stable code
`GraphExpansionError` at the Rust diagnostic layer.

### Python and TypeScript

Python exports `GraphQuerySeedV1` and `GraphExplicitSeedV1` frozen dataclasses
under the `GraphSeedV1` union; `CurrentGraphReadContextV1` and
`FrozenGraphReadContextV1` under `GraphReadContextV1`; and frozen dataclasses
named `GraphExpandRequestV1`, `ResolvedGraphSeedV1`, `GraphOriginV1`,
`GraphTargetV1`, `GraphTargetExplanationV1`,
`GraphExpansionExplanationV1`, and `GraphExpandResultV1`. Their fields are the
Rust fields above in declaration order and snake case. Python represents the
new u64 fields as canonical decimal `str`, u32 as `int`, nullable fields as
`None`, and closed enums as `Literal` unions. The two seed dataclasses have
exact field orders `(schema_version, type, text, ranked_limit)` and
`(schema_version, type, logical_ids)`; the two context dataclasses have
`(schema_version, type, context)`. Each `type` is the corresponding `Literal`.
The method is:

```python
fathomdb.graph.expand(
    engine: fathomdb.Engine,
    request: GraphExpandRequestV1,
) -> GraphExpandResultV1
```

TypeScript exports `GraphQuerySeedV1 | GraphExplicitSeedV1` as `GraphSeedV1`,
`CurrentGraphReadContextV1 | FrozenGraphReadContextV1` as
`GraphReadContextV1`, and interfaces named exactly like every request/response
struct above. Their properties are the canonical wire fields in declaration
order and camel case. TypeScript represents new u64 fields as decimal `string`,
u32 as `number`, nullable response fields as explicit `null`, and enums as
closed string-literal unions. The method is:

```ts
graph.expand(
  engine: Engine,
  request: GraphExpandRequestV1,
): Promise<GraphExpandResultV1>
```

The native PyO3 method is `Engine.graph_expand`; N-API is
`Engine.graphExpand`. The Python wrapper accepts only the exported request,
seed, and context carriers; the TypeScript wrapper validates the complete
runtime object before N-API. Passing a non-request carrier or a string with an
invalid native encoding is the existing Python `TypeError`/TypeScript
`InvalidArgumentError` FFI boundary. Every well-formed carrier refusal below is
the new typed graph-expansion family, except frozen authentication/state errors,
which deliberately retain the existing frozen-read family.

## Canonical wire schema and evolution

Rust/Python field names are snake case. TypeScript and canonical JSON are camel
case. Enum values are the lower-snake spellings in this document. Canonical
request objects are:

```text
GraphSeedV1 =
  { schemaVersion: 1, type: query, text: string, rankedLimit: u32 }
  | { schemaVersion: 1, type: explicit, logicalIds: IdSpace[1..25] }

GraphReadContextV1 =
  { schemaVersion: 1, type: current, context: ReadContextV1 }
  | { schemaVersion: 1, type: frozen, context: FrozenReadContextV1 }

GraphExpandRequestV1 {
  schemaVersion: 1,
  seed: GraphSeedV1,
  direction: incoming | outgoing | both,
  edgeKinds: string[0..32],
  targetKinds: string[0..32],
  context: GraphReadContextV1,
  maxDepth: u32,
  resultLimit: u32,
  maxWorkUnits: u64-decimal-string,
  includeExplanation: bool
}
```

Every response object shown in the Rust section carries `schemaVersion: 1`.
`GraphTargetV1.writeCursor` and `GraphExpandResultV1.workUnits` are canonical
unsigned-decimal strings in Python, TypeScript, and JSON; Rust uses `u64`.
Signs, whitespace, leading zeroes except the single value `0`, decimal points,
exponents, booleans, JSON numbers, and values above `u64::MAX` reject. `u32`
values use JSON numbers and reject booleans, fractions, negatives, and overflow.
Scores are finite JSON numbers. Optional response values are present as `null`.
Forbidden union members are absent.

Requests are recursively closed. At each versioned object, the decoder checks
`schemaVersion` first, then the lexicographically smallest unknown canonical
camel-case field, then required/type fields in declaration order. It enters a
nested object when that field is reached and applies the same rule there.
Unknown fields are identified before RFC 6901 escaping their path segment.
Direct typed Rust construction cannot carry unknown fields, so the canonical
codec tests provide the equivalent Rust-wire proof.

Response readers ignore additive unknown object fields. They reject an
unsupported/newer schema, an unknown enum/union variant, an incoherent union,
malformed integer or score, `complete != true`, a seed ordinal that is not its
zero-based array index, a target origin whose target ID differs from its target,
an explanation whose `perTarget` cardinality or indices differ from `targets`,
or unequal top-level/explanation degradation arrays. Canonical fixture field
order is declaration order.

## Request grammar and semantic validation

Every nested new schema version is exactly 1. The entire canonical request
encoding is at most 64 KiB.

- Query seed `text` must contain at least one non-whitespace Unicode scalar and
  `ranked_limit` is `1..=25`.
- Explicit seed count is `1..=25`. Every `IdSpace` must have exactly
  `space=logical`, a nonempty `value`, and no ASCII record separator `U+001E`.
  Exact duplicate values reject at the later index. Content/passage IDs reject;
  they are never interpreted as logical IDs.
- `direction` is closed and case-sensitive.
- Edge/target kinds are open exact strings. Each list has at most 32 members;
  each member must contain a non-whitespace scalar, and exact duplicates reject
  at the later index. Empty lists mean “any kind”. Values are bound SQLite
  parameters, never SQL fragments.
- `max_depth` is `0..=3`, `result_limit` is `1..=50`, and
  `max_work_units` is `1..=10_000`.
- `include_explanation` must be a real boolean.

After carrier/schema/unknown/type decoding, Engine semantic precedence is:

1. validate the context-union schema/discriminant;
2. for current mode, validate/canonicalize `ReadContextV1`; for frozen mode,
   authenticate the complete `FrozenReadContextV1` and database identity;
3. reject `include_superseded` or `include_inactive` because search projections
   and graph liveness are not version-complete; `include_out_of_window` remains
   the existing explicit temporal relaxation;
4. validate seed semantics, direction, edge kinds, target kinds, depth, result
   limit, work limit, and explanation flag, in that order; and
5. begin/pin one reader transaction, validate frozen state if applicable, then
   resolve seed visibility before reading graph rows.

Thus an unauthenticated frozen context outranks a semantically bad seed or
bound, while closed-wire shape errors that require no database access still
precede authentication. Frozen shape/authentication/database/drift/unavailable
failures retain `EngineError::FrozenRead`, `FDB_FROZEN_READ`, and the existing
nested field paths. No graph error contains submitted IDs, query text, bodies,
tokens, database identity, SQL, or candidate counts.

## Seed contract

### Explicit seeds

Explicit inputs preserve caller order; `seed_ordinal` is the zero-based input
index. Inside the pinned transaction every ID must resolve to one current,
active, dependency-eligible, temporally eligible node satisfying the complete
context eligibility filter. Resolution is all-or-nothing. An absent,
superseded, inactive, erased, closure-fenced, out-of-window, or filter-mismatched
seed returns `graph_seed_unavailable` at `/seed/logicalIds/<index>` and no
partial result. `query_score` is null.

### Query-derived seeds

Query mode executes the existing hybrid node retrieval once with these fixed
controls: no CE (`rerank_depth=0`), no graph arm (`use_graph_arm=false`),
`alpha=0.3`, `pool_n=0`, and no ordinary search explanation. A new internal
`logical_nodes_only` mode applies the context's existing indexed eligibility in
each text/vector candidate statement. The FTS arm also applies
`logical_id IS NOT NULL` before its candidate limit. The vector arm preserves
its accepted metadata-before-KNN and fixed-overfetch contract, but native
hydration rejects edge/anonymous rows before fusion and before the caller's
`ranked_limit`. Edge-body and anonymous/content candidates therefore never
enter the seed ranking. Results deduplicate by logical ID at their first ranked
position, then apply `ranked_limit`; that final order defines zero-based seed
ordinals. `query_score` is the finite existing fused score.

This is not “run ordinary top-K and discard nonlogical hits”: context
eligibility uses the accepted pre-candidate SQL routes, while node identity and
lifecycle/closure are enforced in the native candidate pipeline before fusion
and `ranked_limit`. A query with no matching logical node succeeds with empty
seeds/targets. Query text is not echoed in result, explanation, error, or
telemetry.

The query embedding may execute before the reader transaction because it reads
no database state. Every database read for query candidates, seed resolution,
projection classification, graph traversal, hydration, dependency/lifecycle
explanation, and final frozen-state validation runs inside the same pinned
reader transaction.

## Read-context and transaction contract

Current mode resolves `valid_as_of=None` exactly once after the reader
transaction is pinned and uses that instant everywhere. It validates declared
attribute projections in that transaction. The response does not mint or imply
a reusable frozen context.

Frozen mode authenticates before dispatch, then pins the reader transaction,
validates every bound authority from the token on that snapshot, performs the
complete operation, validates the same authority again before commit, and
returns. A pre-pin write causes typed drift; a post-pin write linearizes after
the read. There is no second search or expansion transaction.

Node visibility uses the context `ReadView`, indexed `SearchFilter`, and
dependency-closure eligibility for seeds, intermediate nodes, and outputs.
Erased rows are absent. Edges must be nonsuperseded and satisfy the same
effective temporal view. An explicitly relaxed temporal view may traverse
out-of-window nodes/edges; existence and dependency closure cannot be relaxed.

## Deterministic traversal and result semantics

Seeds are never targets, including when one seed is reachable from another.
At `max_depth=0`, no edge row is read: the response contains resolved seeds,
empty targets, `work_units="0"`, and `complete=true`.

Traversal is breadth first. The implementation maintains a visited set per
`(seed_ordinal, logical_id)`, so a node is expanded at most once per seed at its
shortest path, but another seed may independently reach it. The seed itself is
initially visited. Self-loops consume work but never enqueue or emit the seed.
Cycles terminate through the per-seed visited set.

At each depth, frontier states are ordered by `(seed_ordinal,
current_logical_id UTF-8 bytes)`. An endpoint-index probe first fetches at most
the globally remaining work budget plus one without a SQL `ORDER BY`. If that
batch fits, the Engine sorts it in this structural order before evaluation:

```text
(actual_direction where outgoing < incoming, edge kind UTF-8 bytes,
 next logical_id UTF-8 bytes, COALESCE(edge logical_id, '') UTF-8 bytes)
```

Structurally identical parallel rows are equivalent for origin selection;
their relative SQLite row order cannot alter a public value. Each parallel row
still consumes one work unit. In `both`, one edge row is enumerated once. Its
actual step is `outgoing` when the current node equals `from_id`, otherwise
`incoming`; a self-loop takes `outgoing` before visited rejection.

Each raw incident edge row is counted before edge-kind, edge liveness, endpoint
visibility, and visited checks. A row may enqueue its endpoint only if:

1. its kind matches `edge_kinds`, or the list is empty;
2. the edge is active and temporally eligible;
3. its endpoint is current, active, dependency-eligible, temporally eligible,
   and satisfies the context's complete indexed eligibility; and
4. that endpoint has not been visited for this seed.

`target_kinds` is output-only. A visible eligible intermediate of another kind
remains traversable and may reach a matching later target. A reached non-seed
node becomes a target candidate only if its kind matches `target_kinds` or that
list is empty.

One target is returned per logical ID. The retained origin is the minimum total
key:

```text
(hop_count, seed_ordinal, predecessor_logical_id UTF-8 bytes,
 terminal_direction where outgoing < incoming,
 terminal_edge_kind UTF-8 bytes, target_logical_id UTF-8 bytes)
```

Targets sort by the same key and only then apply `result_limit`. The traversal
must exhaust every reachable state through `max_depth` under the work bound
before success. Therefore `complete=true` means the returned array is the exact
top N of the complete bounded walk; it does not mean every matching target fit
in the response and it makes no continuation claim. `complete=false` is never
emitted in schema version 1.

## Exact work accounting and complexity

One work unit is one raw directional incident `canonical_edges` row delivered
to the traversal enumerator for one seed frontier state. It is charged before
all semantic filters so a high-degree node cannot hide unbounded scanning behind
a selective kind or eligibility predicate. Reaching/hydrating nodes, bounded
seed resolution, projection metadata, and explanation lookups do not consume
graph work units; each is independently bounded by at most 25 seeds, 50 returned
targets, or the visited/frontier set implied by charged edges.

For budget `W`, each unsorted SQL endpoint probe obtains at most
`remaining + 1` rows through its endpoint index; only an in-budget batch is
sorted in memory. If no more than `W` rows are observed across all frontier
states, success reports their exact count. Observation of row `W+1` immediately
returns `graph_expansion_bound_exceeded` at
`/maxWorkUnits`, with no seeds, targets, explanation, partial work count, or
continuation. Exactly W rows may succeed. A caller cannot distinguish W+1 from
W+n through the error.

Peak traversal storage is `O(S + W + min(T, 50))` where `S <= 25`,
`W <= 10_000`, and `T` is unique eligible targets. Each endpoint scan is driven
by the existing `from_id` or `to_id` index; `both` uses SQLite multi-index OR or
two ordered index probes merged by the Engine. Sorting is bounded by W. The raw
row accounting makes a compound `(endpoint, kind)` index unnecessary and avoids
a Slice 60 migration. Query-plan gates prohibit a full-table
`SCAN canonical_edges` for either direction.

## Explanation and projection behavior

`degradation_codes` is always present, sorted in the enum order above, and
deduplicated. `include_explanation=false` returns `explanation=null`; it does
not hide material degradation. When requested, one content-free random
`correlation_id` is generated for the call. It is not persisted and is never
derived from a query, seed, target, database identity, or token.

`per_target` has exactly one entry for each target at the same zero-based
`target_index`; it repeats that target's compact origin, classifies lifecycle as
`node_active`, and reuses the Slice 55 dependency classifier:
`not_applicable` for a class that does not require a dependency,
`not_registered` when the class requires one but none is registered, or
`registered`. No body, source bytes, excluded ID/path, count of excluded rows,
or query text appears in the sidecar. Expanded exclusion/not-selected
explanation remains deferred.

Explicit seeding does not consume a retrieval projection:
`projection_generation_id=null`, origin/readiness are `not_applicable`, and it
cannot emit a projection degradation. Query seeding loads the exact Slice 40
generation ID, origin, and readiness observed inside the reader transaction.

The hard/soft matrix is:

| Condition | Outcome |
| --- | --- |
| Explicit seeds | Projection fields not applicable; graph reads canonical tables. |
| Query dense generation ready and runtime usable | Normal hybrid seed retrieval; no degradation. |
| Existing search reports a vector soft fallback while synchronous FTS is usable | Native logical-node FTS seeding succeeds and emits `query_seed_text_fallback`. |
| Generation origin/readiness is legacy-unverified, processing, blocked, deferred, or degraded | Preserve whatever node arms the existing search can truthfully serve and emit the exact matching projection classification code; do not infer arm availability from readiness alone. |
| Query returns no eligible logical node | Successful empty result; projection classification remains truthful. |
| Vector-equivalence self-check refused vector-dependent search | Preserve existing `VectorEquivalenceMismatchError`; do not silently relabel it as fallback. |
| No native logical-node search arm is serviceable | `graph_projection_unavailable` at `/projection`; no result. |
| Projection authority, frozen authority, or graph storage is corrupt/unreadable | Typed existing storage/projection/frozen failure, or `graph_corrupt` where the graph-specific invariant is known; no result. |
| Work row W+1 | `graph_expansion_bound_exceeded`; no result. |

This slice does not claim graph recall or semantic correctness. The operation is
a deterministic structural primitive.

## Error contract and FFI mapping

```text
GraphExpansionErrorReasonV1 =
  unsupported_schema_version | unknown_field |
  graph_seed_invalid | graph_direction_invalid |
  graph_edge_kinds_invalid | graph_target_kinds_invalid |
  graph_context_invalid | graph_depth_invalid |
  graph_result_limit_invalid | graph_work_limit_invalid |
  graph_seed_unavailable | graph_expansion_bound_exceeded |
  graph_projection_unavailable | graph_corrupt

GraphExpansionErrorV1 {
  schema_version: 1,
  reason: GraphExpansionErrorReasonV1,
  field_path: RFC-6901 string
}
```

Rust maps this through `EngineError::GraphExpansion`. Python exports
`GraphExpansionError` with `.reason` and `.field_path`; TypeScript exports
`GraphExpansionError` with `reason` and `fieldPath`. Both dynamic bindings use
stable code `FDB_GRAPH_EXPANSION`. Frozen failures remain
`FrozenReadError/FDB_FROZEN_READ`; existing vector, projection, closing,
overload, and storage families remain unchanged.

Exact semantic paths are `/seed`, `/seed/type`, `/seed/text`,
`/seed/rankedLimit`, `/seed/logicalIds`,
`/seed/logicalIds/<index>[/space|/value]`, `/direction`,
`/edgeKinds[/<index>]`, `/targetKinds[/<index>]`, `/context`,
`/maxDepth`, `/resultLimit`, `/maxWorkUnits`, and
`/includeExplanation`. Unsupported nested versions point to their exact nested
`/schemaVersion`; unknown fields point to the exact escaped member. Graph
corruption is `/targets` when a stored graph invariant prevents a trustworthy
target/origin and `/projection` for graph-specific projection authority.

## TDD RED/GREEN and verification contract

Tests use real SQLite databases. RED is committed separately and test files are
frozen through GREEN.

1. **Wire/codec property matrix:** round-trip every request/response/union/enum;
   shuffled response fields and additive response fields; schema/unknown-field
   precedence at every object; malformed u64/u32/finite-score values; union
   incoherence; response cardinality/index/origin corruption; Python/TS exact
   RFC 6901 paths and stable error codes.
2. **Seed matrix:** explicit order, duplicate and nonlogical IDs, all-or-nothing
   invisible/erased/closure-fenced IDs, query-only logical candidates before
   `ranked_limit`, edge/anonymous candidates above logical hits, query
   deduplication, zero matches, fixed no-CE/no-graph settings, and exact seed
   ordinals/scores.
3. **Constraint matrix:** incoming/outgoing/both; empty/single/multiple edge
   kinds; open absent kinds; empty/single/multiple target kinds; the required
   `A(kind X) -> B(kind Y) -> C(kind X)` return-only target-kind case; every
   context eligibility axis below an unfiltered candidate cap.
4. **Traversal matrix:** depth 0/1/2/3 and 4 refusal, global seed exclusion,
   self-loops, cycles, parallel edges, same target from multiple paths/seeds,
   deterministic first origin, shuffled insertion batches, close/reopen, and
   byte-identical canonical response digests across permutations.
5. **Bounds:** exact result limits 1/50 and 0/51 refusals; work limits 1/10,000
   and 0/10,001 refusals; high-degree exactly-W success and W+1 no-result
   failure for every direction; result-limit top-N after complete traversal;
   memory/RSS ceiling proportional to W rather than database size.
6. **Context races:** a deterministic rendezvous before transaction pin and
   after pin proves current one-transaction linearization and frozen pre-pin
   drift/post-pin isolation. The rendezvous is cancellation-safe, bounded, and
   released on Drop, following the Slice 55 deadlock correction.
7. **Liveness/projection/explanation:** node/edge supersession, lifecycle,
   dependency closure, erasure, temporal relaxation, every hard/soft projection
   row, content-free correlation, per-target association/cardinality, and
   explanation-off identical targets/work/database bytes.
8. **Plans/schema/nonregression:** exact endpoint-index query plans for incoming,
   outgoing, and both; no `SCAN canonical_edges`; schema stays 33 and migration
   manifest is unchanged; default search and existing graph methods retain
   signatures, results, and query plans.

Focused GREEN runs precede fast, heavy, all, and applicable all-feature routes.
Final verification includes full-workspace fmt/clippy/check, markdown/design/
reference/release-view gates, fresh Linux Rust/Python wheel/independently packed
Node consumers, and an exact-source Windows archive producing fresh native
Rust/Python/Node modules. Each installed consumer runs the shared request,
response, malformed-input, direction/kind, depth-zero, W/W+1, and frozen-race
fixtures and records artifact/source hashes. Disposable artifacts are removed
after evidence is durable.

CUDA/Metal are N/A when the diff leaves the existing query embedding and dense
dispatch unchanged. If query seeding changes device dispatch, run the same seed
fixture on CPU plus every compiled supported device and prove identical logical
seed order; otherwise no device claim is made. Operator, live-model, registry,
packaging, tag, publication, and post-publication routes are outside this slice.

## Readiness rule

This design is `FIX1_AWAITING_REVIEW`. Slice 7 and Slice 55 are complete, but a
second independent design review must verify that every Cycle 1 P1/P2 finding
is closed before the design may become `READY`. No source or test implementation
is authorized by this document's current status.
