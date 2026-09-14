---
title: FathomDB 0.8.26 Slice 20 — exact graph artifact evidence design
status: CHANGES REQUIRED — GPT-6 ASTRA MEDIUM REVIEW
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
grammar; they are not numeric cursors. Internal write cursors never appear in the
public response or resolver request. Each returned graph target necessarily has a
winning terminal edge, so edge fields are required. Identity is useful for joining
and auditing but is never accepted by a resolver.

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

The joined rows provide immutable artifact facts and complete source provenance.
The engine validates revision identity, role, source identity/version/revision,
UTF-8 locator, canonical hash, and lifecycle. Shared source bytes are validated
and hashed once per unique source while each position retains its source
association. Complete provenance does not imply dependency registration.
Canonical-source nodes use self-source provenance and always have no dependency.
Derived nodes and edges may also have no direct dependency; when one is registered,
it must match the artifact/source identities and exact generation/closure state.

During this same preflight, the engine independently authorizes canonical source
bytes. It applies full class-correct eligibility to the selected artifact, then
applies the established source-byte subset to the source: effective view/window,
created-after, status, every attribute equality, lifecycle, supersession, validity,
and closure. Artifact `kind` and `source_type` are not applied to a differently
typed canonical source. A missing or failing source row is nondisclosing and mints
no sidecar. Canonical-source self-rows follow the same rule without double-applying
incompatible kind/source-type predicates.

Only after all selected rows pass does the engine mint the complete positional
sidecar. Any missing row, invalid locator/hash, incomplete provenance, role
mismatch, or coherence failure refuses the whole evidence-bearing graph result.
No partial sidecar is returned. The engine performs final frozen revalidation and
commits the same reader transaction that selected, hydrated, and authorized the
graph result.

## Reference commitment and nondisclosure

`GraphEvidenceRefV1` is a distinct nominal Rust carrier with the same public
opaque-string discipline and 2,048-byte maximum as ranked `EvidenceRefV1`. The
wire value uses a distinct `fdbgev1` prefix, domain-separated key material, and a
distinct parser, so ranked and graph references cannot be interchanged.

The token is prefix plus fresh nonce, confidentiality-protected bounded selector,
and an outer HMAC covering prefix/version, nonce, and ciphertext. Selector
protection reuses the existing domain-separated keyed-stream construction used by
ranked evidence rather than adding a dependency. The authenticated selector is
recoverable only by the owning database and contains no public revision/source
strings. It carries:

- internal artifact role and lookup cursor;
- target and terminal-edge lookup cursors;
- effective instant, graph target position, direction, and disclosure role; and
- keyed commitments to database identity, frozen context/eligibility, canonical
  graph request, target/predecessor identities, edge kind, and both artifact
  revision identities.

The canonical graph-request commitment is an authenticated mint-time disclosure
binding; the resolver does not reconstruct or recompute the original request.
Otherwise identical disclosures minted from different canonical requests must
produce domain-distinct references. On resolution the engine authenticates the outer MAC before decrypting or parsing
the selector, validates fixed lengths/tags/bounds before lookup, loads only the two
bounded cursors, preserves the authenticated request binding, and recomputes the
database, context, artifact, target/predecessor, edge, and revision commitments
from the supplied context and stored facts. Token parsing
never exposes plaintext selectors in public errors. Per-field selector mutation,
nonce/ciphertext/MAC mutation, truncation, extension, wrong prefix, and token-size
overflow all fail nondisclosingly.

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

## Error envelopes and precedence

- Request/version/closed-field failures use `GraphExpansionErrorV1`. Evidence
  requested with a current context is `graph_context_invalid` at `/context`.
- After authenticated frozen selection, incomplete target provenance is
  `EvidenceErrorV1(evidence_incomplete)` at `/targets/{i}/provenance`; terminal-
  edge provenance uses `/targets/{i}/terminalEdge/provenance`.
- After authenticated frozen selection, any missing or ineligible canonical
  source needed for byte disclosure returns
  `EvidenceErrorV1(evidence_unavailable)` at the deliberately non-positional
  `/evidence` path and no sidecar. This nondisclosing result takes precedence
  over per-target source detail and is identical across Rust, Python, and
  TypeScript.
- Authenticated visible corrupt locators use the corresponding
  `.../provenance/sourceLocator` path; canonical hash mismatch uses the
  corresponding `.../canonicalSourceHash` path. The entire expansion fails and
  returns no sidecar.
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

## Point resolution and erasure ordering

Resolution uses one primary-connection transaction for frozen revalidation,
reference authentication, exact revision lookup, eligibility, and materialization.
It holds the primary mutex through the existing before-evidence-return test hook,
performs a second frozen validation immediately before commit, then returns the
materialized value. `erase_source` and `excise_source` therefore cannot report
completion before an already-started resolver either returns its authorized bytes
or loses the ordering race and fails nondisclosingly. A separately held WAL reader
retains the established typed `ErasureIncomplete { stage: "wal_checkpoint" }`
contract after committed deletion.

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
dependency generation, complete-provenance refusal, corruption precedence,
lifecycle/nondisclosure states, restart, erase/excise ordering, zero statements for
empty results, exactly two bounded indexed hydration statements for nonempty
results, and one hash per shared source.

Property tests cover request/result round trips, sidecar coherence, valid
nonnumeric artifact revision IDs, Unicode/length/grammar boundary refusal, closed
artifact variants, canonical encodings for actual numeric fields, strict booleans,
RFC 6901 error paths, and arbitrary per-field/one-byte reference tampering. Python
and TypeScript run against a real engine and compare the same contract. Fresh installed-package probes prevent
source-tree-only success. Slice 15's full performance campaign is not repeated;
the ordinary byte/plan sentinels and bounded work assertions guard its relevant
conclusions.

A private token test makes request binding non-vacuous: it fixes nonce, selected
artifacts, graph disclosure, context, and database key; changing one canonical
request field must change the request commitment and authenticated token, while
normalization-equivalent requests must produce the same request commitment under
that fixed nonce. Public tests never decode or expose token contents.
