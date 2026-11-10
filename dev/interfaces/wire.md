---
title: Wire Format
date: 2026-11-04
target_release: 0.8.28
desc: On-disk + IPC formats (if any) for 0.8.28; short OK
blast_radius: design/fathomdb-data-plane-architecture-v2.md; design/engine.md; design/migrations.md
status: draft-0.8.28
---

# Wire Format

FathomDB has no standalone IPC wire protocol. The public wire surface is limited
to the on-disk file layout and the schema-version sentinel used on the open
path.

## File layout

The file set is:

- `<db-name>.sqlite`
- `<db-name>.sqlite-wal`
- `<db-name>.sqlite.lock`
- optional `<db-name>.sqlite-journal`

The current layout and admission owners are [engine](../design/engine.md) and
[migrations](../design/migrations.md).

## Schema-version sentinel

The canonical schema-version sentinel is SQLite `PRAGMA user_version`. In the
0.8.28 line `fathomdb-schema::SCHEMA_VERSION` is **37**. Step 28
adds the source-dependency registry and generation singleton. Step 29 adds the
bounded terminal actuation-receipt and source-reference tables without
backfilling legacy rows. Actuation request bodies and source locators are never
persisted in those tables; source erasure replaces a matching terminal receipt
with an opaque operation-ID tombstone. Step 30 adds content-free direct
dependency-closure rows, their monotonic sequence singleton, recovery/root
indexes, and active-retry uniqueness. It performs no canonical-content rewrite
and synthesizes no closure for historical lifecycle events. Step 31 adds the
database-local frozen-read identity, signing key, monotonic visibility
generation, and checked triggers on all real serving-authority tables. It adds
no canonical or projection backfill.

Step 32 adds retained projection-generation metadata, its current-generation
singleton, and the nullable actuation-receipt generation correlation. It does
not rewrite canonical or projection content. The database-local generation ID
has grammar `pgen1:<32-lower-hex>`.

Slice 60 adds no migration: constrained graph expansion reuses the endpoint
indexes introduced in step 12 and recreated in step 23. Step 33 instead adds
Slice 45 page indexes and visibility state/triggers; it has no data migration
or canonical/projection rewrite. Step 34 is content-free: it identifies a
fresh database and changes no canonical rows.

Ownership split:

- this file owns the fact that `PRAGMA user_version` is the public on-disk
  sentinel for schema-version compatibility
- `design/migrations.md` owns how successful migrations advance it
- `design/engine.md` owns when it is read on the open path

Format 35 adds per-vector UTF-8 head spans, permits distinct vector IDs for
future windows of one record, uses 64-row vec0 chunks, and keys node body
FTS rowids by write cursor. No format-34 content is migrated. Coverage and
vectors are committed and removed together; unknown coverage is NULL.

Format 37 adds a constrained transaction-owned visibility coalescing marker.
It retains the format-36 maintenance tables/policy and format-35 vector layout.
Its fresh-bootstrap step adds the column before replacing all 57 visibility
triggers, without canonical or projection backfill. The triggers retain external-writer invalidation; vector mapping
DELETE cleanup stays active. Earlier nonempty formats are refused, not migrated.

## Compatibility contract

- a missing or zero-length path bootstraps directly to schema 37
- a non-empty database opens only when its effective committed
  `PRAGMA user_version` is exactly 37; schema 36 and earlier, future versions, and
  non-empty zero-version SQLite files return the typed incompatible-schema
  error before product mutation
- there is no public automatic migration, compatibility reader, translator,
  or historical receipt/replay reader for earlier database layouts

## Non-surface

- no separate IPC frame format
- no secondary version manifest file
- no public promise around internal SQLite page layout beyond what the on-disk
  files and `PRAGMA user_version` already expose

## Frozen-read wire object (0.8.25 Slice 35)

The cross-SDK object is `FrozenReadContextV1`: `schemaVersion` 1,
`effectiveValidAt`, an echoed `ReadContextV1`, and an opaque database-local
token. The token is bounded to 1 KiB, contains no query, eligibility value, or
stored content, and is not portable to a separately created database. Readers
must ignore additive response fields but reject an unsupported schema version.
The refusal envelope uses `FDB_FROZEN_READ` with closed `reason` and
`fieldPath` members.

## Projection-generation wire objects (0.8.25 Slice 40)

`ProjectionGenerationStatusV1` and `MutationProjectionStatusV1` use
`schemaVersion: 1`. Generation IDs and declaration hashes are lowercase text;
all `u64` cursors, boundaries, and counts cross Python/TypeScript native
boundaries as canonical decimal strings. Readiness is exactly `ready`,
`processing`, `blocked`, `deferred`, or `degraded`; runtime state is exactly
`absent`, `usable`, or `refused`; origin is exactly `fresh`,
`legacy_unverified`, `configuration`, or `rebuild`.

`MutationProjectionStatusRequestV1` contains only `schemaVersion`,
`operationId`, `writeCursor`, and `expectedGenerationId`. Unknown keys and
noncanonical values fail closed. The typed envelope code is
`FDB_PROJECTION_GENERATION`, with closed `reason` and non-disclosing
`fieldPath`. Readers may accept additive response fields but reject unknown
schema versions or closed-enum spellings.

## Pagination wire objects (0.8.25 Slice 45)

`PageRequestV1` contains `schemaVersion: 1`, `limit`, and optional opaque
`cursor`. `PageV1<T>` contains `schemaVersion: 1`, ordered `items`, and nullable
`nextCursor`. Cursors are bounded to 2 KiB and authenticate database,
operation, selector, frozen-context digest, limit, ordering version, and last
write cursor without containing caller selectors or stored content. The typed
envelope code is `FDB_PAGE`; its closed refusal object contains `reason` and
non-disclosing `fieldPath`.

## Evidence wire objects (0.8.25 Slice 50)

`EvidenceSearchRequestV1` and `EvidenceResolveRequestV1` use
`schemaVersion: 1` and require a `FrozenReadContextV1`. Search returns the
ordinary `SearchResult` plus position-associated `EvidenceSidecarEntryV1`
objects. An evidence reference is opaque, database-local, HMAC-authenticated,
content-free, and bounded to 2 KiB. It is never authorization by itself.

`ResolvedEvidenceV1` returns the exact one-source provenance, bytes, UTF-8 byte
locator, lifecycle, projection origin, retrieval contribution, and optional
direct dependency. Python integer cursors remain integers; TypeScript native
`u64` values use canonical decimal strings where applicable. Unknown request
fields and unsupported request schemas fail closed. Additive response fields
may be ignored, but unknown closed variants must be rejected. The typed
envelope is `FDB_EVIDENCE`; non-authorized cases expose only
`evidence_unavailable` at `/evidenceRef`.

## Dependency trace and explanation wire (0.8.25 Slice 55)

Trace request and response objects use `schemaVersion: 1`, camel-case fields,
lower-snake enum values, canonical unsigned-decimal strings for every new
`u64`, and declaration-order canonical JSON. Requests are closed; responses
ignore additive unknown object members but reject unknown closed variants and
incoherent lifecycle unions. Optional response values are present as `null`.

`Explanation.correlationId` follows `perHit`; `PerHitExplain.structural`
follows `confidence`. New native responses require both. The structural object
contains only schema, inclusion, projection-origin, dependency-state,
lifecycle-state, and degradation enums. Trace errors use
`FDB_DEPENDENCY_TRACE`; integrity CLI errors use
`FDB_DATA_PLANE_INTEGRITY`.

The non-empty `Explanation.correlationId` requirement applies equally to
ordinary explained search, `searchFrozen` with explanation enabled, and the
nested search result from `searchWithEvidence` with explanation included.
Explanation-disabled frozen responses carry no explanation and do not create a
telemetry record. Independent explained calls mint independent correlation IDs;
independent evidence calls also mint independent opaque evidence references.

## Graph-expansion wire objects (0.8.25 Slice 60)

`GraphExpandRequestV1` uses `schemaVersion: 1`, a closed query or explicit seed,
a closed current or frozen context, lower-snake enum values, and camel-case
fields. Every request object is recursively closed. `maxWorkUnits`, result
`workUnits`, and target `writeCursor` are canonical unsigned-decimal strings;
other bounded integers are JSON numbers and booleans are never integers.

`GraphExpandResultV1` is additive at object boundaries but closed for enums and
unions. It must carry `complete: true`, zero-based seed ordinals, target origins
coherent with seeds and enclosing targets, and an explanation whose per-target
entries and degradation codes exactly match the top-level result. The typed
envelope code is `FDB_GRAPH_EXPANSION` with closed `reason` and exact RFC 6901
`fieldPath`; frozen-context failures retain `FDB_FROZEN_READ`.

## Exact graph artifact evidence wire

`GraphExpandRequestV1` accepts optional `includeEvidence`; omission and false
are byte-equivalent. True requires a frozen context. The response then includes
`evidence: { schemaVersion: 1, entries: [...] }`, with entry `targetIndex`
equal to its zero-based position and one entry for every target. Each entry has
validated artifact revision strings and opaque target/terminal-edge references.

`GraphEvidenceResolveRequestV1` is closed to `schemaVersion`, `evidenceRef`, and
`context`. `ResolvedGraphEvidenceV1` carries a closed `artifactClass` node/edge
union and canonical source material. References use the graph-only `fdbgev1`
domain and are not interchangeable with ranked evidence references. Evidence
request/refusal envelopes use `FDB_EVIDENCE`; malformed graph result sidecars
use `FDB_GRAPH_EXPANSION` with exact RFC 6901 paths.

The reference wire grammar is exactly `fdbgev1.` followed by 696 lowercase hex
characters (`nonce[16] || ciphertext[300] || mac[32]`), 704 characters total.
The evidence sidecar and resolver carry no write cursor; the pre-existing graph
target `writeCursor` is unchanged. `includeOutOfWindow: true` is refused at
`/context/context/view/includeOutOfWindow` for evidence-bearing expansion.

## Built-in provider admission and verification metadata

Production provider admission is pinned BGE/Mean or explicit none; no arbitrary
provider selection is supported before 1.x. New projection embedder strings are
rejected before mutation, while omitted/null selectors remain valid. Existing
stored strings, profiles, probe references and TC-68 markers are retained without
migration, relabeling or provider substitution. Well-formed historical metadata
remains inspectable under deliberate no-provider access, subject to normal
integrity validation; re-applying an unsupported historical selector is refused.

The checker gate, fingerprint recipe and inputs, persistent-marker authority and
90/45/0 ordinary embedding cadence remain unchanged. A valid marker remains
reusable across restart and MCP reconstruction; cached acceptance is not fresh
verification in this process and can hide same-identity backend/device/library
drift. Marker-write failure retains its best-effort behavior. An actual refusal
returns the typed open error and no engine; newly rejected first-population
references are not published. This adds no new cache, descriptor or schema step.
[Full limits and error payloads](rust.md#built-in-provider-admission-and-cached-verification)
include deliberate mutable no-provider access and the unrepaired provider-free
dense rebuild defect. Loader/device, Mean/CLS distinction and mean recovery
ordering remain unchanged; none of this authenticates historical vector provenance.

## Format 36 explicit erasure authority

Format 36 is a fresh-store boundary. Both policies require validated persisted
physical-erasure policy and opaque store identity metadata. The bounded erasure
request/terminal-receipt table is distinct from the executing RootDebt journal.
Queued selector/cut/fence authority is not structural-zero proof. Activation binds
one ticket and its current immutable scope atomically to native deletion/debt.
Identity-bearing request material is scrubbed before identity-free finalization;
only actual registered-copy proof, original-sink discharge and WAL TRUNCATE permit
terminal Complete. Offline activated completion publishes the same ticket receipt.
The [production design](../design/explicit-maintenance-erasure/production-design.md)
owns metadata budgets, phases and validation. Format 35 binaries refuse 36, and
this release refuses previous formats; no migration or metadata-only opt-in exists.
