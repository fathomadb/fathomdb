---
title: 0.8.25 Slice 55 — basic tracing, explanation, and integrity design
status: DRAFT_FIX_1_REVIEW_REQUIRED
design_version: 6
review_fix: 1
target_release: 0.8.25
depends_on: 50
architecture: dev/design/fathomdb-data-plane-architecture-v2.md
---

# Slice 55 design

## Authority and retained boundary

This slice implements S55-R1 through S55-R7 below: the retained basic subset
of R25/AC25-55, Memex needs 19/20 and its share of 12, N25-02, and
A25-05/A25-06. It adds one-page reciprocal dependency tracing, synchronous
bounded data-plane integrity checks, and a compact additive amendment to the
existing explanation sidecar. It explains structural inclusion and
degradation, never semantic truth, entailment, relevance, exclusion from an
arbitrary candidate set, or answer quality.

The owner-approved scope adjustment excludes persisted trace pages, trace
leases, exhaustive exclusion/not-selected explanation, frozen integrity jobs,
repair plans, reverse-index construction, and repair orchestration. Slice 55
is read-only and adds no schema migration or persistent state.

All prerequisites through Slice 50, including Slice 7 architecture activation,
are complete. This FIX-1 draft is not READY: an independent design re-review
must close every Cycle 1 P1/P2 finding first.

## Requirements and acceptance

- **S55-R1 — non-colliding surfaces.** Preserve the existing operator-only
  `trace_source_ref` and `check_integrity` contracts while adding exact,
  separately named application trace and operator data-plane integrity APIs.
- **S55-R2 — reciprocal trace.** For the same frozen context, source-to-derived
  and derived-to-source reads return the same registered Slice 20 relation and
  endpoints in opposite traversal directions.
- **S55-R3 — bounded snapshot.** Trace and integrity each linearize in one
  SQLite reader transaction, use deterministic order, and enforce both output
  and counted-work caps with cap-plus-one detection.
- **S55-R4 — real dependency authority.** Validate Slice 20's normalized
  dependency row and complete Slice 15 provenance chain; never assume a
  separately stored reverse row or reverse index.
- **S55-R5 — authoritative integrity.** Diagnose dependency-chain,
  searchable-orphan, projection-generation, and mutation-readiness faults by
  reusing Slice 40 membership/completion classification.
- **S55-R6 — explanation successor.** Extend the ratified
  `Explanation { trace, per_hit }` carrier and positionally associated
  `PerHitExplain` without changing any default search result or telemetry
  privacy rule.
- **S55-R7 — wire and nondisclosure.** Every new request, response, enum, and
  error has an exact A25-05 Rust/Python/TypeScript or operator-CLI mapping,
  deterministic validation, and privacy-safe refusal precedence.

The corresponding acceptance rows are:

- **S55-AC1:** reciprocal traces are deterministic, complete, one-page,
  context-bound, and all-or-error at either cap;
- **S55-AC2:** absent, erased, superseded, closure-fenced, ineligible, foreign,
  and context-drifted trace subjects disclose no identity or existence detail;
- **S55-AC3:** integrity work is measurably bounded per check and in aggregate,
  with exact `checked_count` and no partial success;
- **S55-AC4:** every row of the dependency/projection/orphan/readiness finding
  matrix has a real-database corruption fixture and stable finding code;
- **S55-AC5:** explanation correlation, positional structural inclusion, safe
  telemetry subset, degradation codes, and telemetry-off behavior agree across
  Rust, Python, and TypeScript;
- **S55-AC6:** v1 codec fixtures pin field order, unknown behavior, integer
  spelling, validation precedence, RFC 6901 paths, and cross-SDK equality; and
- **S55-AC7:** no check mutates database/WAL bytes, no default search path adds
  work or changes results, and legacy operator reports remain accepted.

## Code-grounded exists-versus-net-new map

| Concern | Exists at Slice 50 | Slice 55 decision |
|---|---|---|
| Legacy source census | Operator-only `Engine::trace_source_ref(&str) -> TraceReport`; CLI `doctor trace --source-ref`; events are canonical rows selected by `source_id`. | Preserve unchanged. Add a dependency-revision trace with a different name and typed result. |
| Legacy doctor integrity | Operator-only `Engine::check_integrity(CheckIntegrityOpts) -> IntegrityReport`; CLI `doctor check-integrity`; report has exactly physical/logical/semantic sections. | Preserve flags, report, CLI JSON, and accepted tests unchanged. Add a separately named bounded data-plane diagnostic. |
| SDK doctor boundary | Python and TypeScript explicitly expose no doctor/recovery methods. | Keep data-plane integrity operator-only; do not add Python/TypeScript methods. |
| Dependency authority | `_fathomdb_source_dependencies` stores dependency ID, derived revision, and registered generation; `_fathomdb_source_links` stores the authoritative source revision. Both lookup directions join them. | Validate this one normalized chain. Add no reverse row/index/table. |
| Read authority | `FrozenReadContextV1` authenticates database, visibility/filter envelope, effective instant, write/dependency/projection state. | Require it on every application trace. One existing reader transaction owns validation and collection. |
| Explanation | `SearchResult.explanation: Option<Explanation>`; `Explanation` contains `QueryTrace` plus one positional `PerHitExplain` per result. | Add one content-free correlation ID and one versioned structural-inclusion object per `PerHitExplain`. |
| Telemetry correlation | Opt-in local telemetry mints a content-free `query_id`; telemetry is off by default. | Mint one correlation identity for a search and reuse it as telemetry `query_id` when telemetry is on. No second `operation_id`. |
| Projection authority | Slice 40 owns generation identity, physical membership, dense completion, and `ProjectionGenerationError`. | Call the shared classifiers. Integrity translates classified corruption to findings; it does not copy their SQL or redefine readiness. |

## Exact public ownership and naming

### Governed application trace

The default Rust engine and `fathomdb` facade add:

```text
Engine::trace_dependency(
  request: DependencyTraceRequestV1
) -> Result<DependencyTraceResultV1, EngineError>
```

The facade re-exports all `DependencyTrace*V1`, `Trace*V1`, and
`DependencyTraceError*` public types. `trace_dependency` is added to the
governed application-method allowlist. It is a read and is not behind the
`operator` feature.

Python adds `Engine.trace_dependency(request: DependencyTraceRequestV1) ->
DependencyTraceResultV1`. TypeScript adds
`Engine.traceDependency(request: DependencyTraceRequestV1):
Promise<DependencyTraceResultV1>`. Both expose typed request/result objects and
`DependencyTraceError`; neither exposes a doctor namespace.

### Operator data-plane integrity

The `operator` feature adds:

```text
Engine::check_data_plane_integrity(
  request: DataPlaneIntegrityRequestV1
) -> Result<DataPlaneIntegrityResultV1, EngineError>
```

The method and its types are operator-seam facade exports and operator
allowlist members only. The CLI adds:

```text
fathomdb doctor data-plane-integrity \
  --check <kind>... --max-work <1..10000> --max-findings <1..100> \
  [--json] <db_path>
```

Repeated `--check` values form the request list. With no `--check`, the CLI
uses all four kinds in normative order. Python and TypeScript add no integrity
method, options, or report type.

`check_integrity` does not call, append, or embed the new report, and
`check_data_plane_integrity` does not manufacture the legacy three-section
report. They may share existing private SQLite helpers only where the meaning
is identical. Likewise, `trace_source_ref` remains a `source_id` census and
does not call `trace_dependency`; the new trace never accepts `source_id`.

## Reciprocal dependency trace contract

### Types and request defaults

```text
DependencyTraceDirectionV1 = to_source | to_dependents
TraceArtifactRoleV1 = canonical_source | derived
TraceArtifactClassV1 = node | edge

DependencyTraceRequestV1 {
  schema_version: 1,
  root_revision_id: string,
  direction: DependencyTraceDirectionV1,
  context: FrozenReadContextV1,
  max_relations: u32 = 100,
  max_work_units: u32 = 101,
}

TraceNodeLifecycleV1 =
  node { schema_version: 1, artifact_class: node,
         state: pending | active | deleted, superseded: bool,
         valid_at_effective: bool }
  | edge { schema_version: 1, artifact_class: edge,
           superseded: bool, valid_at_effective: bool }

DependencyTraceNodeV1 {
  schema_version: 1,
  artifact_revision_id: string,
  artifact_class: TraceArtifactClassV1,
  role: TraceArtifactRoleV1,
  depth: u32,
  lifecycle: TraceNodeLifecycleV1,
}

DependencyTraceEdgeV1 {
  schema_version: 1,
  dependency_id: string,
  source_revision_id: string,
  derived_revision_id: string,
  registered_dependency_generation: u64,
}

TraceReadBoundaryV1 {
  schema_version: 1,
  effective_at_epoch_s: i64,
  observed_write_boundary: u64,
  dependency_generation: u64,
  projection_generation_id: string,
}

DependencyTraceResultV1 {
  schema_version: 1,
  root_revision_id: string,
  direction: DependencyTraceDirectionV1,
  nodes: [DependencyTraceNodeV1],
  dependency_edges: [DependencyTraceEdgeV1],
  checked_work_units: u32,
  complete: true,
  read_boundary: TraceReadBoundaryV1,
}
```

Rust `DependencyTraceRequestV1::new(root_revision_id, direction, context)` sets
the defaults. `.with_bounds(max_relations, max_work_units)` validates an
override. Python snake-case and TypeScript camel-case request objects may omit
only the two bound fields; omission selects those same defaults. `null` never
means omission.

`max_relations` is `1..=100`; `max_work_units` is `1..=101`. The fixed hard
ceilings are not configurable. A work unit is one root artifact candidate or
one registered dependency candidate read from the normalized, fixed-width
joined relation. Each joined candidate may touch only the dependency row,
derived owner, derived source link, canonical owner/node, source-version row,
canonical self-link, and singleton generation. Internal SQLite virtual-machine
steps are not the unit. Indexed statements use the remaining cap plus one, so
the Engine can detect overflow without scanning the remainder.

### Direction, inclusion, ordering, and one snapshot

`to_source` requires a visible complete derived root. `to_dependents` requires
a visible complete canonical-source node root. A visible root with the wrong
role returns `trace_root_role_invalid`. A valid result always includes the root
at depth 0. Each included counterpart is depth 1. Nodes are ordered root first,
then counterpart artifact revision ID. Edges are ordered by
`(derived_revision_id, dependency_id)`. There is no traversal beyond one
registered relation, no continuation, and no partial result.

The trace opens one SQLite deferred reader transaction, authenticates the
required `FrozenReadContextV1` in that transaction, resolves its one effective
instant, captures every boundary field, validates the root, and collects the
joined relation before commit. The transaction is the linearization point.
No state-generation retry loop or before/after comparison substitutes for the
snapshot.

The relation itself has no active/inactive state. Eligibility applies to the
registered relation's two endpoints and all Slice 30 closure barriers. The
root and counterpart must independently satisfy the exact authenticated view,
attribute eligibility, lifecycle/currentness, temporal validity, erasure, and
physical closure fences. An ineligible counterpart is omitted, not returned
with `visible=false`. Thus, under the same context, any returned edge is
reciprocal: tracing its source `to_dependents` and derived endpoint `to_source`
returns the same `SourceDependencyV1` identity.

An eligible derived root without a registered Slice 20 relation returns only
the root. An eligible source with no eligible registered dependents does the
same. Missing, erased, superseded, closure-fenced, or ineligible roots all
return `trace_unavailable` at `/rootRevisionId`. That nondisclosing outcome
precedes role and corruption detail. A visible root whose registered chain is
malformed returns `trace_corrupt` without endpoint IDs. Trace returns no source
body, payload, query, predicate value, `source_id`, source version, locator, or
hash; Slice 50 remains the authorized exact-source path.

If either cap-plus-one probe observes excess work/relations, the call returns
`trace_bound_exceeded`, no result, and no partial IDs. `checked_work_units`
therefore appears only on complete success and equals one plus the number of
registered relation candidates fully classified, including candidates omitted
because their counterpart was ineligible.

## Bounded data-plane integrity

### Request, result, work, and snapshot

```text
DataPlaneIntegrityCheckV1 =
  dependency_chain | active_searchable_orphans |
  projection_generation | mutation_readiness

DataPlaneIntegrityRequestV1 {
  schema_version: 1,
  checks: [DataPlaneIntegrityCheckV1],
  max_work_units: u32,
  max_findings: u32,
}

DataPlaneIntegritySeverityV1 = error | critical

DataPlaneIntegrityFindingCodeV1 =
  dependency_row_invalid | dependency_derived_owner_missing |
  dependency_derived_role_invalid | dependency_source_link_missing |
  dependency_source_link_mismatch | dependency_source_owner_missing |
  dependency_source_role_invalid | dependency_source_version_mismatch |
  dependency_source_self_link_mismatch | dependency_generation_mismatch |
  search_projection_owner_missing | search_projection_outside_membership |
  search_projection_identity_mismatch | dense_projection_owner_missing |
  dense_projection_partial | dense_projection_identity_mismatch |
  dense_projection_outside_membership | projection_generation_corrupt |
  projection_member_corrupt | mutation_receipt_corrupt |
  mutation_readiness_unavailable | mutation_readiness_corrupt

DataPlaneIntegrityFindingV1 {
  schema_version: 1,
  code: DataPlaneIntegrityFindingCodeV1,
  severity: DataPlaneIntegritySeverityV1,
  artifact_revision_ids: [string],
  dependency_id: string?,
  projection_generation_id: string?,
  operation_id: string?,
  write_cursor: u64?,
}

DataPlaneIntegrityBoundaryV1 {
  schema_version: 1,
  effective_at_epoch_s: i64,
  observed_write_boundary: u64,
  dependency_generation: u64,
  projection_generation_id: string,
}

DataPlaneIntegrityCheckCountV1 {
  schema_version: 1,
  check: DataPlaneIntegrityCheckV1,
  checked_count: u32,
  finding_count: u32,
}

DataPlaneIntegrityResultV1 {
  schema_version: 1,
  read_boundary: DataPlaneIntegrityBoundaryV1,
  check_counts: [DataPlaneIntegrityCheckCountV1],
  checked_count: u32,
  findings: [DataPlaneIntegrityFindingV1],
  complete: true,
}
```

`checks` must be nonempty and duplicate-free. Callers may provide any order,
but execution and `check_counts` always use the closed order shown in the enum.
`max_work_units` is required and `1..=10_000`; `max_findings` is required and
`1..=100`. The CLI default is `max_work_units=10_000` and
`max_findings=100`. These hard ceilings cannot be raised.

One work unit is one candidate authority object fully classified by a check:

- `dependency_chain`: one `_fathomdb_source_dependencies` row;
- `active_searchable_orphans`: one synchronous FTS/attribute owner candidate
  or one dense owner tuple keyed by `(artifact_class, write_cursor, kind)`;
- `projection_generation`: one current generation authority row plus one
  physical member classified through the shared Slice 40 classifier; and
- `mutation_readiness`: one non-erased receipt pending-cursor pair.

Each check uses stable primary-key order and an indexed remaining-cap-plus-one
query. The aggregate remaining cap is passed into each later check; there is no
independent per-check allowance that can exceed the aggregate. A candidate
increments its check and aggregate `checked_count` exactly once after its
fixed-width joined state has been loaded, whether clean or a finding. The same
candidate may legitimately be classified by two requested checks and then
costs one unit in each, because the checks make different assertions.

The operation opens one SQLite deferred reader transaction, resolves exactly
one effective instant, captures the boundary, and executes all selected checks
before commit. It neither creates a frozen job nor observes drift between
pages. A work cap-plus-one candidate, a finding cap-plus-one result, count
overflow, or inability to classify the entire requested set returns
`integrity_bound_exceeded` and no `DataPlaneIntegrityResultV1`. Success is
always `complete=true`; `checked_count` equals the checked-count sum and
`findings.len()` equals the finding-count sum.

### Dependency-chain finding matrix

The dependency check validates this actual normalized chain:

```text
_fathomdb_source_dependencies
  -> derived _fathomdb_artifact_revisions
  -> derived _fathomdb_source_links
  -> canonical _fathomdb_artifact_revisions + canonical_nodes
  -> _fathomdb_source_versions
  -> canonical self _fathomdb_source_links
  -> _fathomdb_open_state dependency generation
```

| Condition in the one snapshot | Finding code | Severity | IDs returned |
|---|---|---|---|
| Dependency schema version, ID grammar, or stored generation is invalid. | `dependency_row_invalid` | error | Dependency ID only when grammar is valid. |
| Derived owner is missing or does not resolve its exact canonical node/edge row. | `dependency_derived_owner_missing` | critical | Dependency and derived revision. |
| Derived owner is not complete, is not `derived_semantic`, or self-references. | `dependency_derived_role_invalid` | error | Dependency and derived revision. |
| Derived source link is missing. | `dependency_source_link_missing` | critical | Dependency and derived revision. |
| Link artifact/source/version fields disagree with the dependency or owners. | `dependency_source_link_mismatch` | critical | Dependency and both valid revision IDs. |
| Canonical owner/node is missing. | `dependency_source_owner_missing` | critical | Dependency and source revision. |
| Canonical owner is not complete `canonical_source` node. | `dependency_source_role_invalid` | error | Dependency and source revision. |
| Source-version row is missing or disagrees with canonical owner/self-link. | `dependency_source_version_mismatch` | critical | Dependency and source revision. |
| Canonical self-link is missing or is not whole-body/self/hash coherent. | `dependency_source_self_link_mismatch` | critical | Dependency and source revision. |
| Registered generation is zero/ahead of the singleton, or singleton is malformed/regressed. | `dependency_generation_mismatch` | critical | Dependency when attributable; otherwise none. |

There is no `dependency_reciprocity` reverse-row finding and no injected
missing-reverse-row fixture. Forward and reverse lookup are two indexes/joins
over the same row and source link. Test hooks corrupt the real rows/fields in
the matrix, one fault per disposable database, and prove both lookup directions
fail closed where applicable.

### Search/projection/orphan finding matrix

The effective instant is the boundary's `effective_at_epoch_s`. Physical
membership and dense completion are the exact shared Slice 40 predicates; the
integrity implementation must call those helpers rather than reproduce their
SQL. Strict-current source eligibility, Slice 30 physical closure fences,
node lifecycle/currentness, edge currentness/validity, registered-source
validity, and generation membership are evaluated at that instant.

| Authority/candidate | Legitimate exclusion | Finding code | Severity / IDs |
|---|---|---|---|
| Node/edge body FTS row or property-FTS/attribute row | Historical node rows retained for an accepted relaxed read; expired edge residue explicitly classified dormant by Slice 40. | `search_projection_owner_missing` when no canonical owner; `search_projection_outside_membership` when residue survives erasure, supersession, or completed closure that promised pruning; `search_projection_identity_mismatch` for wrong kind/row identity. | critical for missing/outside; error for mismatch; artifact revision when resolvable plus cursor. |
| `_fathomdb_vector_rows`, `vector_default`, and terminal tuple | A complete node tuple may remain after enrolment changes; node `legitimate-stranded`, scheduler-pending all-missing state, clean failed all-missing state, and Slice 40 dormant expired-edge state are accepted exactly as classified there. | `dense_projection_owner_missing`, `dense_projection_partial`, `dense_projection_identity_mismatch`, or `dense_projection_outside_membership`. | critical for missing/outside; error for partial/mismatch; artifact revision when resolvable plus cursor. |
| Current generation row/singleton/declaration | No legacy exception beyond Slice 40's valid `legacy_unverified` degraded generation. | `projection_generation_corrupt`. | critical; generation ID only when valid. |
| Current-generation physical member | Slice 40 `complete`, `scheduler-pending`, `legitimate-stranded`, and clean `failed` states. | `projection_member_corrupt`. | error; generation ID, artifact revision when resolvable, and cursor. |
| Non-erased actuation receipt pending cursor | Redacted/erased receipts and pre-step-32 receipts with null generation are excluded; a valid pending cursor may now be ready, processing, blocked, deferred, or degraded. | `mutation_receipt_corrupt`, `mutation_readiness_unavailable`, or `mutation_readiness_corrupt`. | error; operation ID only for operator output, generation ID when valid, and cursor. |

`mutation_readiness` scans the persisted bounded pending-cursor arrays of
non-erased receipts; it does not scan all canonical cursors or infer pending
work. It invokes the Slice 40 point classifier with the receipt's exact
operation ID, cursor, and expected generation. `ProjectionGenerationError`
maps as follows inside integrity only:

- `projection_generation_corrupt` becomes `projection_generation_corrupt` for
  generation-wide classification or `mutation_readiness_corrupt` for a receipt
  point;
- `projection_generation_unavailable` becomes
  `mutation_readiness_unavailable` for a valid retained receipt;
- `mutation_not_tracked`, `wrong_projection_generation`, invalid persisted
  IDs/cursors, or a receipt/pending-list disagreement becomes
  `mutation_receipt_corrupt`; and
- request-construction errors are implementation defects and fail the whole
  operation as `integrity_corrupt`; they are never silently skipped.

Normal public Slice 40 methods retain their original typed
`ProjectionGenerationError`; only this operator report translates a fully
classified fault into a finding.

### Finding order and privacy

Findings order by check enum, candidate stable key, then finding-code spelling.
Every optional ID is omitted when malformed or not authorized by the operator
candidate. Details, SQL, bodies, hashes, locators, source IDs/versions, query
text, predicates, attribute values, and natural keys never appear. Findings
contain only the closed code and the minimum typed identities needed for local
operator repair outside this slice. The method itself is read-only and offers
no repair hint or executable action.

## Additive explanation successor

The accepted carrier remains:

```text
SearchResult { ..., explanation: Explanation? }
Explanation { trace: QueryTrace, per_hit: [PerHitExplain], ... }
```

Slice 55 amends the two non-exhaustive structs additively:

```text
Explanation {
  trace: QueryTrace,
  per_hit: [PerHitExplain],
  correlation_id: string,
}

PerHitExplain {
  id, arm, vector_rank?, text_rank?, graph_rank?,
  fused_score, ce_score?, blended, importance?, confidence?,
  structural: StructuralInclusionV1,
}

StructuralInclusionStateV1 = included | degraded
StructuralProjectionOriginV1 =
  synchronous_body_fts | current_dense_generation | graph_traversal
StructuralDependencyStateV1 =
  not_applicable | not_registered | registered
StructuralLifecycleStateV1 =
  node_pending | node_active | node_deleted | edge_valid
StructuralDegradationCodeV1 =
  soft_fallback_text | soft_fallback_text_edge |
  projection_legacy_unverified | projection_blocked |
  projection_deferred | graph_bound_reached

StructuralInclusionV1 {
  schema_version: 1,
  inclusion_state: StructuralInclusionStateV1,
  projection_origin: StructuralProjectionOriginV1,
  dependency_state: StructuralDependencyStateV1,
  lifecycle_state: StructuralLifecycleStateV1,
  degradation_codes: [StructuralDegradationCodeV1],
}
```

`per_hit[i]` still associates only by array position with `results[i]`.
`PerHitExplain.id` remains the existing internal positional write cursor and is
not reinterpreted as `SearchHit.id` or artifact identity. Structural data
describes only a returned, already-visible hit. It never names a rejected
candidate, and it carries no revision, source, dependency, operation, receipt,
projection-generation, or caller identity.

`included` means no closed degradation code applies. `degraded` means at least
one applies, and `degradation_codes` is nonempty, unique, and in enum order.
The projection origin is the representative winning origin, not a semantic
causal claim. `dependency_state` is `registered` only when the returned
artifact's valid complete Slice 20 row exists in the same search snapshot;
absence is `not_registered`, not corruption. Canonical sources and legacy
incomplete artifacts use `not_applicable`.

Ordinary `search_explained` provides this content-free structural object from
the search snapshot but never upgrades authorization: it includes no evidence
identity or source fields. `search_with_evidence(include_explanation=true)`
uses the same structural object and correlation identity while its separately
authenticated Slice 50 evidence entries remain the only path to exact source
identity/bytes. The two surfaces do not copy evidence-only fields into
`Explanation`.

One Engine-minted content-free `correlation_id` identifies the search. It uses
the existing canonical telemetry `query_id` grammar and one Engine-owned
open-nonce/monotonic-sequence allocator. When local opt-in telemetry is on, the
same value is written as `TelemetryEvent.query_id`; no `operation_id` or second
join identity exists. When telemetry is off, explanation still returns the
correlation ID but writes no event and opens no sink. When explanation is off,
the default path allocates no structural objects or explanation-only ID; the
existing telemetry-on path may mint its normal query ID.

Telemetry may contain only its accepted schema plus this safe structural
subset if a later additive telemetry field is implemented here:
`correlation_id`, query length, result positional IDs, winning arm, rank
positions, finite scores, inclusion state, projection-origin enum,
dependency-state enum, lifecycle-state enum, and degradation codes. It must not
contain raw artifact/source/source-version/dependency/operation/generation IDs,
locators, hashes, bodies, query text, predicates, attribute values, owner/scope,
or other caller natural keys. Slice 55 does not require adding the structural
subset to telemetry; reuse of the identity is required.

## A25-05 wire, codec, and error contract

### Exact naming and scalar encoding

Rust fields are snake case. Python request mappings and attributes are snake
case. TypeScript and canonical JSON are camel case. Enum/discriminant values
are the lower-snake-case spellings listed above. Optional JSON members are
present as `null` in responses; forbidden union members must be absent.

Every new object shown in this design carries `schema_version/schemaVersion =
1`. Existing `Explanation`, `QueryTrace`, and `PerHitExplain` are not silently
made closed or given a retroactive schema field; only the nested
`StructuralInclusionV1` is a new versioned object.

New Rust `u64` fields use `u64`. New Python, TypeScript, and JSON `u64` fields
use canonical unsigned decimal strings, including zero. Signs, leading zeroes,
whitespace, decimal points, exponent notation, booleans, numeric substitutes,
and values above `u64::MAX` reject. `u32` uses JSON numbers and is checked to
its range. `i64` effective instants use signed JSON numbers and are checked to
the exact range. Existing `PerHitExplain.id` remains its accepted dynamic SDK
integer representation. Every existing/new score is a finite JSON number;
NaN and infinity are corruption, never serialized.

Canonical response field order is the declaration order in each pseudo-schema
above. For the additive structs, `correlationId` follows `perHit`, and
`structural` follows `confidence`. Canonical fixtures pin exact bytes for
Rust serde helpers, PyO3/Python, N-API/TypeScript, and CLI JSON.

Requests are closed: reject unsupported schema first; select the
lexicographically smallest unknown canonical camel-case field next; then check
required/type/semantic fields in declaration order. Multiple unknown fields
select before RFC 6901 escaping. Responses ignore additive unknown object
fields but reject unsupported/newer schema versions, unknown enums, incoherent
unions, malformed integers, nonfinite scores, and count/array disagreement at
the exact nested path. Unknown required variants never degrade to a default.

### Trace errors and precedence

```text
DependencyTraceErrorReasonV1 =
  unsupported_schema_version | unknown_field |
  trace_root_invalid | trace_direction_invalid |
  trace_limit_invalid | trace_unavailable |
  trace_root_role_invalid | trace_bound_exceeded | trace_corrupt

DependencyTraceErrorV1 { schema_version: 1, reason, field_path }
```

`EngineError::DependencyTrace` maps to Python `DependencyTraceError` and
TypeScript `DependencyTraceError`, both with stable code
`FDB_DEPENDENCY_TRACE`. Python exposes `.reason`/`.field_path`; TypeScript
exposes `reason`/`fieldPath`. Validation order is schema, unknown field,
`rootRevisionId`, `direction`, `context`, `maxRelations`, then
`maxWorkUnits`. Invalid paths are exactly `/schemaVersion`, the escaped unknown
top-level field, `/rootRevisionId`, `/direction`, `/context`,
`/maxRelations`, and `/maxWorkUnits`.

Frozen-context shape/authentication failures retain `FrozenReadError` and its
exact nested paths. After context authentication, root absence/invisibility
collapses to `trace_unavailable` at `/rootRevisionId`; only a visible root may
produce role, bound, or corruption detail. Trace errors include no submitted or
stored ID value.

### Integrity errors and CLI mapping

```text
DataPlaneIntegrityErrorReasonV1 =
  unsupported_schema_version | unknown_field |
  checks_empty | duplicate_check | integrity_check_invalid |
  integrity_limit_invalid | integrity_bound_exceeded | integrity_corrupt

DataPlaneIntegrityErrorV1 { schema_version: 1, reason, field_path }
```

`EngineError::DataPlaneIntegrity` is operator-only. The CLI emits the same
lower-snake reason and camel-case RFC 6901 `fieldPath` in its version-1 JSON
error envelope. A successful clean report exits 0; a successful report with at
least one finding exits 65 (`DOCTOR_FOUND_ISSUES`); every request, bound, or
integrity error exits 70 (`UNRECOVERABLE`), except the existing open-time lock
mapping to 71. The new CLI success envelope is
`{schemaVersion:"fathomdb.doctor.data-plane-integrity.v1",status,report}` with
`status=clean|findings`; its error envelope uses the same schema version plus
`status=error`, `verb=data-plane-integrity`, the stable engine code, reason,
and `fieldPath`.
Validation order is schema, unknown field, `checks` container, check values in
array order, duplicate detection at the later duplicate index,
`maxWorkUnits`, then `maxFindings`. Exact paths include `/checks`,
`/checks/<index>`, `/maxWorkUnits`, and `/maxFindings`.

Integrity request/refusal messages contain no database identity, SQL, table
contents, caller IDs, or candidate counts. A cap failure does not report which
check crossed the cap or how many more rows exist. Successful operator findings
use only the authoritative matrix's minimum IDs.

## Implementation shape and non-mutation

Trace and integrity live in separate modules with shared private helpers for
loading one validated normalized dependency chain and invoking Slice 40's
membership/completion classifier. No public raw-SQL seam is introduced. Test
hooks may execute narrowly allowlisted fault mutations only under
`test-hooks`; production code has no corruption injector.

Both operations use read-only deferred transactions. Tests compare database,
WAL, and SHM bytes before/after each operation after checkpoints appropriate to
the existing WAL contract. Trace/integrity never register dependencies, change
lifecycle, enqueue projection work, advance generation/write cursors, create a
receipt/job, rebuild an index, or repair a fault.

The ordinary search path retains its current branch and response behavior.
Only an explain-enabled search allocates the new correlation/structural data.
Result order, scores, `SearchHit`, SQL query plans, and database bytes are
identical with explanation off.

## Forward allocation

- 0.8.28 owns expanded exclusion/not-selected explanation and rich trace/path
  continuation if a concrete consumer proves the one-page contract inadequate.
- 0.8.33 owns frozen integrity jobs and any governed repair planning.
- Repair, reverse-index generation, semantic validation, and unbounded browse
  remain outside Slice 55.

## Readiness rule

Design v6/FIX-1 resolves the nine Cycle 1 findings by proposal. It remains
`DRAFT_FIX_1_REVIEW_REQUIRED` until an independent reviewer verifies the exact
surface, bound, matrix, privacy, wire, and test contracts and records PASS. A
P1 or P2 finding blocks READY and implementation.
