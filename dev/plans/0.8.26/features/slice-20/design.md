---
title: FathomDB 0.8.26 Slice 20 — exact graph artifact evidence design
status: APPROVED — GPT-6 ASTRA MEDIUM RE-REVIEW PASS
decision: D26-01 option A at seq-290
---

# Slice 20 design — exact graph artifact evidence

## Contract boundary

`GraphExpandRequestV1` gains an opt-in `include_evidence` boolean. Dynamic wire
and SDK callers may omit it; omission and false are equivalent. Evidence requires
an authenticated `FrozenReadContextV1`. Current-context evidence requests fail at
`/context` rather than silently minting weaker authority.

`GraphExpandResultV1` gains optional `evidence: GraphEvidenceSidecarV1`. The JSON
member is absent unless requested, preserving ordinary response bytes. A requested
zero-target expansion returns a present sidecar with zero entries. The versioned,
closed Rust carriers are:

```text
GraphEvidenceSidecarV1 { schema_version: 1, entries: Vec<GraphEvidenceSidecarEntryV1> }
GraphEvidenceSidecarEntryV1 {
  schema_version: 1,
  target_index: u32,
  target_artifact_revision_id: ArtifactRevisionId,
  target_evidence_ref: GraphEvidenceRefV1,
  terminal_edge_artifact_revision_id: ArtifactRevisionId,
  terminal_edge_evidence_ref: GraphEvidenceRefV1,
}
GraphEvidenceResolveRequestV1 {
  schema_version: 1,
  evidence_ref: GraphEvidenceRefV1,
  context: FrozenReadContextV1,
}
```

Canonical JSON uses `schemaVersion`, `entries`, and these entry members:

- `target_index` / `targetIndex`;
- `target_artifact_revision_id` / `targetArtifactRevisionId`;
- `target_evidence_ref` / `targetEvidenceRef`;
- `terminal_edge_artifact_revision_id` / `terminalEdgeArtifactRevisionId`; and
- `terminal_edge_evidence_ref` / `terminalEdgeEvidenceRef`.

Artifact revision IDs preserve the existing validated `ArtifactRevisionId` string
grammar; they are not numeric cursors. The new sidecar and resolver request never
carry a cursor. The pre-existing `GraphTargetV1.writeCursor` remains in the
ordinary target object to preserve its public byte contract, but it is not an
evidence handle and is never accepted by the resolver. Each returned graph target
necessarily has a winning terminal edge, so edge fields are required. Identity is
useful for joining and auditing but is never accepted by a resolver.

`resolve_graph_evidence` / `resolveGraphEvidence` accepts
`GraphEvidenceResolveRequestV1 { evidence_ref, context }`. This is the companion
point resolver for the selected V1 sidecar, not another graph-expansion API and not
a parallel V2 surface. Reusing ranked `resolve_evidence` would be untruthful because
its result requires ranking contribution and projection origin.

The resolver returns these exact Rust carriers, reusing existing truthful evidence
types:

```text
enum GraphEvidenceArtifactV1 {
  Node { logical_id: String, kind: String, body: String },
  Edge {
    logical_id: Option<String>, kind: String, body: Option<String>,
    from: String, to: String,
  },
}
struct ResolvedGraphEvidenceV1 {
  schema_version: u32,
  artifact_revision_id: ArtifactRevisionId,
  artifact: GraphEvidenceArtifactV1,
  source_id: String,
  source_version_id: String,
  source_revision_id: SourceRevisionId,
  locator: SourceLocator,
  canonical_source_body: String,
  evidence_text: String,
  canonical_source_hash: CanonicalHash,
  effective_valid_at: i64,
  artifact_lifecycle: EvidenceArtifactLifecycleV1,
  source_lifecycle_state: LifecycleState,
  dependency: Option<SourceDependencyV1>,
}
```

The response is additive at its outer object boundary. Canonical JSON maps the
outer fields to `schemaVersion`, `artifactRevisionId`, `artifact`, `sourceId`,
`sourceVersionId`, `sourceRevisionId`, `locator`, `canonicalSourceBody`,
`evidenceText`, `canonicalSourceHash`, `effectiveValidAt`, `artifactLifecycle`,
`sourceLifecycleState`, and `dependency`. `dependency` is always present and is
either the full versioned dependency object or null.

`artifact` is a closed `artifactClass: "node" | "edge"` union. The node object
requires exactly `artifactClass`, `logicalId`, `kind`, and `body`; edge-only keys
are absent. The edge object requires exactly `artifactClass`, `logicalId`, `kind`,
`body`, `from`, and `to`; `logicalId` and `body` are present-null when absent.
`artifactLifecycle` reuses `EvidenceArtifactLifecycleV1` and its canonical object:
`kind`, nullable `state`, `superseded`, and nullable `validAtEffective`; node has a
nonnull state and null validity, edge has null state and nonnull validity. No
response field claims a score, rank, ranking contribution, projection
origin/generation, or full traversal path.

The request is closed to exactly `schemaVersion`, `evidenceRef`, and `context`.
Unknown request fields fail `unknown_field` at their exact RFC 6901 path;
unsupported carrier versions fail `unsupported_schema_version` at
`/schemaVersion`. Dynamic result validators reject unknown union variants,
variant-incoherent members, invalid artifact-revision strings, noncanonical actual
numeric fields, and invalid nullability at the exact nested path.

## Expansion transaction

Graph expansion stays on its existing reader-pool connection and transaction.
The engine retains the winning terminal edge cursor internally through global
candidate selection. After final target ordering/capping, it builds cursor-
deduplicated target and edge vectors while retaining positional reconstruction.
An empty result executes no hydration data statement. A nonempty result executes
exactly two class-specific joined data statements, each bounded by the result
limit (at most 50 cursors).

Each statement begins with a requested `(ordinal, write_cursor)` relation and
uses nullable joins for the artifact registry, canonical artifact, exact
artifact-to-source link, source registry/canonical row, source-version row, and
self-source link. It must retain exactly one classified row per requested ordinal
even when one of those records is absent. An inner join may be used only below a
derived subquery that has already preserved the requested ordinal. This structure
is required to distinguish unavailable authority from incomplete provenance.

The two result sets are handled in two logical phases without executing another
query. Phase 1 considers both classes together and classifies authorization facts
only: selected artifact existence/class/eligibility, exact source-link existence,
linked source artifact and canonical-source existence, source lifecycle and
eligibility, and closure state. It does not decode a locator, compare a disclosed
hash, materialize source bytes, or emit a positional detail error. Phase 2 runs
only if every required artifact and linked source is authorized; it validates
revision identity, provenance role, source identity/version/revision, UTF-8
locator, canonical hash, self-link, and dependency coherence. Shared source bytes
are validated and hashed once per unique source while reconstruction preserves
every target position and association.

All artifact and source eligibility is compiled into the two SQL statements.
The node statement embeds the existing node eligibility grammar for each selected
target. Terminal edges are governed by their committed graph selection and edge
lifecycle; target `SearchFilter` predicates are not reinterpreted as terminal-edge
predicates. Both statements also embed a source-byte eligibility projection that
applies effective validity, created-after, status, each attribute equality,
lifecycle, supersession, and closure to the linked canonical source while omitting
artifact `kind` and `source_type`. Correlated `EXISTS` predicates inside either
statement count as part of that statement. No hydration path may call
`text_hit_passes_filter`, an attribute helper that executes SQL, or any other
query-emitting helper.

The eligibility edge cases deliberately match the shipped search grammar. Missing
`vector_default` metadata is neutral when neither created-after nor status needs
it, and fails when either such predicate requires that row. An absent canonical
attribute fails an equality predicate; a present row whose value is the empty
string satisfies a requested empty value. Tests exercise multiple distinct
sources and nonempty metadata/attribute filters while an execution counter proves
exactly two prepared/executed data statements for nonempty results and zero for
empty results, including every helper-issued statement.

Complete provenance does not imply dependency registration. Canonical-source
nodes use self-source provenance and always have no dependency. Derived nodes and
edges may also have no direct dependency; when one is registered, it must match
the artifact/source identities and exact generation/closure state.

Only after both phases pass does the engine mint the complete positional sidecar.
No partial sidecar is returned. The engine performs final frozen revalidation and
commits the same reader transaction that selected, hydrated, and authorized the
graph result.

### Winning terminal edge

Evidence preserves the edge selected by the existing traversal, rather than
choosing a second edge during hydration. Incident edges are visited in ascending
tuple order: direction rank (`outgoing`, `incoming`, `both`), edge-kind bytes,
next-logical-ID bytes, edge logical-ID bytes with absent ordered as the empty
string, then edge write cursor. Candidate origins retain the existing ascending
order of hop count, seed ordinal, predecessor-ID bytes, direction rank,
edge-kind bytes, and target-ID bytes. For parallel edges with the same origin,
the first incident-edge tuple therefore wins. Hydration carries that exact cursor
forward and the sidecar reports its exact edge revision identity. The ADR and
regression test must pin this rule.

## Reference commitment and nondisclosure

`GraphEvidenceRefV1` is a distinct nominal Rust carrier with the same public
opaque-string discipline and 2,048-byte maximum as ranked `EvidenceRefV1`. The
wire value is exactly `fdbgev1.` followed by 696 lowercase hexadecimal characters
encoding `nonce[16] || ciphertext[300] || mac[32]`, for a total encoded length of
704 bytes. Any other length, case, alphabet, prefix, or decoded framing is
rejected before lookup. The 2,048-byte carrier ceiling remains a generic parser
guard, not permission for an extended V1 form.

The plaintext selector is exactly 300 bytes in network byte order:

```text
u32 selector_schema_version = 1
u8  disclosure_role         // 0 target, 1 terminal edge
u8  artifact_class          // 0 node, 1 edge
u8  traversal_direction     // 0 outgoing, 1 incoming, 2 both
u8  reserved = 0
u32 target_index
u64 artifact_cursor
u64 target_cursor
u64 terminal_edge_cursor
i64 effective_valid_at
[u8;32] database_commitment
[u8;32] frozen_context_commitment
[u8;32] request_commitment
[u8;32] target_logical_id_commitment
[u8;32] predecessor_logical_id_commitment
[u8;32] terminal_edge_kind_commitment
[u8;32] target_revision_id_commitment
[u8;32] terminal_edge_revision_id_commitment
```

All fields are required, so the V1 selector has no optional-value ambiguity. The
role/class combination must be target+node or terminal-edge+edge; reserved bytes,
direction, target index, and cursor coherence are checked before lookup. The
selector contains no plaintext public revision, logical, source, locator, or body
string.

Minting uses a fresh cryptographic 16-byte nonce. Stream block `i` is
`HMAC-SHA256(key, "fathomdb.graph-evidence.stream.v1\0" || nonce ||
u32_be(i))`; blocks start at zero, concatenate until 300 bytes, truncate only the
last block, and XOR length-preservingly with the selector. This explicitly covers
ten blocks and does not inherit the ranked helper's 64-byte mask limit. The MAC is
`HMAC-SHA256(key, "fathomdb.graph-evidence.mac.v1\0" || ASCII("fdbgev1.") ||
nonce || ciphertext)`. It is verified in constant time before decryption.
Commitments use their own literal domains
`fathomdb.graph-evidence.commit.{database,context,request,target,predecessor,
edge-kind,target-revision,edge-revision}.v1\0`; no stream, MAC, ranked-evidence,
or commitment domain is reusable as another. Cross-domain tokens fail
nondisclosingly.

The canonical graph-request commitment is a length-prefixed binary encoding of
the semantic request, excluding the literal frozen-context token and including:
request schema; seed variant; exact query UTF-8 plus ranked limit, or explicit
logical IDs in caller order; direction; edge-kind set; target-kind set; max depth;
result limit; max work units; `includeExplanation`; and `includeEvidence=true`.
Validated edge- and target-kind lists are each sorted by UTF-8 byte order before
encoding because their request semantics are sets. Query text is not trimmed,
case-folded, or otherwise changed, and explicit seed order is preserved because
seed ordinal affects graph disclosure. Two freshly minted frozen contexts with
the same authenticated database, view, eligibility, and effective instant have
the same frozen-context commitment and request commitment even though their
literal context tokens differ; fresh evidence nonces still make their references
different.

The resolver does not reconstruct the original request. It authenticates the MAC
before decrypting or parsing the selector, validates the fixed framing and tags,
loads only the bounded cursors, preserves the authenticated request commitment,
and recomputes the database, semantic frozen-context, artifact, graph-origin, and
revision commitments from the supplied context and stored facts. Token parsing
never exposes plaintext selectors in public errors. Tests cover all 300 plaintext
bytes, stream blocks beyond byte 64, fixed-nonce semantic normalization,
single-field mutation, nonce/ciphertext/MAC mutation, truncation, extension,
wrong prefix, and cross-domain use.

The resolver authenticates token structure, database, and frozen authority before
looking up artifact material. Malformed, tampered, foreign, mismatched, drifted,
out-of-window, ineligible, revoked, superseded, erased, closure-fenced, and absent
states collapse to the existing nondisclosing `evidence_unavailable` family at
`/evidenceRef`. Corruption detail is exposed only after authenticated visibility
permits it, matching ranked-evidence precedence.

Target resolution rechecks frozen target eligibility, including target-oriented
source type, kind, status, time, and attributes. Terminal-edge resolution rechecks
edge lifecycle/source visibility and the committed graph edge selection; it does
not reinterpret target `kind` or other target-only filters as edge predicates.
For both artifact classes, resolution independently reapplies the canonical-source
byte subset described above before returning source body or evidence text.

## Temporal validity contract

Evidence-bearing graph expansion rejects an otherwise authenticated frozen
context whose `context.view.includeOutOfWindow` is true with
`graph_context_invalid` at `/context/context/view/includeOutOfWindow`. The check
occurs after frozen-context authentication, so it cannot become an oracle for a
malformed or foreign context. Ordinary graph expansion without evidence retains
its existing relaxed-window behavior.

For accepted evidence requests the effective instant is frozen and every required
class must be actually valid at that instant:

| Class | Valid at effective instant | Invalid cases |
| --- | --- | --- |
| target node | `valid_from IS NULL OR valid_from <= effective`, and `valid_until IS NULL OR effective < valid_until` | future start; effective equal to or after end |
| terminal edge | `t_valid IS NULL OR t_valid <= effective`, and `t_invalid IS NULL OR effective < t_invalid` | future start; effective equal to or after end |
| canonical source node | same start-inclusive/end-exclusive node rule as target | future start; effective equal to or after end |

Lifecycle, supersession, status, closure, and attribute predicates remain
independent additional gates. Traversal's incident-edge recency/order is a graph
selection rule, not evidence authorization; hydration and resolution recheck the
exact winning edge against the table above. Consequently an edge lifecycle result
may report `validAtEffective=true` only after actual window validation, never
because a relaxed visibility flag converted invalidity into truth. Tests cover
future starts, ended windows, equality at each start and end, and authenticated
relaxed-window refusal for target, winning edge, and source cases.

## Error envelopes and precedence

- Request/version/closed-field failures use `GraphExpansionErrorV1`. Evidence
  requested with a current context is `graph_context_invalid` at `/context`.
  An authenticated relaxed-window evidence request is `graph_context_invalid`
  at `/context/context/view/includeOutOfWindow`.
- Expansion applies one global precedence across both hydration result sets
  before emitting any row-specific detail. First, an absent/ineligible selected
  artifact, or an existing artifact-source link whose exact source artifact or
  canonical source is absent/ineligible, is
  `EvidenceErrorV1(evidence_unavailable)` at the deliberately non-positional
  `/evidence` path. This wins even if another row has malformed locator/hash or
  incomplete provenance.
- Second, after every linked source is authorized, a selected artifact with no
  exact artifact-to-source link is `evidence_incomplete` at
  `/targets/{i}/provenance`; terminal-edge provenance uses
  `/targets/{i}/terminalEdge/provenance`. A missing source-link is therefore an
  incomplete stored provenance fact, while a link naming a missing source is an
  unavailable authority fact. If several detail errors share this precedence,
  the lowest target index wins and target precedes terminal edge at that index.
- Third, after authority and provenance completeness pass globally, corrupt
  visible locators use the corresponding `.../provenance/sourceLocator` path;
  canonical hash mismatch uses the corresponding `.../canonicalSourceHash`
  path. The same target-index/target-before-edge ordering selects among equal-
  precedence corruptions. The entire expansion fails and returns no sidecar.
- Resolver request unknown fields and unsupported schema versions use the evidence
  error family at the exact request path. Malformed, oversized, wrong-domain,
  tampered, foreign, context-mismatched, unauthorized, or no-longer-visible graph
  references are `evidence_unavailable` at `/evidenceRef`.
- Only after token/database/context authentication and current visibility may a
  genuinely stored incomplete/corrupt artifact return `evidence_incomplete` at
  `/provenance` or `evidence_corrupt` at `/provenance/sourceLocator` or
  `/canonicalSourceHash`. Authentication/nondisclosure wins over corruption detail.
- Canonical response decoders report `graph_corrupt` at `/evidence`,
  `/evidence/entries/{i}`, or the exact incoherent member path. Python and
  TypeScript map the same reason and RFC 6901 path without reclassification.

Regression fixtures pair denied source authority with malformed locator/hash,
missing linked source with missing source-link, and mixed target/edge faults. They
assert that neither returned reason/path nor logs/traces parse or identify an
unauthorized source before the global authority phase completes.

Those expansion fixtures establish their stored incomplete/corrupt state before
minting the frozen context, then expand under the authenticated snapshot of that
state. A mutation after frozen-context mint is state drift and the existing
`FrozenReadErrorV1(state_drifted)` wins before hydration; the implementation must
not defer or bypass snapshot validation to expose a more specific evidence error.
Likewise, storage drift after an evidence reference is minted fails frozen
authority before resolver provenance detail. The mixed-fault precedence above
governs faults already present in the authenticated snapshot, not post-mint
mutation.

## Point resolution and erasure ordering

Resolution uses one primary-connection transaction for frozen revalidation,
reference authentication, exact revision lookup, eligibility, and materialization.
It holds the primary mutex through the existing before-evidence-return test hook,
copies the response-owned bytes, performs a second frozen validation immediately
before commit, and commits while still holding that mutex. Successful final
validation and transaction commit are the resolver's linearization point.

If `erase_source` or `excise_source` commits first, the later resolver sees the
erased/invalid authority and fails nondisclosingly. If resolution commits first,
its already copied bytes remain a valid result of that earlier authorized read;
erasure does not revoke memory already returned from a completed database read.
Native/Python/TypeScript conversion and caller observation may occur after the
mutex is released and even after erasure reports completion. Tests synchronize at
the transaction boundary and assert which database operation linearized first;
they do not assert cross-thread or cross-SDK delivery order. A separately held WAL
reader retains the established typed
`ErasureIncomplete { stage: "wal_checkpoint" }` contract after committed deletion.

## Code and public-document alignment

Production work is intentionally narrow:

- `fathomdb-engine` graph carriers/codecs/selection, evidence token/materializer,
  engine dispatch/public resolver, and focused property/integration tests;
- facade reexports plus Python and N-API bridges;
- Python and TypeScript request/result/sidecar/intrinsic carriers, strict codecs,
  declarations, resolver methods, and real-engine tests;
- one accepted graph-evidence ADR, all four interface contracts, public frozen-
  evidence/API documentation, changelog, and exact governed-surface allowlist/pin.

No schema or migration is required. No durable evidence table/cache, batch
resolver, raw-ID lookup, logical-ID predicate, full path, compatibility layer, or
parallel graph API is introduced.

## Verification model

RED/GREEN tests prove literal ordinary JSON stability, frozen-only opt-in, empty
and positional sidecars, winning parallel-edge identity, canonical/derived node
and edge evidence, derived evidence with and without dependency, exact registered
dependency generation, complete-provenance refusal, global authorization-before-
corruption precedence, lifecycle/nondisclosure states, actual temporal truth at
start/end boundaries, relaxed-window refusal, restart, erase/excise transaction
ordering, zero statements for empty results, exactly two bounded indexed hydration
statements for nonempty filtered multi-source results, no helper queries, and one
hash per shared source.

Property tests cover request/result round trips, sidecar coherence, valid
nonnumeric artifact revision IDs, Unicode/length/grammar boundary refusal, closed
artifact variants, canonical encodings for actual numeric fields, strict booleans,
RFC 6901 error paths, arbitrary per-field/one-byte reference tampering, the full
300-byte selector, stream blocks after byte 64, domain separation, and fixed-nonce
request normalization. Python and TypeScript run against a real engine and compare
the same contract. Fresh installed-package probes prevent source-tree-only success.
Slice 15's full performance campaign is not repeated;
the ordinary byte/plan sentinels and bounded work assertions guard its relevant
conclusions.

A private token test makes request binding non-vacuous: it fixes nonce, selected
artifacts, graph disclosure, context, and database key; changing one canonical
request field must change the request commitment and authenticated token, while
reordered validated kind sets and semantically equivalent newly minted frozen
contexts produce the same respective request/context commitments under that fixed
nonce. Reordered explicit seeds must change the request commitment. Public tests
never decode or expose token contents.
