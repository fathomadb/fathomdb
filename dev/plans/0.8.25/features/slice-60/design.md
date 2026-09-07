---
title: 0.8.25 Slice 60 — minimal constrained combined-expansion design
status: READY
design_version: 5
review_fix: 3
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

The default facade re-exports every type below. Attribute placement is part of
the contract: request carriers stay exhaustive so external callers can
construct them with literals, response structs are non-exhaustive for additive
response evolution, and every schema-v1 enum remains exhaustive because its
wire vocabulary is closed.

```rust
#[derive(Clone, Debug, Eq, PartialEq)]
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

#[derive(Clone, Debug, Eq, PartialEq)]
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

#[derive(Clone, Debug, Eq, PartialEq)]
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

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct ResolvedGraphSeedV1 {
    pub schema_version: u32,
    pub logical_id: String,
    pub seed_ordinal: u32,
    pub query_score: Option<f64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
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

#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct GraphTargetV1 {
    pub schema_version: u32,
    pub logical_id: String,
    pub kind: String,
    pub body: String,
    pub write_cursor: u64,
    pub origin: GraphOriginV1,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum GraphExpansionDegradationCodeV1 {
    QuerySeedTextFallback,
    ProjectionLegacyUnverified,
    ProjectionProcessing,
    ProjectionBlocked,
    ProjectionDeferred,
    ProjectionDegraded,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct GraphTargetExplanationV1 {
    pub schema_version: u32,
    pub target_index: u32,
    pub origin: GraphOriginV1,
    pub lifecycle_state: StructuralLifecycleStateV1,
    pub dependency_state: StructuralDependencyStateV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
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

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
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

The remaining public enums have these exact attributes and variants:

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GraphSeedSourceV1 { Query, Explicit }

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GraphReadModeV1 { Current, Frozen }

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GraphProjectionOriginV1 {
    NotApplicable, Fresh, LegacyUnverified, Configuration, Rebuild,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GraphProjectionReadinessV1 {
    NotApplicable, Ready, Processing, Blocked, Deferred, Degraded,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GraphExpansionErrorReasonV1 {
    UnsupportedSchemaVersion, UnknownField, GraphSeedInvalid,
    GraphDirectionInvalid, GraphEdgeKindsInvalid, GraphTargetKindsInvalid,
    GraphContextInvalid, GraphDepthInvalid, GraphResultLimitInvalid,
    GraphWorkLimitInvalid, GraphSeedUnavailable,
    GraphExpansionBoundExceeded, GraphProjectionUnavailable, GraphCorrupt,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphExpansionErrorV1 {
    pub schema_version: u32,
    pub reason: GraphExpansionErrorReasonV1,
    pub field_path: String,
}
```

No type above has an implicit `Default`, serialization derive, hash derive, or
ordering derive beyond those shown. In particular, types containing
`query_score: Option<f64>` derive `PartialEq` but not `Eq`. The exhaustive
`GraphExpandRequestV1`, `GraphSeedV1`, and `GraphReadContextV1` definitions are
the exact external construction path; Slice 60 adds no builder or constructor
that could apply hidden defaults. Response structs alone carry
`#[non_exhaustive]`; their fields remain publicly readable.

The lower-snake wire spellings are `query | explicit`, `current | frozen`,
`not_applicable | fresh | legacy_unverified | configuration | rebuild`, and
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
zero-based array index, or any of these response-coherence violations:

1. each target origin's `seedOrdinal` must be an in-range index into `seeds`;
2. its `seedLogicalId` must exactly equal
   `seeds[seedOrdinal].logicalId`;
3. its `targetLogicalId` must exactly equal its enclosing target's
   `logicalId`;
4. explanation `perTarget` cardinality and zero-based `targetIndex` values must
   exactly match `targets`; and
5. each explanation origin must be field-for-field equal to the corresponding
   `targets[targetIndex].origin`, and its degradation array must equal the
   top-level array.

The exact failure paths are `/targets/<i>/origin/seedOrdinal`,
`/targets/<i>/origin/seedLogicalId`,
`/targets/<i>/origin/targetLogicalId`,
`/explanation/perTarget`,
`/explanation/perTarget/<i>/targetIndex`,
`/explanation/perTarget/<i>/origin`, and
`/explanation/degradationCodes`, respectively. These are binding-contract
decode failures, not evidence of stored database graph corruption. Their public
mapping nevertheless reuses this operation's typed family, following Slice
50's strict malformed-native-response precedent: an unsupported response
schema raises reason `unsupported_schema_version` at that object's exact
`/.../schemaVersion`; every other malformed native response raises reason
`graph_corrupt` at the first exact camel-case path selected by the validation
order above.

Python raises exported `fathomdb.GraphExpansionError` with
instance/class `.code == "FDB_GRAPH_EXPANSION"` plus
`.reason`/`.field_path`; TypeScript raises exported `GraphExpansionError` with
`static readonly code = "FDB_GRAPH_EXPANSION"` plus
`.reason`/`.fieldPath`. This follows the shipped `DependencyTraceError`
exception/code installation pattern. Native error translation and wrapper-side
response validation therefore share stable code `FDB_GRAPH_EXPANSION`; the
fixed message is `<reason> at <path>`. They never
surface as Python `ValueError`/`TypeError`, a bare TypeScript `FathomDbError`,
TypeScript `InvalidArgumentError`, a frozen-read error, or a storage error. Shared
Rust-wire/Python/TypeScript malformed-native fixtures cover unsupported top and
nested response schemas plus every coherence path above and assert exact
exception class, code, reason, and path. Canonical fixture field order is
declaration order.

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
   and graph liveness are not version-complete; `include_out_of_window` is the
   existing explicit relaxation for node validity only and never relaxes edge
   recency;
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
active, dependency-eligible node satisfying the complete effective node
`ReadView` and context eligibility filter. Node temporal eligibility is
required when `include_out_of_window=false` and omitted when it is `true`.
Resolution is all-or-nothing. An absent, superseded, inactive, erased,
closure-fenced, filter-mismatched, or unrelaxed out-of-window seed returns
`graph_seed_unavailable` at `/seed/logicalIds/<index>` and no partial result.
`query_score` is null.

### Query-derived seeds

Schema 33 exposes indexed vec0 metadata for accepted search predicates but no
indexed logical-node-identity discriminator. Consequently schema-v1 graph
query seeding **declines the entire vector arm before embedding and before any
KNN statement**. It must not run fixed-overfetch bit-KNN and discard
edge/anonymous rows afterward: more than `TOP_K_BIT_CANDIDATES` nearer
nonlogical rows would starve a valid logical seed and violate A25-06 and the
accepted predicate-before-KNN rule. Adding vec0 metadata or a migrated vector
partition is outside this no-migration slice.

Query mode therefore executes one native logical-node FTS statement, sharing
the existing tokenizer/ranking and applying `logical_id IS NOT NULL`, the
current nonsuperseded node predicate, and every context indexed-eligibility
predicate in SQL before its candidate limit. It requests at most
`ranked_limit` eligible rows, orders them by the existing FTS score and stable
row tie-break, deduplicates by logical ID at first position, and uses that order
as zero-based seed ordinals. `query_score` is that finite existing FTS score.
The operation emits `query_seed_text_fallback` for every query-seeded success,
including an empty success, because the dense arm was deliberately declined;
explicit seeding never emits it.

This is not ordinary hybrid search and it performs no CE, graph arm, fusion,
query embedding, dense device dispatch, or vec0 KNN. Edge-body and
anonymous/content candidates cannot enter the statement. A query with no
matching logical node succeeds with empty seeds/targets. Query text is not
echoed in result, explanation, error, or telemetry. Every database read for
FTS candidates, seed resolution, projection classification, graph traversal,
hydration, dependency/lifecycle explanation, and final frozen-state validation
runs inside the same pinned reader transaction.

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
Erased rows are absent. `include_out_of_window=true` drops only the node
validity-window predicate for explicit/query seeds, intermediate nodes, and
outputs; with `false`, each node must satisfy the effective `ReadView` window.
Every edge always follows the accepted shipped graph admission rule: it is
nonsuperseded and `t_invalid IS NULL OR t_invalid > effective_instant`.
`t_valid` is retained provenance and never gates this operation, even when it
is later than the effective instant. Thus temporal relaxation cannot restore
an edge whose `t_invalid` is equal to or earlier than the instant; existence
and dependency closure likewise cannot be relaxed.

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
2. the edge is nonsuperseded, and its `t_invalid` is absent or strictly later
   than the effective instant, regardless of its `t_valid` and
   `include_out_of_window`;
3. its endpoint is current, active, dependency-eligible, satisfies the
   context's complete indexed eligibility, and satisfies node temporal
   eligibility unless `include_out_of_window=true`; and
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
not hide material degradation. When requested, graph expansion reuses the
existing Engine explanation-only allocator and its exact grammar:
`x<32-lower-hex-open-nonce>-<canonical-u64-seq>`. The open nonce is the
Engine's already-minted `explanation_open_nonce`; the sequence is the shared
`explanation_sequence.fetch_add(1, Relaxed)` used by explained search. Graph
expansion has no telemetry event schema and therefore never selects or mints a
`q<nonce>-<seq>` telemetry ID. It allocates exactly one `x...` value after the
reader result succeeds and before that result escapes, only when
`include_explanation=true`; a failed or explanation-off call consumes no
sequence value. The ID is not persisted and is never derived from a query,
seed, target, database identity, or token.

Tests validate the grammar
`^x[0-9a-f]{32}-(0|[1-9][0-9]*)$`, uniqueness for concurrent successful
explained calls, and the explanation-off allocation-free path. For
byte/digest/permutation comparisons only, each binding's test harness validates
the real value and then replaces that response field with the single sentinel
`x00000000000000000000000000000000-0` before canonical encoding. Tests never
control the nonce/sequence, compare literal live IDs across calls, or rewrite a
golden fixture from generated output.

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
That metadata is diagnostic context; query graph expansion still declines the
dense arm before KNN regardless of runtime state.

Code composition is exhaustive and mechanical. Start with an empty vector,
then apply these three mappings and finally sort by the declared
`GraphExpansionDegradationCodeV1` enum order and deduplicate:

| Axis | Value | Appended code |
| --- | --- | --- |
| Seed route | explicit | none |
| Seed route | query (FTS route; vector declined pre-KNN) | `query_seed_text_fallback` |
| Projection origin | `not_applicable`, `fresh`, `configuration`, or `rebuild` | none |
| Projection origin | `legacy_unverified` | `projection_legacy_unverified` |
| Projection readiness | `not_applicable` or `ready` | none |
| Projection readiness | `processing` | `projection_processing` |
| Projection readiness | `blocked` | `projection_blocked` |
| Projection readiness | `deferred` | `projection_deferred` |
| Projection readiness | `degraded` | `projection_degraded` |

`not_applicable` origin/readiness and a null generation ID occur together only
for explicit seeds. Query seeds carry a non-null generation ID and the exact
typed Slice 40 origin/readiness pair. Thus the required
`legacy_unverified + degraded` query case produces, in exact order,
`[query_seed_text_fallback, projection_legacy_unverified,
projection_degraded]`; neither code masks the other. Other multi-axis cases
compose identically. Top-level and explanation arrays are byte-for-byte equal.

The hard/soft outcome matrix is:

| Condition | Outcome |
| --- | --- |
| Explicit seeds | Projection fields not applicable; graph reads canonical tables; no degradation. |
| Any query-seeded success, including zero eligible seeds | Native logical-node FTS seeding; apply the exhaustive composition above. |
| Dense runtime absent, refused, processing, blocked, deferred, or degraded | Do not initialize/embed/query it; FTS success remains soft and truthfully reports the observed status codes. |
| Synchronous logical-node FTS route unavailable | `graph_projection_unavailable` at `/projection`; no result. |
| Projection authority, frozen authority, or graph storage corrupt/unreadable | Typed existing storage/projection/frozen failure, or `graph_corrupt` where the graph-specific invariant is known; no result. |
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
   incoherence; and shared Rust-wire/Python/TypeScript malformed-response
   fixtures for out-of-range origin `seedOrdinal`, origin seed-ID mismatch,
   origin target-ID mismatch, explanation target-index/cardinality mismatch,
   and explanation-origin mismatch at the exact RFC 6901 paths above.
2. **Seed matrix:** explicit order, duplicate and nonlogical IDs, all-or-nothing
   invisible/erased/closure-fenced IDs, query-only logical candidates before
   `ranked_limit`, query deduplication, zero matches, and exact seed
   ordinals/scores. A required real-database fixture inserts more than
   `TOP_K_BIT_CANDIDATES` nearer vector edge/anonymous rows plus a farther
   FTS-matching logical node, proves the logical seed is returned, and uses the
   vector/embedder test seam to prove no embedding or KNN statement executed.
3. **Constraint matrix:** incoming/outgoing/both; empty/single/multiple edge
   kinds; open absent kinds; empty/single/multiple target kinds; the required
   `A(kind X) -> B(kind Y) -> C(kind X)` return-only target-kind case; every
   context eligibility axis below an unfiltered candidate cap.
4. **Traversal matrix:** depth 0/1/2/3 and 4 refusal, global seed exclusion,
   self-loops, cycles, parallel edges, same target from multiple paths/seeds,
   deterministic first origin, and close/reopen. The byte-identity permutation
   oracle uses explicit seeds, `include_explanation=false`, a fixed current
   context with an explicit validity instant, and node rows inserted once in a
   fixed order so their bodies and exposed write cursors are identical. Each
   database copy then inserts only the same edge multiset in a different order,
   with no subsequent node write. Canonical complete response bytes must match
   across those edge-only permutations. Query-seed permutations and arbitrary
   node insertion permutations are not byte-identity oracles; their exposed
   node cursors and equal-score tie-break inputs may legitimately differ.
5. **Bounds:** exact result limits 1/50 and 0/51 refusals; work limits 1/10,000
   and 0/10,001 refusals; high-degree exactly-W success and W+1 no-result
   failure for every direction; result-limit top-N after complete traversal;
   memory/RSS ceiling proportional to W rather than database size.
6. **Context races:** a deterministic rendezvous before transaction pin and
   after pin proves current one-transaction linearization and frozen pre-pin
   drift/post-pin isolation. The rendezvous is cancellation-safe, bounded, and
   released on Drop, following the Slice 55 deadlock correction.
7. **Liveness/projection/explanation:** node/edge supersession, lifecycle,
   dependency closure, erasure, every hard/soft projection row and ordered
   multi-code composition, correlation grammar/allocation/normalization,
   per-target association/cardinality, and explanation-off identical
   targets/work/database bytes. The temporal matrix fixes one instant and
   crosses in-window/out-of-window nodes with edges whose `t_invalid` is null,
   equal to, earlier than, or later than that instant. Null/later edges are
   admitted and equal/earlier edges are excluded in both relaxation modes.
   Separate rows set `t_valid` later than the instant and prove it never gates.
   `include_out_of_window=true` may restore only the node cases, for current and
   frozen reads and every traversal direction.
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

CUDA/Metal are N/A: this schema-v1 operation is required to decline the vector
arm before embedding/KNN and must not enter dense device dispatch. Operator,
live-model, registry, packaging, tag, publication, and post-publication routes
are outside this slice.

## Readiness rule

This design is `READY`. Slice 7 and Slice 55 are complete, and the fourth
independent design review—the final review allowed by the four-cycle cap—found
no P0, P1, P2, or optional P3 findings. Implementation is authorized under the
handoff's RED/GREEN, independent-review, and CI-only boundary.
