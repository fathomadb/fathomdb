---
title: 0.8.25 Slice 50 — compact source-complete evidence design
status: REVIEW_PENDING
design_version: 9
target_release: 0.8.25
depends_on: 45
architecture: dev/design/fathomdb-data-plane-architecture-v2.md
---

# Slice 50 design

## Decision and scope

Slice 50 implements the retained subset of N25-02, R25/AC25-50, Memex needs
10/11 and its share of 12, and A25-02/A25-05/A25-07. FathomDB returns a compact
optional evidence reference and safely resolves it to one exact canonical
source revision. It does not decide whether evidence entails an answer.

This design supersedes the Slice 50 portion of the pre-scope-adjustment
cross-slice draft. It reuses:

- Slice 15 immutable artifact/source identities, UTF-8 locators, and hashes;
- Slice 20's zero-or-one source dependency bound;
- Slice 30 lifecycle/erasure fences;
- Slice 35 canonical `ReadContextV1` encoding and frozen authority;
- Slice 40 projection generation identity; and
- Slice 45's content-free authenticated-token pattern.

No schema step is added. The existing database identity and read-context
signing key are reused with an evidence-specific domain separator. Persisted
evidence leases/receipts, multi-source evidence, replay retention, narrower
context proofs, graph-path evidence, semantic verification, and citation
wording are allocated after 0.8.25.

## Requirements and acceptance

| Requirement | Acceptance boundary |
| --- | --- |
| S50-R1 opt-in carrier | S50-AC1 an evidence search returns one sidecar entry per hit, in result order, with matching index and artifact revision; any unrepresentable hit fails the whole call. Existing search results are unchanged. |
| S50-R2 content-free reference | S50-AC2 the canonical reference is authenticated, database-scoped, at most 2 KiB, and contains no query/body/span/source/logical/natural identity, locator text, or eligibility value. Tamper and noncanonical encodings fail closed. |
| S50-R3 exact one-source resolution | S50-AC3 success returns the exact artifact revision, source/version/revision, whole canonical UTF-8 body, selected byte-aligned span, SHA-256, validity/lifecycle state, projection origin, retrieval contribution, and optional registered dependency. |
| S50-R4 eligibility-bound authorization | S50-AC4 resolution requires a valid newly supplied frozen context whose canonical resolved `ReadContextV1` exactly matches the originating envelope; the current snapshot applies the full filter to the artifact and the defined source-byte subset to the source. A reference never grants access. |
| S50-R5 non-disclosure and lifecycle | S50-AC5 malformed, foreign, mismatched, invisible, stale, inactive, superseded, erased, missing, or closure-fenced cases return the same `evidence_unavailable` reason/path with no identity or state detail. Only an authenticated, visible row may return corruption/incomplete detail. |
| S50-R6 atomicity and compatibility | S50-AC6 search, provenance resolution, and reference creation use one validated reader snapshot; no partial sidecar or persistence occurs. Existing APIs, schema 33, default SQL/hit shape, database, and WAL remain unchanged. |
| S50-R7 parity | S50-AC7 Rust, Python, TypeScript, canonical JSON fixtures, fresh packages, restart, and Windows native builds agree on versions, u64/f64 encoding, errors, and exact bytes. |

## Public contract

Rust introduces the following additive types and methods; Python uses
snake-case field names and TypeScript uses lower camel case. Dynamic-language
requests reject unknown fields before execution. Responses may ignore additive
unknown fields, but unknown identity, visibility, lifecycle, contribution, or
reference variants reject.

```text
EvidenceSearchRequestV1 {
  schema_version: 1,
  query: string,
  context: FrozenReadContextV1,
  rerank_depth: u32,
  use_graph_arm: bool,
  alpha: finite f64,
  pool_n: u32,
  include_explanation: bool,
  limit: u32                 // 1..=100
}

EvidenceRefV1(string)        // opaque; no public payload decoder

EvidenceSidecarEntryV1 {
  schema_version: 1,
  result_index: u32,
  artifact_revision_id: string,
  evidence_ref: EvidenceRefV1
}

EvidenceSearchResultV1 {
  schema_version: 1,
  search_result: SearchResult,
  evidence: [EvidenceSidecarEntryV1]
}

EvidenceResolveRequestV1 {
  schema_version: 1,
  evidence_ref: EvidenceRefV1,
  context: FrozenReadContextV1
}

ResolvedEvidenceV1 {
  schema_version: 1,
  logical_id: string?,
  artifact_revision_id: string,
  source_id: string,
  source_version_id: string,
  source_revision_id: string,
  locator: SourceLocator,
  canonical_source_body: string,
  evidence_text: string,
  canonical_source_hash: CanonicalHash,
  effective_valid_at: i64,
  artifact_lifecycle: EvidenceArtifactLifecycleV1,
  source_lifecycle_state: LifecycleState,
  projection_origin: EvidenceProjectionOriginV1,
  retrieval_contribution: EvidenceContributionV1,
  dependency: SourceDependencyV1?
}
```

The nested types are closed and versioned:

```text
EvidenceArtifactClassV1 = node | edge

EvidenceArtifactLifecycleV1 =
  node { state: pending | active | deleted | purged, superseded: bool }
  edge { superseded: bool, valid_at_effective: bool }

EvidenceArmV1 = vector | text | text_edge | graph_arm

EvidenceGraphOriginV1 =
  entity_seed
  edge_seed { edge_artifact_revision_id: string }
  traversal { edge_artifact_revision_id: string, hop_count: u32 }

EvidenceProjectionOriginV1 {
  schema_version: 1,
  artifact_class: EvidenceArtifactClassV1,
  representative_arm: EvidenceArmV1,
  projection_generation_id: string,
  graph_origin: EvidenceGraphOriginV1?
}

EvidenceContributionV1 {
  schema_version: 1,
  vector_rank: u32?,
  text_rank: u32?,
  graph_rank: u32?,
  fused_score: finite f64,
  ce_score: finite f64?,
  blended_score: finite f64,
  importance: finite f64?,
  confidence: finite f64?
}

EvidenceErrorReasonV1 =
  unsupported_schema_version
  unknown_field
  evidence_unavailable
  evidence_incomplete
  evidence_corrupt

EvidenceErrorV1 {
  reason: EvidenceErrorReasonV1,
  field_path: RFC-6901 string
}
```

`artifact_revision_id` and the resolved source fields always describe the
artifact whose `body` appears in the associated `SearchHit`. For a graph-arm
node that is the reached node and its own canonical source. `graph_origin`
separately identifies the exact edge that introduced the candidate, when an
edge exists. It does not claim that the edge's source bytes are the body
evidence, and it is not full path evidence. An entity-FTS seed has no edge and
uses `entity_seed`. An edge-matched endpoint uses `edge_seed`; a BFS neighbor
uses `traversal` and the emitted hop count. Slice 60 may add full path evidence
without changing this distinction.

`soft_fallback` is query-level and remains only on the embedded
`SearchResult`; it is not copied into every contribution. A nullable score is
encoded as JSON `null`, never NaN or infinity. Rust `u64` values exposed to
dynamic SDKs use canonical decimal strings; `u32` ranks and indexes use JSON
numbers. Finite `f64` values use JSON numbers and the existing canonical JSON
fixture spelling. Enum values use the lower-snake-case spellings above. An
unknown request field, schema, enum, or identity/visibility/contribution
variant rejects before execution. Additive unknown response fields may be
ignored; unknown response variants reject. Unsupported request schemas use
`unsupported_schema_version` at `/schemaVersion`; unknown request fields use
`unknown_field` at the exact request field. Malformed or unknown reference
payload variants intentionally collapse to `evidence_unavailable` at
`/evidenceRef` so the reference cannot become a disclosure oracle.

Dynamic SDK response decoding is strict in this order: top-level schema,
nested sidecar/projection/contribution/dependency schemas, closed
discriminants, union payload coherence, unsigned integer bounds, finite scores,
and positional sidecar agreement. Unknown schemas return
`unsupported_schema_version` at their exact `/.../schemaVersion`; every other
native response-contract violation returns `evidence_corrupt` at the exact
camel-case response path. A whole-body locator forbids offsets; a UTF-8 locator
requires ordered canonical `u64` offsets. Node lifecycle requires a known state
and forbids `validAtEffective`; edge lifecycle requires `validAtEffective` and
forbids state. Graph origin exists exactly for `graph_arm`: `entity_seed` has
neither edge nor hop, `edge_seed` has an edge and no hop, and `traversal` has an
edge plus a bounded `u32` hop. All ranks/hops are `u32`; all contribution values
are finite; dependency generation is canonical `u64`. Validation occurs before
constructing the public object, so a newer native binary cannot be silently
downcast by an older facade.

`Engine::search_with_evidence(&EvidenceSearchRequestV1)` and
`Engine::resolve_evidence(&EvidenceResolveRequestV1)` are the only new Engine
operations. The facade re-exports all public evidence types. High-level Python
and TypeScript expose equivalent methods; they do not surface Engine-internal
write cursors.

`include_explanation` controls only the returned ordinary explanation sidecar.
The Engine always captures its existing `PerHitExplain` calculation internally
for evidence contribution, then removes the ordinary explanation when the
caller did not request it. Ranking, hit order, and scores are identical to
`search_frozen` with the same controls.

## Reference encoding

The reference is `fdbev1.<lower-hex-payload>.<lower-hex-mac>`. It is canonical:
decode followed by encode must reproduce the input exactly. Maximum encoded
length is 2,048 bytes. The HMAC is SHA-256 over
`"fathomdb.evidence-ref.v1\0" || payload`, keyed by the existing database-local
read-context key.

The payload contains only:

1. format version and a keyed commitment to the Engine-minted database
   identity;
2. a keyed commitment to the canonical originating `ReadContextV1` plus the
   effective validity instant;
3. artifact class plus internal numeric `write_cursor`;
4. domain-separated keyed commitments to artifact revision, source revision,
   locator columns, and canonical source hash;
5. a fresh 128-bit nonce plus authenticated, nonce-bound encrypted
   projection-generation selector that permits a direct primary-key probe
   without exposing the generation identity;
6. closed retrieval-arm discriminant; and
7. fixed nullable vector/text/graph ranks plus finite fused, CE, blended,
   importance, and confidence values copied from `PerHitExplain`; for graph
   candidates, a graph-origin discriminant, optional internal edge cursor,
   keyed edge-revision commitment, and hop count.

It never contains the query; artifact/source/logical/public identity text;
body/span bytes; source/owner/scope/attribute value; locator/path text; or
caller-defined projection name. Artifact and source revision IDs are returned
in the sidecar/result only after authorization; the opaque reference stores
their keyed commitments. Each commitment is
`HMAC-SHA-256(database_key, field_domain || canonical_field_bytes)`, with a
different fixed domain for database, context, artifact revision, source
revision, locator, source hash, protected projection generation, and graph
edge revision. No unkeyed digest of a private value appears in the reference.
The generation selector is encrypted by XOR with two domain-separated
HMAC-SHA-256 stream blocks derived from the database-local key and a fresh
SQLite `randomblob(16)` nonce. The nonce is visible and the ciphertext is
covered by the reference's outer HMAC. A nonce is minted independently per
reference, so knowledge of a current public generation ID and one token cannot
recover another token's stream or retired generation ID. Tests include this
known-plaintext cross-token case in addition to plaintext/dictionary scans.
Fixed contribution fields eliminate truncation and the prior
arbitrary 16-component ambiguity. Nonfinite scores, ranks above `u32`, unknown
arms, oversized payloads, or noncanonical option encodings fail the entire
opt-in search.

Deliberately visible token metadata is limited to the format version,
artifact class, internal numeric artifact cursor, arm/ranks/scores, effective
validity instant, and—only for graph candidates—the internal edge cursor,
origin kind, and hop count. These values are needed to locate and validate
state or are authorized retrieval metadata; none is a caller identity or
source/body commitment. The privacy suite tests not only plaintext markers but
low-entropy candidate dictionaries: raw SHA-256 values for guessed source,
revision, locator, hash, context, and generation values must not occur or be
verifiable without the database-local key.

## Search and sidecar construction

1. Validate the request and authenticate its frozen context.
2. Dispatch the evidence-only reader request. Its transaction-owning wrapper
   opens and validates one frozen reader transaction, then calls the shared
   search-on-snapshot core with `explain=true` and a private
   `EvidenceOriginCapture` implementation. Eligibility remains before every
   retrieval-arm truncation.
3. Before that transaction closes, validate the bound frozen snapshot again.
   For each result, use its captured `(artifact_class, write_cursor)` to join
   exactly one
   `_fathomdb_artifact_revisions` row and exactly one complete
   `_fathomdb_source_links` row. A derived artifact may additionally join one
   Slice 20 dependency. A graph origin carries the edge cursor captured at the
   exact seed/traversal site; resolve and validate that edge revision in the
   same transaction. Never infer it from the hit's legacy `source_id`.
4. Validate that the hit, explanation entry, revision row, source link,
   projection generation, graph origin, and dependency agree. Decrypt the
   authenticated nonce-bound generation selector and perform one primary-key
   lookup; never scan retained generation history. The existing
   GraphArm `SearchHit.source_id` remains the traversed edge's source for
   compatibility and may differ from the node body source returned by evidence;
   that difference is expected and is not an agreement condition. Build the
   sidecar and HMAC
   references in memory. Any missing, duplicate, incomplete, over-cap, or
   inconsistent row fails the whole operation.
5. Revalidate the frozen binding on the same transaction, commit the read, and
   return. Strip `SearchResult.explanation` when `include_explanation=false`.

The implementation factors the current transaction body into a private
`read_search_on_snapshot<C: SearchOriginCapture>` core. The existing
`read_search_in_tx` remains the transaction-owning ordinary wrapper and calls
the core with a zero-sized `NoEvidenceCapture`; monomorphization and inlining
remove capture calls, branches, maps, provenance SQL, and allocations from that
path. A separate `read_search_with_evidence_in_tx` wrapper owns the evidence
transaction and calls the same core with `EvidenceOriginCapture`. Candidate
creation records artifact class at node/edge hydration. Graph candidate
creation additionally records `entity_seed`, or the exact edge cursor and hop
at the seed/traversal statement. Fusion retains the origin of its existing
representative hit. This is the required same-snapshot seam: search and
provenance lookup never run in different transactions. No evidence row, query,
sidecar, or reference is written to SQLite, WAL, telemetry, or an Engine cache.

Content-ID hits may resolve only when their internal cursor has a complete
Slice 15 provenance row. Legacy `migrated_incomplete` rows make the entire
opt-in call fail `evidence_incomplete`; ordinary search remains available.
Passage IDs cannot originate from Engine search and are rejected.

## Equivalent context and current authorization

The reference binds the canonical encoding of the originating frozen
context's already-resolved `ReadContextV1`, not its state token. Resolution may
therefore use a newly minted frozen context after restart or unrelated state
change only when:

- its schema is supported and token authenticates for this database;
- its frozen snapshot validates now;
- its effective validity instant and canonical context digest exactly equal
  the origin; and
- its view and eligibility currently admit the artifact, while the source-byte
  authorization subset below admits the canonical source.

Callers re-mint equivalently by passing the original resolved
`frozen.context`, including its concrete `valid_as_of`. A broader, narrower, or
otherwise different view/filter is not equivalent in v1. Predicate reordering
normalizes through Slice 35's canonical context encoding.

This separation is intentional: the evidence reference records origin, while
the supplied frozen context authorizes the current read. Reindexing or an
unrelated write does not invalidate otherwise live evidence merely because the
original state token drifted. The originating projection generation must still
exist and match its stored identity, but it need not remain serving.

## Resolution and error precedence

Resolution runs in one reader transaction and uses this fixed order:

1. reject an unsupported request schema;
2. enforce reference/context size bounds;
3. authenticate and validate the supplied frozen context;
4. authenticate and canonically decode the reference/database identity;
5. compare the exact context digest and effective validity instant;
6. locate the artifact/source rows by internal cursor, verify all keyed
   commitments, apply the full supplied view and eligibility to the artifact,
   and apply the source-byte authorization subset below before returning any
   field;
7. when graph origin names an edge, resolve its captured cursor to the same
   committed edge revision, verify its keyed revision commitment and `edge`
   artifact class, and apply the graph-origin authorization rules below;
8. recheck Slice 30 source/dependent closure barriers and current lifecycle,
   erasure, validity, and supersession state for every applicable subject;
9. only after current authorization, validate provenance completeness,
   locator UTF-8 boundaries, whole-source SHA-256, projection-generation
   identity, contribution values, and optional dependency; and
10. revalidate the frozen snapshot before returning exact bytes.

Steps 2–8 collapse to one public error:

```text
EvidenceError {
  reason: evidence_unavailable,
  field_path: "/evidenceRef"
}
```

The payload contains no nested cause, identity, lifecycle state, or existence
signal. Malformed, forged, foreign-database, mismatched-context, missing,
superseded, inactive, invalid-at-time, ineligible, erased, dependency-closed,
and stale-digest cases are deliberately indistinguishable. Tests compare the
complete serialized error. Constant-time HMAC comparison is retained; the
remaining fixed bounded probes make no network or content-bearing log.

After the artifact and source are currently authorized, only two more reasons
may escape:

- `evidence_incomplete` at `/provenance` for a visible
  `migrated_incomplete`/unlinked legacy artifact; and
- `evidence_corrupt` at the failing structural field for an authenticated,
  visible row whose persisted locator/hash/dependency/generation contract is
  internally inconsistent.

Storage failure and Engine closing retain their existing top-level errors.

### Graph-origin authorization

`graph_origin` is a third authorization subject, distinct from the returned
body artifact and its canonical source. `entity_seed` has no edge subject. For
`edge_seed` or `traversal`, the resolver must, in the same frozen reader
transaction:

1. locate the captured edge cursor and matching `_fathomdb_artifact_revisions`
   row;
2. verify class `edge`, the keyed edge-revision commitment, and that the edge
   still connects the returned node's logical identity;
3. require `superseded_at IS NULL`, reject temporal-fallback edges, and apply
   the same edge validity interval at `effective_valid_at` used by graph-arm
   admission;
4. reapply the original endpoint admission rule: the returned node remains an
   eligible endpoint under the full context filter and view; and
5. reject any active dependency-closure barrier or erased/missing dependency
   state for the edge revision.

This is last-edge origin validation, not full path replay. If any check fails,
the whole resolution returns the identical `evidence_unavailable` error and
does not disclose the edge revision ID, body/source identity, or failure
detail. Only after these checks may `graph_origin.edge_artifact_revision_id`
be returned. Post-mint edge supersession, erasure, replacement, validity, and
endpoint-eligibility tests enforce the rule.

### Eligibility subjects

The origin search and current resolution apply the complete `SearchFilter` to
the returned artifact. Predicates do not silently change subjects:

| Predicate | Hit artifact | Canonical source bytes |
| --- | --- | --- |
| `source_type` | Apply using existing node/edge search semantics. | Do not apply; it classifies the candidate artifact. |
| `kind` | Apply to the node kind or edge relation exactly as search does. | Do not apply; a derived claim and its source document legitimately differ. |
| `created_after` | Apply using the existing indexed candidate metadata. | Apply to the canonical source's indexed metadata. |
| `status` | Apply using the existing indexed candidate metadata. | Apply to the canonical source's indexed metadata. |
| every declared attribute equality | Apply before candidate truncation. | Apply to the source's own `canonical_attributes` row; missing or unequal denies bytes. |

The supplied `ReadView`, source lifecycle/supersession state, validity at the
effective instant, erasure state, and dependency-closure barrier always apply
to the canonical source. Thus a derived `kind=claim` may resolve a canonical
`kind=document` when their access-bearing metadata agrees, while an owner/scope
attribute mismatch or absence returns only `evidence_unavailable`. FathomDB
does not assign semantic meaning to attribute names; conservatively applying
every caller-supplied attribute term prevents an eligible derived row from
bypassing source-byte eligibility.

## Exact bytes and lifecycle semantics

The canonical source is the node named by `source_revision_id`. Resolution
loads its SQLite TEXT body as UTF-8, recomputes SHA-256 over those exact UTF-8
bytes, and compares the stored whole-source hash. `WholeBody` returns that body
as `evidence_text`; `Utf8Bytes` returns the half-open byte slice only if both
bounds are ordered, in range, and code-point aligned.

`artifact_lifecycle` is total by captured class. A node reports the existing
closed `LifecycleState`—with exactly `pending`, `active`, `deleted`, and
`purged` wire spellings—and its `superseded` flag. An edge reports its
`superseded` flag and whether its validity interval contains the effective
instant; edges do not fabricate the node-only `state` axis. The canonical
source is always a node and reports `source_lifecycle_state`. Successful strict
resolution reports an active, nonsuperseded, valid artifact and source; the
fields remain explicit so the evidence is self-describing. A valid-as-of
request may resolve historical world-time content, but this slice does not
make search version-complete and continues to reject
`include_superseded`/`include_inactive` on search. Transaction-history access is
not implied.

Physical erasure removes the revision/link/source bytes. A retained reference
then returns only `evidence_unavailable`. Supersession, deletion, invalidation,
or closure fencing does the same under a strict origin envelope. Reactivation
may make a still-matching revision resolvable again only if all original
keyed identity commitments and the exact context remain valid; a replaced
revision does not inherit the old reference.

## Compatibility, privacy, and performance

- `SearchHit`, `SearchResult`, existing search methods, schema 33, migrations,
  and default serialization remain byte-for-byte unchanged.
- Reference creation is opt-in and stateless. A before/after database, WAL,
  query-log, and telemetry witness proves no new persistence.
- A privacy fixture uses unique query, source, logical, revision, owner, and
  body markers, scans decoded reference payload/string output for each, and
  attempts low-entropy SHA-256 dictionary matches against every keyed field.
- The 2 KiB limit and fixed contribution shape bound per-hit memory. The result
  limit remains 100, bounding one response to 100 references.
- Slice 75 measures evidence create/resolve latency and memory in its integrated
  workload. Slice 50 proves default search executes the unchanged path and no
  new provenance SQL or allocation branch.
- Optional CE reranking is represented by the existing nullable CE score; the
  all-feature route proves a finite CE contribution when a deterministic test
  reranker is active. Installed CE performance remains assigned to Slice 75.

## Test design

The preserved RED set includes:

1. real-database whole-body and UTF-8-span success, restart, equivalent remint,
   and valid-as-of cases;
2. property tests for canonical codec round-trip, single-bit tamper, length,
   noncanonical encodings, finite floats, and privacy markers;
3. exact sidecar order/association, explanation composition, node-text/vector,
   edge-text/vector, and graph entity-seed/edge-seed/traversal contribution;
   graph cases prove body-source versus traversal-edge distinction, and one
   incomplete hit causes whole-request rollback;
4. wrong database, mismatched/broader/narrower context, supersession,
   lifecycle deletion, erasure, validity, eligibility, closure fence, and
   replacement all producing byte-identical `evidence_unavailable` errors;
   derived-source fixtures prove different artifact/source kinds can succeed,
   while missing or different owner/scope-style attributes deny bytes;
   graph fixtures mutate the captured edge after mint through supersession,
   erasure, replacement, validity, endpoint eligibility, and closure and prove
   the same non-disclosure result;
5. authorized locator/hash/link/dependency/generation corruption producing only
   `evidence_corrupt`, and visible legacy rows producing
   `evidence_incomplete`;
6. mutation races at post-search/pre-sidecar and pre-return boundaries proving
   same-snapshot result or whole-call refusal;
7. default-search parity plus zero evidence-table query/persistence witness;
8. strict unknown schema/field/variant and u64/f64 wire fixtures across all
   SDKs; and
9. fresh Linux wheel/npm and Windows native end-to-end package smokes.

No test mocks SQLite, changes a historical fixture, or uses generated semantic
oracles. The existing query-plan and connection-attribution invariants remain
load-bearing for any new test hook.

## Documentation and completion

Implementation adds one evidence ADR, updates the decision index and
Rust/Python/TypeScript/wire interfaces, and records RED, design review,
implementation review, independent verification, and status in this slice
directory. The slice is complete only when all S50 acceptance rows, selected
platform/package routes, release-state views, Markdown checks, and final fast
gate pass at reviewed Git commits.
