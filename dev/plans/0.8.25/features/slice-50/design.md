---
title: 0.8.25 Slice 50 — compact source-complete evidence design
status: DRAFT_RECONCILED_REVIEW_PENDING
design_version: 5
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
| S50-R4 eligibility-bound authorization | S50-AC4 resolution requires a valid newly supplied frozen context whose canonical resolved `ReadContextV1` exactly matches the originating envelope and whose current snapshot authorizes both artifact and source. A reference never grants access. |
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
  artifact_lifecycle_state: LifecycleState,
  source_lifecycle_state: LifecycleState,
  superseded: bool,
  projection_origin: EvidenceProjectionOriginV1,
  retrieval_contribution: EvidenceContributionV1,
  dependency: SourceDependencyV1?
}
```

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

1. format version and Engine-minted database identity;
2. canonical originating `ReadContextV1` digest and effective validity instant;
3. artifact class plus internal numeric `write_cursor`;
4. SHA-256 digests of artifact revision, source revision, locator columns, and
   canonical source hash;
5. Engine-minted projection-generation identity;
6. closed retrieval-arm discriminant; and
7. fixed nullable vector/text/graph ranks plus finite fused, CE, blended,
   importance, and confidence values copied from `PerHitExplain`.

It never contains the query; artifact/source/logical/public identity text;
body/span bytes; source/owner/scope/attribute value; locator/path text; or
caller-defined projection name. Artifact and source revision IDs are returned
in the sidecar/result only after authorization; the opaque reference stores
their digests. Fixed contribution fields eliminate truncation and the prior
arbitrary 16-component ambiguity. Nonfinite scores, ranks above `u32`, unknown
arms, oversized payloads, or noncanonical option encodings fail the entire
opt-in search.

## Search and sidecar construction

1. Validate the request and authenticate its frozen context.
2. Dispatch the existing frozen search with `explain=true` on one reader
   transaction. Eligibility remains before every retrieval-arm truncation.
3. Before that transaction closes, validate the bound frozen snapshot again.
   For each result, join `(artifact_class, write_cursor)` to exactly one
   `_fathomdb_artifact_revisions` row and exactly one complete
   `_fathomdb_source_links` row. A derived artifact may additionally join one
   Slice 20 dependency.
4. Validate that the hit, explanation entry, revision row, source link,
   projection generation, and dependency agree. Build the sidecar and HMAC
   references in memory. Any missing, duplicate, incomplete, over-cap, or
   inconsistent row fails the whole operation.
5. Revalidate the frozen binding on the same transaction, commit the read, and
   return. Strip `SearchResult.explanation` when `include_explanation=false`.

The implementation extends the existing reader request/result only for this
new operation. It must not run search and provenance lookup on different
snapshots, and it must not add provenance work or a feature branch to any
existing search call. No evidence row, query, sidecar, or reference is written
to SQLite, WAL, telemetry, or an Engine cache.

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
- its view and eligibility currently admit both artifact and canonical source.

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
6. locate the artifact/source rows by internal cursor, verify all identity
   digests, and apply the supplied view and eligibility to both rows before
   returning any field;
7. recheck Slice 30 source/dependent closure barriers and current lifecycle,
   erasure, validity, and supersession state;
8. only after current authorization, validate provenance completeness,
   locator UTF-8 boundaries, whole-source SHA-256, projection-generation
   identity, contribution values, and optional dependency; and
9. revalidate the frozen snapshot before returning exact bytes.

Steps 2–7 collapse to one public error:

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

## Exact bytes and lifecycle semantics

The canonical source is the node named by `source_revision_id`. Resolution
loads its SQLite TEXT body as UTF-8, recomputes SHA-256 over those exact UTF-8
bytes, and compares the stored whole-source hash. `WholeBody` returns that body
as `evidence_text`; `Utf8Bytes` returns the half-open byte slice only if both
bounds are ordered, in range, and code-point aligned.

`artifact_lifecycle_state` and `source_lifecycle_state` are parsed from their
current canonical rows. `superseded` is false on strict search origins; a
valid-as-of request may resolve historical world-time content, but this slice
does not make search version-complete and continues to reject
`include_superseded`/`include_inactive` on search. Transaction-history access is
not implied.

Physical erasure removes the revision/link/source bytes. A retained reference
then returns only `evidence_unavailable`. Supersession, deletion, invalidation,
or closure fencing does the same under a strict origin envelope. Reactivation
may make a still-matching revision resolvable again only if all original
identity digests and the exact context remain valid; a replaced revision does
not inherit the old reference.

## Compatibility, privacy, and performance

- `SearchHit`, `SearchResult`, existing search methods, schema 33, migrations,
  and default serialization remain byte-for-byte unchanged.
- Reference creation is opt-in and stateless. A before/after database, WAL,
  query-log, and telemetry witness proves no new persistence.
- A privacy fixture uses unique query, source, logical, revision, owner, and
  body markers and scans decoded reference payload/string output for each.
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
3. exact sidecar order/association, explanation composition, graph/text/vector
   contribution, and whole-request rollback on one incomplete hit;
4. wrong database, mismatched/broader/narrower context, supersession,
   lifecycle deletion, erasure, validity, eligibility, closure fence, and
   replacement all producing byte-identical `evidence_unavailable` errors;
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
