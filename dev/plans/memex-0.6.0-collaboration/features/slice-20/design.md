---
title: Memex collaboration Slice 20 — exact graph-target evidence design
status: DRAFT
design_version: 1
date: 2026-09-12
---

# Exact graph-target evidence design

## Decision summary

FathomDB, not Memex, should implement the missing primitive. FathomDB alone
owns canonical artifact-revision identity, frozen authorization, lifecycle,
erasure, dependency closure, and the transaction needed to evaluate them
consistently.

The preferred mechanism has two parts:

1. Graph results expose the immutable artifact revision associated with each
   evidence-capable target and terminal edge.
2. A governed read resolves evidence by that immutable revision under an
   Engine-minted frozen context.

Memex consumes this surface and records its own use/citation receipt. It does
not implement resolution by re-searching a body or reading FathomDB's private
tables.

## Problem

`GraphTargetV1` carries `logical_id`, `kind`, `body`, `write_cursor`, and a
compact graph origin. `read.get` returns the same mutable logical identity and
row cursor shape. Neither provides immutable artifact-revision identity,
source identity, locator, or an evidence reference. The only public exact
resolver accepts an opaque reference minted by `search_with_evidence`, while
`SearchFilter` has no logical-ID predicate.

Searching the target body and comparing logical IDs is not an identity lookup:
ranking, body duplication, projection state, and top-K truncation can all alter
the candidate set. It cannot satisfy an evidence invariant.

## Identity and result evolution

The lookup key is `artifact_revision_id`, never logical ID or write cursor.
Logical ID selects a changing entity; write cursor is an internal ordering
carrier; artifact revision names the immutable revision whose provenance must
be resolved.

The implementation-design step must select one of these compatible response
evolution forms:

- Prefer an additive `artifact_revision_id: Option<String>` on the
  non-exhaustive Rust `GraphTargetV1`, an appended-default Python field, and an
  optional TypeScript field if the wire and construction-compatibility review
  proves this safe.
- Otherwise introduce a successor graph target/result version and leave V1
  byte-for-byte unchanged.

`None` is permitted only for a legacy or incomplete row for which the Engine
cannot prove the immutable mapping. The Engine must never synthesize an ID.
An evidence-requesting graph operation may instead fail atomically with
`evidence_incomplete`; ordinary graph expansion remains available.

The compact origin is also extended or accompanied by a sidecar containing the
terminal edge's immutable artifact revision when that edge has evidence-grade
provenance. Target and terminal-edge identities are separate fields because
they answer different questions.

## Proposed governed read

Conceptually, the new operation is:

```text
resolve_artifact_evidence(
  ArtifactEvidenceLookupV1 {
    schema_version: 1,
    artifact_revision_id,
    context: FrozenReadContextV1,
  }
) -> ResolvedArtifactEvidenceV1
```

`ResolvedArtifactEvidenceV1` contains the intrinsic evidence record:

- logical and immutable artifact-revision identity;
- source, source-version, and source-revision identity;
- whole-body or exact UTF-8-byte locator;
- canonical source bytes, selected evidence text, and verified SHA-256;
- artifact and source lifecycle;
- direct registered dependency, when one exists; and
- artifact class plus any applicable graph target or terminal-edge origin.

It deliberately does not fabricate search ranks, fused scores, or cross-encoder
scores. Those values belong to `search_with_evidence` and its existing
`ResolvedEvidenceV1`, not to an identity lookup. Implementation should share an
internal intrinsic-evidence record between both resolvers instead of copying
authorization logic.

## Read and authorization algorithm

One reader transaction performs the complete operation:

1. Validate the closed request and authenticate the frozen context for this
   database.
2. Resolve the immutable artifact revision by an indexed provenance key.
3. Reapply the frozen view's validity and eligibility to the artifact and its
   canonical source.
4. Recheck supersession, artifact/source lifecycle, erasure, and dependency
   closure at the admitted boundaries carried by the context.
5. Load and validate the canonical provenance mapping, locator, bytes, and
   stored hash; recompute the hash before returning.
6. Return the intrinsic evidence record only if every check succeeds.

Frozen-state drift fails the attempt. The consumer remints a context and
restarts graph expansion and evidence lookup together; it does not resolve an
old target under new authority.

## Failure and disclosure contract

Malformed schema or field shapes receive their typed validation errors before
database access. Once a syntactically valid identity is supplied, unknown,
foreign, unauthorized, invisible, superseded, inactive, erased,
closure-fenced, or context-mismatched identities collapse to one
`evidence_unavailable` refusal at `/artifactRevisionId`. Incomplete or corrupt
provenance is distinguishable only after identity and authorization have been
validated, matching the existing evidence disclosure boundary.

An evidence reference or artifact revision is an identifier, not authority.
Possessing either never weakens the supplied frozen context.

## Target evidence versus relationship evidence

Target-content evidence proves the canonical source mapping for the returned
target revision. Terminal-edge evidence proves the canonical source mapping
for the final traversed relationship revision. Neither proves that all earlier
edges in a multi-hop traversal were valid evidence for a downstream claim.

If Memex later needs full path evidence, FathomDB should design an ordered,
bounded path-evidence carrier that identifies every edge revision and enforces
one-snapshot authorization. That larger contract is outside this slice.

## Alternatives

### Attach existing evidence references to graph targets

This gives strong positional association and reuses `resolve_evidence`, but the
current resolved type requires search-specific projection and ranking
contributions. Inventing neutral scores would be false, while expanding the
token payload creates a second meaning for a search-oriented contract. It can
be reconsidered after intrinsic evidence has been factored cleanly.

### Resolve by frozen logical ID

This is deterministic only for the active revision at the frozen boundary; it
does not name the exact revision already returned and becomes ambiguous around
supersession. It is a convenience lookup, not the evidence primitive.

### Resolve by logical ID plus write cursor

This can detect replacement but promotes an interim row-ordering carrier into
durable public identity. It is rejected in favor of artifact revision.

### Implement the join in Memex

Raw SQLite, a shadow mapping, or body re-search duplicates FathomDB authority
and can race lifecycle, erasure, closure, and projection changes. It is
rejected.

## Compatibility and storage

No schema migration is expected because 0.8.25 already persists immutable
artifact provenance and indexes the existing evidence path. Implementation
must prove the exact query plan; a missing immutable-revision index requires an
explicit additive-index decision, not a scan.

The new operation is additive. Existing graph, search, evidence, and read
methods are unchanged. Response-carrier evolution must be reviewed across Rust
construction rules, Python positional/default behavior, TypeScript structural
typing, and strict wire decoding before choosing additive V1 versus successor
V2.

## Test model

The RED suite must include:

- two graph targets with identical bodies but different artifact revisions;
- an exact target excluded from ordinary search top-K;
- supersession between graph attempts and evidence resolution;
- narrower, broader, tampered, stale, and foreign frozen contexts;
- deleted, erased, closure-fenced, incomplete, and corrupt evidence;
- target and terminal edge with different canonical sources;
- multi-hop traversal proving only terminal-edge, not full-path, evidence;
- close/reopen and equivalent-context resolution; and
- Rust/Python/TypeScript/wire round trips and non-disclosure parity.

Property tests cover codec round trips, context binding, and the invariant that
changing any authenticated identity component cannot resolve another
artifact.
