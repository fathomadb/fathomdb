---
title: Wire Format
date: 2026-11-23
target_release: 0.8.31
desc: On-disk + IPC formats (if any) for 0.8.31; short OK
blast_radius: design/fathomdb-data-plane-architecture-v2.md; design/engine.md; design/migrations.md
status: locked
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
0.8.31 line `fathomdb-schema::SCHEMA_VERSION` is **38**. Step 28
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

Format 38 adds nullable `properties` and inert `scope` columns to
`canonical_nodes` and `canonical_edges` (and `scope` to `operational_mutations`
and `operational_state`), the unified typed property index table
`canonical_property_values(entity_class, write_cursor, attr_name, value_type, text_val, num_val, bool_val)`
(`WITHOUT ROWID`, clustered primary key `(entity_class, write_cursor, attr_name)`)
with its three partial typed-value indexes (`canonical_property_values_text_idx`,
`canonical_property_values_num_idx`, `canonical_property_values_bool_idx`),
`_fathomdb_projection_registry` columns
`entity_class` (`'node'` | `'edge'`) and `vector_kinds`,
`_fathomdb_vector_kinds` (opt-in custom node vector-kind registry),
`_fathomdb_commit_timestamps` with `_fathomdb_commit_timestamps_ms_idx`, the
five composite node/edge indexes (`canonical_nodes_logical_sup_idx`,
`canonical_edges_from_active_idx`, `canonical_edges_to_active_idx`,
`canonical_edges_from_sup_kind_idx`, `canonical_edges_to_sup_kind_idx`), and
`cpv` visibility coalescing triggers on `canonical_property_values`. No legacy
format is auto-migrated on `Engine::open` (committed Schema 37 stores upgrade
offline via `fathomdb migrate`).

## Compatibility contract

- a missing or zero-length path bootstraps directly to schema 38
- a non-empty database opens only when its effective committed
  `PRAGMA user_version` is exactly 38; schema 37 and earlier, future versions, and
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

## Format 38 typed inline properties and operational collections (0.8.31 Slice 20)

Inline typed properties are persisted canonically as sorted-key JSON objects in
nullable `properties` columns on `canonical_nodes` and `canonical_edges`
(`NULL` when omitted or empty `{}`). Values are flat primitives only (`Str`,
signed 64-bit `Int`, finite IEEE-754 `Float` with `-0.0` canonicalized to `0.0`,
and `Bool`); `null`, nested objects/arrays, non-finite floats, empty keys, and
keys or string values containing NUL (`\0`) or RS (`\x1e`) are refused with
`WriteValidation` / `WriteValidationError`.

Graph-expansion and graph-evidence wire responses carry optional additive
`properties` on `GraphTargetV1` and `GraphEvidenceArtifactV1`, and optional
additive `terminalEdgeProperties` on `GraphOriginV1`. When present, integer
values outside IEEE-754 safe integer range (`±2^53 - 1`) remain exact signed
64-bit JSON integers; TypeScript callers parse them losslessly into `bigint`.

`operational_collections` stores registered collection definitions (`name`,
`kind`, `schema_json`, `retention_json`, `format_version`, `created_at`). Fresh
stores seed the built-in `projection_failures` operational collection. Calling
`AdminSchema` with an existing collection name and a different `kind` is refused
with `WriteValidation` / `WriteValidationError`.

`ExciseReport` / `EraseReport` adds `completed_pending_erasure`
(`completedPendingErasure` in TypeScript), a boolean indicating whether the
erasure call completed previously pending post-commit erasure work.

## Graph traversal predicates and graph-arm seed provenance wire (0.8.31 Slice 30)

`GraphExpandRequestV1` adds optional closed arrays `edgePredicates` and
`targetPredicates` (each at most `32` entries), reusing the untagged plain JSON
scalar predicate wire shape `{"key": string, "op": "eq"|"ne"|"lt"|"lte"|"gt"|"gte", "value": scalar}`
shared with `eligibility.propertyPredicates`. Both fields are omitted during
canonical encoding when empty so unpredicated request JSON stays byte-identical.
Invalid predicates return `FDB_GRAPH_EXPANSION` with reason
`graph_edge_predicates_invalid` (under `/edgePredicates/...`) or
`graph_target_predicates_invalid` (under `/targetPredicates/...`). Frozen graph
evidence commitments canonically normalize and authenticate both predicate
arrays.

`PerHitExplain` adds additive optional `graphSeedSource`
(`"fused_top_k" | "edge_fts" | "entity_fts" | null`) and `graphHopDepth`
(`u32 | null`), populated only for `arm == "graph_arm"` hits.
`EvidenceGraphOriginV1` adds required `seedSource`
(`"fused_top_k" | "edge_fts" | "entity_fts"`) on both `edge_seed` and
`traversal` origins, authenticated inside the opaque evidence token payload.

## Historical bitemporal graph expansion wire (0.8.31 Slice 40)

`GraphExpandRequestV1` adds two optional, mutually exclusive knowledge-time
bounds, both omitted during canonical encoding when absent so default
current-time request bytes stay unchanged:

- `asOfWriteCursor`: canonical unsigned base-10 decimal string (`"0"` through
  `u64::MAX`, matching `maxWorkUnits` / `writeCursor`). Non-strings, explicit
  JSON `null`, leading zeros, negative values, and values exceeding `u64::MAX`
  refuse with `FDB_GRAPH_EXPANSION`, reason `graph_context_invalid`, at
  `/asOfWriteCursor`.
- `asOfEpochMs`: signed 64-bit JSON integer (`i64`; TypeScript validates
  `Number.isSafeInteger`). Non-integers, floats, strings, booleans, explicit
  JSON `null`, and out-of-range values refuse with `FDB_GRAPH_EXPANSION`, reason
  `graph_context_invalid`, at `/asOfEpochMs`.

Supplying both `asOfWriteCursor` and `asOfEpochMs` refuses deterministically
with `FDB_GRAPH_EXPANSION`, reason `graph_context_invalid`, at `/asOfEpochMs`.
`ReadView` remains world-time-only; `includeSuperseded: true` and
`includeInactive: true` remain refused on `graph.expand` at `/context`.

Frozen graph evidence commitments bind historical requests via prefix-free
trailing tags (`2u8` + `u64` for `asOfWriteCursor`, `3u8` + `i64` for
`asOfEpochMs`, emitted after the optional `1u8` predicate block and omitted
when neither bound is set). The `fdbgev1.` reference wire grammar stays 704
characters (`nonce[16] || ciphertext[300] || mac[32]`), using reserved selector
byte `payload[7]` (`0` = current-time minted, `1` = historical `as_of` minted).
Resolved historical evidence sets `artifactLifecycle.superseded: true` when the
bound revision was later superseded in the frozen snapshot, and collapses
erased, transitioned, or closure-barrier-blocked artifacts to
`evidence_unavailable`.

## Batch evidence resolution and search hints wire (0.8.31 Slice 50)

- **`EvidenceResolveManyRequestV1` / `EvidenceResolveManyItemV1`:**
  `EvidenceResolveManyRequestV1` is closed to `schemaVersion` (`1`),
  `evidenceRefs` (`1..=50` raw strings before deduplication), and `context`
  (`FrozenReadContextV1`). Request-level schema mismatch uses `FDB_EVIDENCE`
  (`unsupported_schema_version` at `/schemaVersion`); raw count `0` or `> 50`
  uses `InvalidArgument`; embedded NUL / lone UTF-16 surrogates use
  `WriteValidation`; invalid or drifted frozen contexts collapse into whole-call
  `FDB_EVIDENCE` (`evidence_unavailable`). Each slot in the response array is
  either `{ status: "resolved", resolved: ResolvedEvidenceV1 }` or
  `{ status: "refused", reason: EvidenceErrorReasonV1, fieldPath: "/evidenceRefs/{i}" }`,
  preserving exact input order and duplicate arity. Empty, oversized (`> 2048`
  bytes), malformed, tampered, wrong-DB, wrong-context, erased, or superseded
  tokens all yield per-slot `evidence_unavailable` at `"/evidenceRefs/{i}"`.
- **`EvidenceSearchRequestV1.hints` & `SearchResult.hints`:**
  `EvidenceSearchRequestV1` accepts optional boolean `hints` (default `false`);
  omission and `false` leave `SearchResult.hints` absent/`None`. When `true`,
  `SearchResult.hints` carries closed-enum `Hint` entries
  (`action`: `"search" | "none"`, `reason`: `"degraded" | "no_results" | "more_results"`,
  optional `mode`: `"text" | "hybrid"`, optional `limit`, optional `arm`,
  optional `cause`), never containing record content.
- **Governed operation parity:** Adding `engine.resolve_evidence_many` advances
  `src/conformance/governed-operation-parity.json` and
  `src/conformance/governed-surface-allowlist.json` to `56` live operations and
  `93` allowlist members (`5` core, `5` recovery denylist, `2` runtime controls).
