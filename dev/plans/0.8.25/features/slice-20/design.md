---
title: 0.8.25 Slice 20 — core dependency registration design
status: READY_REVIEW_PASS_CYCLE_5
design_version: 7
review_fix: 4
depends_on: 15
architecture: dev/design/fathomdb-data-plane-architecture-v2.md
---

# Slice 20 design

## Authority and boundary

This slice implements S20-R1 through S20-R5, the retained core of R25/AC25-20,
N25-01, Memex need 4, and A25-05. It exposes one provenance-safe dependency
relation from an immutable canonical source revision to an immutable
caller-derived artifact revision. FathomDB validates and indexes the relation;
it does not infer it, choose whether it should exist, or assign semantic truth.

Slice 15 already made `_fathomdb_source_links` authoritative for a derived
artifact's exact pinned source revision. Slice 20 must not create a parallel
source relation. Registration succeeds only when its endpoints equal that
existing complete link. The new state adds only a caller dependency ID, a
registration generation, and bounded reciprocal lookup over the same immutable
relationship.

Multi-source sets, derived-to-derived dependencies, logical-current
retargeting, and configurable liveness are allocated to
[`0.8.x-after-0.8.25-design-notes.md`](../../../../design/0.8.x-after-0.8.25-design-notes.md).
They are not dormant variants in this contract.

## Public and wire contract

```text
DependencyId(string)
SourceDependencyRegistrationV1 {
  schema_version: 1,
  dependency_id,
  source_revision_id: SourceRevisionId,
  derived_revision_id: ArtifactRevisionId
}
DependencySourceLookupV1 {
  schema_version: 1,
  source_revision_id: SourceRevisionId
}
DependencyDerivedLookupV1 {
  schema_version: 1,
  derived_revision_id: ArtifactRevisionId
}
SourceDependencyV1 {
  schema_version: 1,
  dependency_id,
  source_revision_id: SourceRevisionId,
  derived_revision_id: ArtifactRevisionId,
  registered_dependency_generation: decimal string
}
DependencyListV1 { schema_version: 1, items }
```

`DependencyId` uses Slice 15's caller-ID grammar: 1–128 ASCII bytes, an
alphanumeric first byte, remaining bytes from `[A-Za-z0-9._:-]`, and no
reserved `_fdb:` prefix. The source must be a complete Slice 15 canonical
source revision. The dependent must be a distinct complete caller-derived
revision. Both references are pinned and never retarget when either logical
record receives a new revision. A derived revision has at most one dependency
row in 0.8.25. Multi-source support requires a successor schema and contract;
it cannot silently relax this relation's uniqueness.

The exact Rust surface is:

```text
SourceDependencyRegistrationV1::new(
  dependency_id: impl Into<String>,
  source_revision_id: impl Into<String>,
  derived_revision_id: impl Into<String>
) -> Result<Self, DependencyError>

DependencySourceLookupV1::new(
  source_revision_id: impl Into<String>
) -> Result<Self, DependencyError>

DependencyDerivedLookupV1::new(
  derived_revision_id: impl Into<String>
) -> Result<Self, DependencyError>

Engine::register_source_dependency(
  request: SourceDependencyRegistrationV1
) -> Result<SourceDependencyV1, EngineError>

Engine::dependencies_for_source(
  request: DependencySourceLookupV1
) -> Result<DependencyListV1, EngineError>

Engine::dependency_for_derived(
  request: DependencyDerivedLookupV1
) -> Result<Option<SourceDependencyV1>, EngineError>
```

Constructors are the dependency-request validation boundary. They accept raw
strings, reuse Slice 15's grammar, and map invalid endpoint values to
`dependency_reference_invalid` at `/sourceRevisionId` or
`/derivedRevisionId`; they do not expose Slice 15's `/provenance/...` paths.
Stored and returned endpoints use the existing `SourceRevisionId` and
`ArtifactRevisionId` types.

Python exposes `register_source_dependency`, `dependencies_for_source`, and
`dependency_for_derived`; those methods accept closed snake-case request
mappings and return typed objects. TypeScript exposes
`registerSourceDependency`, `dependenciesForSource`, and
`dependencyForDerived`; those methods accept the corresponding camel-case
request objects and return `Promise<SourceDependencyV1>`,
`Promise<DependencyListV1>`, and `Promise<SourceDependencyV1 | null>`.
Python result attributes are snake case; TypeScript results are camel case.

Requests are closed. Validation order is schema discriminator, deterministic
unknown-field selection, required field/type in the order below, identity
grammar in that same order, then database-dependent validation:

- registration: `dependencyId`, `sourceRevisionId`, `derivedRevisionId`;
- source lookup: `sourceRevisionId`; and
- derived lookup: `derivedRevisionId`.

Multiple unknown fields choose the lexicographically smallest canonical
camel-case name and escape it only after selection. Responses are open to
additive unknown object fields but reject a new schema version. Dependency
generations are canonical unsigned decimal strings in Python and TypeScript;
Rust stores/exposes `u64`.

Registration is idempotent for an identical dependency ID and authoritative
endpoints. An exact replay returns the stored row without allocating a cursor.
Reuse of that ID or derived revision with different endpoints fails. There is
no retire, detach, restore, or retarget operation. A registration remains part
of lifecycle closure until its source or derived artifact is hard-erased.

## Persistence and mutation boundaries

Schema step 28 adds no backfill and creates:

```text
_fathomdb_source_dependencies(
  schema_version INTEGER NOT NULL CHECK(schema_version = 1),
  dependency_id TEXT PRIMARY KEY,
  derived_revision_id TEXT NOT NULL UNIQUE,
  registered_dependency_generation INTEGER NOT NULL
    CHECK(registered_dependency_generation > 0)
)
INDEX _fathomdb_source_links by (source_revision_id, artifact_revision_id)
```

The dependency table deliberately does not duplicate `source_revision_id`.
Forward lookup joins registration by `derived_revision_id` to the
authoritative `_fathomdb_source_links` row. Reverse lookup does the same join.
Registration, replay, and both reads validate the complete chain:

```text
dependency registration -> derived artifact owner -> derived source link ->
canonical artifact owner/node -> source-version mapping -> canonical self-link
```

Every persisted schema version, role, completeness value, artifact/source
revision ID, source ID/version pair, and reciprocal reference must agree.
Requested absence is `dependency_reference_missing`; a present but incomplete
owner is `dependency_provenance_incomplete`. After registration, any missing
row, drift, malformed value, or disagreement in that chain fails closed as
`EngineError::Storage`, including exact replay.

No SQLite foreign-key mode change is introduced. Dependency mutation does not
consume the canonical/global write cursor and never writes
`_fathomdb_projection_terminal`. It therefore cannot create a projection gap
or alter readiness/rebuild behavior.

Migration step 28 initializes the mandatory `_fathomdb_open_state` key
`_fathomdb_dependency_generation` to canonical decimal `"0"`. This is a
separate monotonic state version, not a record identity or write boundary.
At schema 28, open requires exactly one key, parses it as canonical unsigned
decimal no greater than SQLite `i64::MAX`, and requires it to be at least
`MAX(registered_dependency_generation)` from extant rows. Missing, duplicate,
malformed, negative, padded, out-of-range, or regressed state fails open as
`EngineOpenError::Corruption` with `CorruptionKind::SchemaInconsistent`; it
never degrades to zero.

One committed transaction that changes dependency membership advances the
generation exactly once, regardless of how many relations it inserts or
deletes. The new generation is stored on every row inserted by that
transaction and in the singleton key. Hard erasure deletes raw dependency IDs
but advances and retains the singleton generation. Replay, refusal, reads, and
an erasure that removes no dependency touch neither generation nor the
canonical/global cursor. Generation exhaustion at `i64::MAX` refuses the whole
dependency-changing transaction atomically.

Registration validates before mutation:

1. both immutable revisions exist and have the required roles;
2. both owners are `complete`, not runtime/legacy incomplete;
3. the source is a canonical node revision and the dependent has role
   `derived_semantic`;
4. the dependent's authoritative `_fathomdb_source_links` row exists and names
   exactly the requested source revision;
5. the endpoints differ and no dependency already owns the derived revision;
6. no other dependency ID owns the endpoint pair; and
7. IDs and versions satisfy the public grammar.

The role restriction makes a cycle structurally impossible. Self-reference,
derived-as-source, and canonical-as-dependent requests return typed
`dependency_cycle_or_role_invalid`; the Engine must not accept a generic edge
and rely on a later closure pass to discover the error. A valid-role pair with
a different Slice 15 source link returns `dependency_provenance_mismatch`.

Slice 20 performs structural validation only: a superseded, inactive, or
valid-time-outside source revision remains registerable if it is complete and
authoritative. Slice 30 adds lifecycle/barrier admission checks before the
release can ship. This avoids inventing partial liveness policy in Slice 20.

Implementation factors registration into a side-effect-free validator over
persisted plus prospective revision/source-link state and a transaction-scoped
apply helper taking the enclosing SQLite transaction, prospective state, and
the enclosing transaction's next dependency generation. The apply helper only
inserts the validated relation with that generation. The transaction owner is
solely responsible for loading/checking the current generation, reserving at
most one successor generation for all membership changes, applying every
relation mutation, updating the singleton key once, and publishing no
in-memory cursor state before commit.

The standalone method owns those steps around `BEGIN IMMEDIATE`. Slice 25
reuses the same helpers for records created earlier in one atomic actuation
batch without nesting a transaction; its receipt adds optional
`resulting_dependency_generation`, while `resulting_write_boundary` continues
to describe only the existing canonical/global cursor. Prospective validation
includes all earlier batch records, links, and dependency IDs. Rollback leaves
both generation and cursor state unchanged.

A rejected operation changes none of the canonical, revision,
source-link, dependency, projection, queue, registry, terminal, or cursor
state. Storage errors roll back the whole transaction and do not update the
in-memory cursor.

## Bounded reciprocal reads

Until Slice 45, source lookup returns at most 100 ordered entries. More than
100 matching entries returns `dependency_lookup_bound_exceeded` with no
partial result. The Engine queries 101 rows to distinguish a full result from
overflow. `dependencies_for_source` orders by
`(derived_revision_id, dependency_id)`. `dependency_for_derived` returns zero
or one row. Input identity is validated, but absence returns an empty list or
`None`/`null` rather than revealing whether a revision once existed.

The same normalized table and indexes serve both directions. No client-side
scan, hidden unbounded fetch, or application-owned reverse index is part of
the contract. Dependency continuation is allocated to the 0.8.27 cursor
protocol, not Slice 45's canonical and `operational_state` scope.

## Lifecycle and compatibility

Slice 30 consumes every registered dependency for lifecycle and erasure
closure and derives eligibility from endpoint lifecycle and barriers. The
dependency itself has no liveness state. Source supersession never silently
retargets a dependency. A future revision requires its own provenance and
dependency registration.

The existing `purge(logical_id)` removes all revisions of the target node and
its touching edges. In that same transaction, Slice 20 deletes registrations
for every purged derived-node or derived-edge revision. If a purged canonical
source revision has a registered dependent outside the affected cursor set,
purge fails closed until Slice 30 supplies governed closure. Source erasure
deletes registrations whose derived owner belongs to the erased source bucket
and proves that no registration still joins to its source revisions. A raw
registration with a missing derived owner, missing/mismatched source link, or
outside-bucket owner fails closed rather than being silently detached.
Reindex/rebuild never remints dependency identity or its registered generation.

The feature is additive. Existing records remain readable. Legacy derived
records without registered dependencies remain explicitly unlinked and cannot
claim dependency-complete evidence. All public and persisted objects follow
Slice 15 version, unknown-field/variant, typed-error, Rust/Python/TypeScript,
Windows CPU/native, and locally packaged parity rules. There is no automatic
registration or migration from existing `_fathomdb_source_links`; the caller
must opt into dependency identity. Hard erasure deletes the caller dependency
ID; reuse after complete erasure is permitted with a later generation. Only the
non-content singleton generation remains.

## Typed failures

Rust adds `DependencyError { reason, field_path }` under
`EngineError::Dependency`. Python raises `DependencyError`; TypeScript uses
`FDB_DEPENDENCY`. All expose this closed lower-snake-case reason set:

```text
dependency_id_invalid | dependency_reference_invalid |
dependency_reference_missing |
dependency_provenance_incomplete | dependency_provenance_mismatch |
dependency_cycle_or_role_invalid | dependency_conflict |
dependency_lookup_bound_exceeded | dependency_generation_exhausted |
unsupported_schema_version | unknown_field
```

`field_path` is an RFC 6901 pointer over canonical camel-case request names.
`dependency_id_invalid` uses `/dependencyId`.
Cross-row conflicts use an empty path; request-local validation names the
field. Errors carry no body, source text, or discovered stored identifier.
Every refusal is atomic. A persisted unknown schema version, invalid generation,
or inconsistent owner/link join fails closed as `EngineError::Storage` until
Slice 55 adds operator-visible integrity findings.

## RED/GREEN and verification

RED commits precede production changes and cover:

- schema step 28, no backfill, constraints, and migration idempotency;
- ID grammar, exact replay, conflicting reuse, generation monotonicity,
  generation exhaustion, and rollback snapshots;
- missing/malformed/out-of-range/regressed generation state, erased-highest
  restart, batch coalescing, and replay non-advance;
- registration between pending and later canonical projections, readiness
  drain, close/reopen, hard erasure, full rebuild, and vector-only rebuild,
  proving no canonical cursor or terminal changes;
- Slice 25 prospective-state reuse, one-generation ownership, receipt fields,
  and rollback leaving dependency generation and canonical cursor unchanged;
- missing, incomplete, wrong-role, self, and provenance-mismatched references;
- raw corruption of every derived-owner, source-link, canonical-owner,
  canonical-node, source-version, and canonical-self-link chain member,
  including exact replay;
- reciprocal lookup, stable order, 100/101 bounds, restart, supersession, and
  reindex;
- purge/source-erasure cleanup, surviving-dependent refusal, and raw orphan
  corruption from both endpoint axes;
- property-based ID/order/reciprocity/replay invariants;
- strict Python/TypeScript request codecs and one canonical cross-SDK fixture;
  and
- facade exports, updates to `dev/interfaces/{rust,python,typescript,wire}.md`,
  Windows builds, and locally packaged SDK behavior.

Close with focused schema/Engine/property/binding/lifecycle tests,
`scripts/agent-verify.sh --tier=fast`, applicable all-feature tests, Windows
CPU/native jobs, packaged smokes, operator purge/erasure tests, and focused
projection-rebuild non-regression. Use `--tier=heavy` only for a discovered
cross-cutting risk. CUDA, live-model, and pre-publication registry routes are
N/A.

Stop on duplicate source authority, fabricated completeness, partial mutation,
unbounded reads, stale dependency rows after erasure, or cross-SDK semantic
drift. An independent design PASS is required before RED begins.
