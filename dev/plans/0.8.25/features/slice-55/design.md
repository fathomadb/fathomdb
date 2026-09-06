---
title: 0.8.25 Slice 55 — basic tracing, explanation, and integrity design
status: READY
design_version: 8
review_fix: 3
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
are complete. Independent design review Cycle 4 passed at design v8/FIX-3
with no unresolved P1/P2 finding. This design is READY for TDD implementation.

## Requirements and acceptance

- **S55-R1 — non-colliding surfaces.** Preserve the existing operator-only
  `trace_source_ref` and `check_integrity` contracts while adding exact,
  separately named application trace and operator data-plane integrity APIs.
- **S55-R2 — reciprocal trace.** For the same frozen context, source-to-derived
  and derived-to-source reads return the same registered Slice 20 relation and
  endpoints in opposite traversal directions.
- **S55-R3 — bounded snapshot.** Trace and integrity each linearize in one
  SQLite reader transaction and use deterministic order. Trace caps eligible
  output classification; integrity caps all counted candidate classification.
  Both use cap-plus-one detection for their declared units.
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
| Telemetry correlation | Opt-in local telemetry mints `q0-<seq>` under the sink lock; every enable/re-enable resets `nonce=0`, `seq=0`, and `last_query_id=None`. There is no public disable operation. | An explained search uses the telemetry ID only when the sink is enabled at finalization; otherwise it uses a separate explanation-only ID allocator. Non-explain telemetry bytes and re-enable reset remain unchanged. |
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
ceilings are not configurable. A trace work unit bounds authorized output
classification only: one eligible root or one registered relation whose two
endpoints have independently passed the authenticated eligibility selection.
Each classified relation may load only the dependency row, derived owner,
derived source link, canonical owner/node, source-version row, canonical
self-link, and singleton generation. Internal SQLite virtual-machine steps and
physical rows examined while proving eligibility are deliberately not trace
work units. The Engine detects only eligible-output overflow with
`LIMIT remaining+1`; it makes no strict physical rows-visited bound for trace.

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
physical closure fences.

The indexed candidate statement performs endpoint authorization in its `JOIN`
and `WHERE` clauses before eligible-output `LIMIT`, work accounting, bound
detection, or corruption classification. It first proves each endpoint from
independently valid owner/lifecycle rows; it does not trust the relation row to
establish an endpoint's visibility. Only then may it validate the registered
relation's schema, dependency identity/generation, and cross-row agreement. An
ineligible endpoint or a malformed chain that cannot independently prove both
endpoints eligible is indistinguishable from no relation: it contributes no
edge, node, work unit, count, bound error, or `trace_corrupt` result. The
operator-only integrity check remains the path that diagnoses such hidden
state. Hidden physical candidates may increase internal query work, but never
change response bytes, errors, counts, or bound outcomes.

The lookup must use existing schema indexes and adds no migration. `to_source`
starts with `_fathomdb_source_dependencies`' unique
`derived_revision_id` index. `to_dependents` walks
`_fathomdb_source_links_source_derived_idx(source_revision_id,
artifact_revision_id)` and joins the dependency row by its unique derived key.
Owner/revision and closure lookups use their existing primary/lookup indexes.
After-key query pages follow index order; the at-most-100 eligible relations
are canonically sorted in bounded Rust memory. `EXPLAIN QUERY PLAN` must show
the named source/derived indexes and must contain neither `USE TEMP B-TREE` nor
`MATERIALIZE`. Trace must not build a full candidate vector or SQL temporary
sort, even though its public work counter does not bound index entries visited.

`trace_corrupt` is permitted only after both independently resolved endpoints
passed the authenticated eligibility envelope and the remaining relation
metadata is contradictory. It carries no endpoint IDs. Thus, under the same
context, any returned edge is reciprocal: tracing its source `to_dependents`
and derived endpoint `to_source` returns the same `SourceDependencyV1`
identity.

An eligible derived root without an eligible, independently verifiable
registered relation returns only the root. An eligible source with no eligible,
independently verifiable registered dependents does the same. Missing, erased,
superseded, closure-fenced, or ineligible roots all return `trace_unavailable`
at `/rootRevisionId`. That nondisclosing outcome precedes role and corruption
detail. Trace returns no source body, payload, query, predicate value,
`source_id`, source version, locator, or hash; Slice 50 remains the authorized
exact-source path.

If either post-eligibility cap-plus-one probe observes excess work/relations,
the call returns `trace_bound_exceeded`, no result, and no partial IDs.
`checked_work_units` therefore appears only on complete success and equals one
plus the number of eligible registered relations fully classified. Hidden,
ineligible, or unverifiable candidates never influence it.

The nondisclosure fixture compares canonical response/error bytes and work
counts across databases containing: no relation; an ineligible counterpart;
an erased/superseded/closure-fenced counterpart; a malformed dependency row;
a missing derived owner; a source-link mismatch that prevents independent
source authorization; and more than either caller cap of hidden relations. It
also races eligibility loss before the reader transaction. Every case is
observationally identical to no relation. Separate fixtures prove
`trace_corrupt` only when both endpoints remain eligible and an independently
non-authorizing relation field is corrupt.

A preregistered performance fixture gives one visible source 50,000 registered
dependents, all hidden by the authenticated context, and calls the one-page API
with `maxRelations=1` and `maxWorkUnits=2`. The response must equal the
no-relation fixture and must not return a bound error. On the release-mode
`ubuntu-latest` x86-64 reference job, the candidate statement is limited to
10,000,000 SQLite VM steps, 5.0 seconds elapsed, and 64 MiB peak-RSS delta.
Fixture construction is excluded; measurement brackets only the trace call.
The VM-step ceiling is the deterministic regression gate; elapsed/RSS are
recorded platform ceilings. These numbers are fixed before implementation and
may not be relaxed after observation. This is a measured performance guard,
not a claim that `maxWorkUnits` bounds hidden physical rows. Trace remains one
page, has no continuation token, and never traverses beyond depth 1.

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
  node_body_fts_missing | node_body_fts_v2_missing |
  edge_body_fts_missing | canonical_attribute_missing |
  property_fts_missing |
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

One work unit is one authority row or member candidate loaded and fully
classified by a check:

- `dependency_chain`: one dependency-generation singleton row, then one
  `_fathomdb_source_dependencies` row;
- `active_searchable_orphans`: one projection-registry authority row, one
  expected synchronous member, one scanned physical member, or one dense
  owner tuple keyed by `(artifact_class, write_cursor, kind)`;
- `projection_generation`: one current-generation singleton row, one current
  generation record, then one physical member classified through the shared
  Slice 40 classifier; and
- `mutation_readiness`: one `_fathomdb_actuation_receipts` row, then one unit
  for each pending-cursor element in that row. An empty pending array costs the
  receipt-row unit and zero pair units.

Each check uses the index-supported stable order specified below and an indexed
remaining-cap-plus-one query. The aggregate remaining cap is passed into each
later check; there is no independent per-check allowance that can exceed the
aggregate. A candidate increments its check and aggregate `checked_count`
exactly once after its bounded joined state has been loaded, whether clean or a
finding. The same candidate may legitimately be classified by two requested
checks and then costs one unit in each, because the checks make different
assertions.

Singletons are real units even when the corresponding collection is empty.
`mutation_readiness` does not call `load_receipt` and is not a whole-receipt
validator. It selects exactly these columns:
`operation_id`, `schema_version`, `operations_count`, `outcome`,
`resulting_write_boundary`, `pending_projection_write_cursors_json`, and
`projection_generation_id`. It does not select or validate `request_sha256`,
refusal fields, `reason_codes_json`, `affected_revision_ids_json`,
`resulting_dependency_generation`, `closure_operation_ids_json`, or receipt
source-reference rows.

The first bounded SQL pass returns only rowid plus type/byte-length/JSON
metadata, ordered through the receipt primary-key index; it does not return a
variable field to Rust. Every selected variable field has an existing-grammar
guard: `operation_id` is ASCII caller identity of 1..=128 bytes; `outcome` is
one of the four schema values and at most 25 bytes;
`projection_generation_id` is null or exactly the 38-byte
`pgen1:<32-lower-hex>` form; and pending JSON is an array of at most 128 values
and at most 2,945 bytes (128 quoted 20-digit `u64` strings, 127 commas, and two
brackets). Fixed columns must have SQLite integer/null types and their existing
schema ranges. A failed guard produces `mutation_receipt_corrupt` without
fetching, allocating, or deserializing the variable value.

If pending array length would exceed aggregate remaining work, cap-plus-one
fails before the second rowid lookup fetches the guarded minimal columns. Only
a valid, bounded array is decoded. Its strings must be canonical nonzero
decimal `u64`, strictly ascending and unique; its length must not exceed
`operations_count`; committed outcomes require a nonnegative write boundary
not below any pending cursor; refused/erased rows require an empty pending
array and null boundary/generation; and an empty pending array requires a null
generation. For nonempty pending arrays, a valid generation is required except
for a pre-step-32 receipt covered by a valid `legacy_unverified` generation
through its maximum pending cursor; that legacy pair classifies as unavailable,
not corrupt. Empty arrays cost the receipt-row unit and zero pair units.

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

The synchronous check has two bounded directions. The expected-owner direction
derives every required member from canonical authority and reports a missing
row. The physical direction scans every stored member and reports an absent,
ineligible, duplicate, or mismatched owner. A check that only scanned physical
rows could never detect false-negative retrieval caused by deletion, so both
directions are mandatory.

The exact synchronous authorities and member classes, in execution order, are:

1. each `_fathomdb_projection_registry` row in its binary `name` primary-key
   order;
2. `node_body_fts_v1`: every canonical node for which the shared synchronous
   projector-retention predicate requires one `search_index` row;
3. `node_body_fts_v2`: the same owner set requiring one `search_index_v2` row;
4. `edge_body_fts`: every body-bearing canonical edge for which the shared
   edge projector-retention predicate requires one `search_index_edges` row;
5. `canonical_attribute`: each active, non-superseded node and registry
   declaration whose configured source path resolves to a supported scalar and
   whose roles require EAV, requiring one exact `(write_cursor, attr_name,
   attr_value)` row in `canonical_attributes`;
6. `property_fts`: each such expected EAV member whose declaration requires
   property FTS, requiring one identical tuple in `property_search_index`; and
7. `dense_owner_tuple`: the Slice 40 physical-member tuple over
   `_fathomdb_vector_rows`, `vector_default`, and
   `_fathomdb_projection_terminal`.

The fixed member-class ordinal is the first ordering component. Physical scans
of `search_index`, `search_index_v2`, `search_index_edges`, and
`property_search_index` use each FTS5 table's indexed implicit `rowid` as the
only within-class key. The ordinary `canonical_attributes` physical scan also
uses its indexed rowid. Duplicate rows are therefore counted and never
collapsed. Expected body-owner scans use
`canonical_nodes_write_cursor_idx` or `canonical_edges_write_cursor_idx`.
Expected attribute/property scans loop declarations in the registry's binary
`name` primary-key order and walk canonical owners through
`canonical_nodes_write_cursor_idx`; their stable key is
`(member_class_ordinal, declaration_name, write_cursor)`. They never issue
`ORDER BY write_cursor, attr_name` or any other order unsupported by an
existing index.

Each registry row, expected member, scanned physical row, and dense tuple costs
one work unit. Per-class after-key statements receive aggregate remaining cap
plus one before constructing a body/attribute value. Matched physical rows
still count, making actual scan cost explicit. Only after both bounded
directions complete does Rust reconcile their at-most-10,000-member keyed sets;
it never performs a per-owner lookup on an FTS `write_cursor`/`attr_name`
column declared `UNINDEXED`. Exact `EXPLAIN QUERY PLAN` RED tests require the
named canonical indexes or FTS rowid lookup and reject `USE TEMP B-TREE`,
`MATERIALIZE`, and an unbounded full-result materialization. No index or schema
migration is permitted.

| Authority/candidate | Legitimate exclusion | Finding code | Severity / IDs |
|---|---|---|---|
| Required `search_index` member | A canonical owner outside the shared synchronous projector-retention predicate. Historical node rows retained for relaxed reads remain expected, not excluded. | `node_body_fts_missing` | critical; artifact revision and cursor. |
| Required `search_index_v2` member | Same predicate as `search_index`. | `node_body_fts_v2_missing` | critical; artifact revision and cursor. |
| Required `search_index_edges` member | Body-less edge; or an edge excluded by the shared governed synchronous-pruning predicate. Slice 40 dormant expired-edge retention remains expected when its physical row is retained. | `edge_body_fts_missing` | critical; artifact revision and cursor. |
| Required `canonical_attributes` scalar member | Missing/null/object/array source-path terminal, non-EAV declaration, pending/deleted/superseded node, erased owner, or completed closure whose policy removes the member. | `canonical_attribute_missing` | critical; artifact revision and cursor; never attribute name/value. |
| Required `property_search_index` member | Every EAV exclusion above plus a declaration without property FTS. | `property_fts_missing` | critical; artifact revision and cursor; never attribute name/value. |
| Scanned node/edge body FTS or attribute/property row | Historical node and dormant expired-edge members accepted by the shared retention predicate. | `search_projection_owner_missing` for no canonical owner; `search_projection_outside_membership` for residue after erasure/supersession/completed closure promised pruning; `search_projection_identity_mismatch` for wrong body/kind/status/name/value, cursor, or duplicate cardinality. | critical for missing/outside; error for mismatch; artifact revision when resolvable plus cursor. |
| `_fathomdb_vector_rows`, `vector_default`, and terminal tuple | A complete node tuple may remain after enrolment changes; node `legitimate-stranded`, scheduler-pending all-missing state, clean failed all-missing state, and Slice 40 dormant expired-edge state are accepted exactly as classified there. | `dense_projection_owner_missing`, `dense_projection_partial`, `dense_projection_identity_mismatch`, or `dense_projection_outside_membership`. | critical for missing/outside; error for partial/mismatch; artifact revision when resolvable plus cursor. |
| Current generation row/singleton/declaration | No legacy exception beyond Slice 40's valid `legacy_unverified` degraded generation. | `projection_generation_corrupt`. | critical; generation ID only when valid. |
| Current-generation physical member | Slice 40 `complete`, `scheduler-pending`, `legitimate-stranded`, and clean `failed` states. | `projection_member_corrupt`. | error; generation ID, artifact revision when resolvable, and cursor. |
| Actuation receipt row and each pending-cursor pair | Refused/erased receipts are valid rows that cost one row unit and have zero pairs. A pre-step-32 null-generation pending pair covered by `legacy_unverified` is valid but unavailable. Other valid pairs may be ready, processing, blocked, deferred, or degraded. | `mutation_receipt_corrupt`, `mutation_readiness_unavailable`, or `mutation_readiness_corrupt`. | error; operation ID only for operator output, generation ID when valid, and cursor. |

Deletion RED fixtures remove exactly one expected `search_index`,
`search_index_v2`, `search_index_edges`, `canonical_attributes`, or
`property_search_index` row after creating it through a public write. Each
fixture proves the corresponding missing-row code, stable key/order, work
count, no false orphan for the explicitly legitimate exclusions, and no
automatic repair.

`mutation_readiness` scans only the minimal guarded readiness subset above; it
does not scan all canonical cursors, infer pending work, call whole-receipt
loading, or diagnose unrelated receipt JSON. For each valid pending pair it
invokes the private shared Slice 40 physical point classifier in the same
transaction with the receipt's operation ID, cursor, and expected generation;
it does not call the public method that reloads the full receipt.
`ProjectionGenerationError` maps as follows inside integrity only:

- `projection_generation_corrupt` becomes `projection_generation_corrupt` for
  generation-wide classification or `mutation_readiness_corrupt` for a receipt
  point;
- `projection_generation_unavailable` becomes
  `mutation_readiness_unavailable` for a valid retained receipt;
- invalid selected readiness fields, a selected-field disagreement, a pending
  cursor absent from canonical revision authority, or wrong stored projection
  generation becomes `mutation_receipt_corrupt`; and
- request-construction errors are implementation defects and fail the whole
  operation as `integrity_corrupt`; they are never silently skipped.

Normal public Slice 40 methods retain their original typed
`ProjectionGenerationError`; only this operator report translates a fully
classified readiness fault into a finding. `mutation_receipt_corrupt` means
only that the selected readiness subset is malformed or incoherent. Corruption
confined to reason, affected-revision, closure, refusal, digest, dependency, or
source-reference fields is outside this check and must not produce that code.

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

One Engine-minted content-free `correlation_id` identifies an explained search,
but Slice 55 does not silently move the accepted telemetry counter into a
shared allocator. Finalization chooses exactly one exclusive source while
holding the existing telemetry mutex:

1. If an enabled sink is present at that lock linearization point, the sink is
   the sole source. It mints the exact existing `q{nonce}-{seq}` value, assigns
   it to `Explanation.correlation_id`, writes that same value as
   `TelemetryEvent.query_id`, increments the sink sequence once, and sets
   `last_query_id` exactly as before.
2. If no sink is present, a separate explanation-only atomic sequence plus the
   Engine's content-free open nonce mints
   `x<32-lower-hex-open-nonce>-<canonical-u64-seq>`. It does not read or advance
   telemetry nonce, sequence, timestamp base, or `last_query_id`, and writes no
   event.

The result arrives from the reader with structural data but no correlation;
one post-reader `finalize_search_observability(&mut SearchResult, query)`
performs the selection before the result can escape. This avoids predicting
telemetry state in the reader transaction and guarantees exactly one ID.
Telemetry-enabled non-explain searches continue through the existing sink-only
capture path byte-for-byte. Explain-disabled, telemetry-off searches never
touch the explanation allocator or allocate structural objects.

Current code has no public disable operation. The accepted transitions are
initial disabled state and `enable_telemetry`, including idempotent re-enable.
Every enable/re-enable still replaces the sink with `nonce=0`, `seq=0`, a new
time base, and `last_query_id=None`; its next telemetry event remains `q0-0`
with the existing canonical JSON bytes/order. Slice 55 adds no disable API. If
enable races search finalization, the telemetry mutex orders them: enable
before the finalization lock selects `q0-0` and writes one event; enable after
it selects the `x...` explanation ID and leaves the newly reset sink untouched.
Concurrent explained/non-explained searches serialize only telemetry ID/event
assignment under the existing sink lock; each telemetry event gets the next
unique unchanged `q0-N`, while telemetry-off explained searches use the
independent atomic sequence. No search can consume both sequences or emit two
events.

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

### Response presence versus construction compatibility

Response-wire presence and user construction are separate evolution rules.
Every response produced by a new Slice 55 Engine/native binding contains a
nonempty valid `Explanation.correlation_id` and a populated
`PerHitExplain.structural` for every explained hit. Canonical v1 response
fixtures therefore require both members; omission from a new native response
is a binding-contract failure.

Python must remain source compatible with existing user/test construction and
dataclass field-order rules. `Explanation` appends
`correlation_id: str = ""` after the existing defaulted `per_hit`; empty is the
legacy/local-construction sentinel and is never emitted by a new Engine.
`PerHitExplain` appends
`structural: StructuralInclusionV1 | None = None` after the existing defaulted
`importance` and `confidence`. The wrapper always maps a native object with an
absent member to those safe defaults via checked `getattr`; there is no runtime
capability discriminator and absence is never interpreted as an
advertised-version violation. Direct candidate-native conformance tests,
rather than wrapper inference, require and validate populated values on new
Engine responses. No required field follows a defaulted field.

TypeScript keeps existing object literals source compatible by declaring
`Explanation.correlationId?: string` and
`PerHitExplain.structural?: StructuralInclusionV1`. The mapper includes each
property when present and valid and otherwise omits it for any native object;
it does not consult or invent a capability/version discriminator, an empty
correlation ID, or a default structural classification. Direct candidate
native conformance fixtures require presence on every new Engine response.
New Engine method return documentation narrows the runtime guarantee to
present/nonempty even though the user-constructible interface remains
optional.

Rust's `#[non_exhaustive]` response structs remain externally
non-constructible by field literal; in-crate constructors and matches are
updated. PyO3 and N-API add tail fields only and retain all existing names and
method ABIs. Tests compile unchanged pre-Slice-55 Rust readers, instantiate the
old Python/TypeScript object literals, decode simulated older native objects,
and assert new Engine responses populate/validate both additions. The required
successor ADR records this additive construction/wire split and supersedes only
the relevant 0.8.8 field-set evolution paragraph; it does not rewrite the
ratified carrier or history.

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

Independent Cycle 4 review passed design v8/FIX-3 at
`4a5ec9b4dd7854986ab19f5f5d9f1510fc737148`, verified C3-55-01 through
C3-55-04 resolved, and found no unresolved P1/P2 issue. The design is `READY`.
The nonblocking P3 review note requires implementation verification evidence
to name the exact SQLite VM-step and RSS measurement mechanism used for the
preregistered performance ceiling; it does not alter the approved contract.
