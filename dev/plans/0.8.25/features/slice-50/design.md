---
title: 0.8.25 Slice 50 — compact source-complete evidence design
status: DRAFT_SCOPE_RECONCILED_BLOCKED_ON_SLICE_7
design_version: 4
target_release: 0.8.25
depends_on: 45
architecture: dev/design/fathomdb-data-plane-architecture-v2.md
---

# Slice 50 design

## Authority and boundary

Implements the retained subset of R25/AC25-50, Memex needs 10/11 and its share
of 12, N25-02, and A25-02/A25-05/A25-07. It adds opt-in evidence identity and
exact resolution without changing the default `SearchHit` shape, allocation,
or storage behavior.

Persisted evidence receipts/leases, multi-source evidence, and replayable
retention policy are allocated after 0.8.25.

## Contract

```text
EvidenceRefV1(string)
EvidenceSidecarEntryV1 {
  schema_version: 1, result_index, artifact_revision_id, evidence_ref
}
EvidenceSearchResultV1 {
  schema_version: 1, search_result: SearchResult, evidence: [EvidenceSidecarEntryV1]
}
ResolvedEvidenceV1 {
  schema_version: 1,
  logical_id, artifact_revision_id,
  source_id, source_version_id, source_revision_id,
  locator, canonical_bytes, canonical_hash,
  effective_valid_at, lifecycle_state,
  projection_origin, retrieval_contributions,
  dependency_id?, supersession_status
}
```

`search_with_evidence(query, options, context)` returns the ordinary search
result plus an equal-length sidecar. Entry `i` has `result_index=i` and names
the exact immutable artifact revision underlying hit `i`. Failure to create any
reference fails the whole opt-in request; a partial sidecar never escapes.

`resolve_evidence(ref, context)` requires the same canonically normalized view
and eligibility envelope; semantically equivalent encodings normalize to the
same digest. A broader, narrower, or otherwise different context is not
accepted in 0.8.25.

## Stateless reference and resolution

The authenticated database-scoped reference contains only format/database
identity, artifact/source revision IDs, locator/hash digest, originating
read-boundary and eligibility digest, projection-generation IDs, retrieval-arm
and numeric rank/contribution data. It contains no query text, source bytes,
owner/scope value, public natural key, or free-form path. No evidence receipt,
lease, or hit row is written.

The Engine stores one random database-scoped signing key in internal metadata
so references remain authentic across reopen; that key is not an evidence
receipt or authorization grant. A reference is at most 4 KiB and records at
most 16 numeric contribution components. Overflow fails the opt-in search
before any sidecar escapes.

Resolution performs this fixed order:

1. authenticate format/database and require equivalent context;
2. recheck current access, lifecycle, erasure, and Slice 30 closure fences;
3. verify the immutable artifact and its single complete Slice 15 source link;
4. verify the originating generation/boundary remains reproducible;
5. load exact canonical bytes, validate UTF-8 locator and whole-source SHA-256,
   and return the structured result.

Steps 1–3 return the same non-disclosing `evidence_unavailable` for invisible,
wrong-database, unauthorized, or malformed handles. After authorization,
typed results may distinguish `evidence_erased`, `evidence_stale`,
`evidence_corrupt`, and `legacy_provenance_incomplete`. A reference identifies
but never authorizes. Erasure makes it permanently unresolvable even if the
opaque string remains.

The single-source bound matches Slice 15/20. An unlinked or
`migrated_incomplete` artifact cannot produce source-complete evidence.
Dependency and retrieval contribution output is bounded and complete for the
0.8.25 record; no truncation is hidden.

## Compatibility and verification

The new method and types are additive and follow Slice 15 wire/SDK rules.
Default search has an explicit byte/allocation/non-write regression test.

RED/GREEN tests cover sidecar order/rollback, UTF-8/hash tampering, equivalent
and mismatched contexts, wrong database, close/reopen, projection replacement,
concurrent supersession/erasure/access revocation, incomplete provenance,
invalid contribution data, and every authorized/non-disclosing outcome. A
privacy fixture scans the reference encoding, database, and WAL to prove the
opt-in call persisted no query, source bytes, or natural IDs.

Run fast, heavy, all/all-feature, Windows Rust/Python/Node, and locally packed
evidence routes. Operator, CUDA, live-model, and pre-publication registry
routes are N/A. A formal independent READY review remains required after Slice
7 and Slice 45 complete.
