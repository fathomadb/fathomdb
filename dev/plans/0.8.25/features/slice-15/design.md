---
title: 0.8.25 Slice 15 — identity, provenance, and wire evolution design
status: DRAFT_SCOPE_RECONCILED_BLOCKED_ON_SLICE_7
design_version: 2
review_fix: 2
depends_on: 10
---

# Slice 15 design

## Authority and predecessor disposition

Implements R25/AC25-15, N25-01/N25-02, Memex needs 1/2 and feature-local 23,
plus A25-02/A25-05. Engine identity and canonical-identity records remain
historical authority; this is their revision-provenance successor.
Provenance-retention is reused. Bindings, wire, and JSON-schema policy are
amended additively below. READY remains blocked on Slice 7/review.

## Identity and provenance contract

Artifact revision and canonical source revision are distinct:

```text
ArtifactRevisionId(string)
SourceRevisionId(string)
SourceVersionId(string)
CanonicalHash { algorithm: sha256, digest_hex }
SourceLocator = WholeBody | Utf8Bytes { start_inclusive, end_exclusive }
SourceProvenanceLinkV1 {
  schema_version: 1, source_id, source_version_id, source_revision_id,
  locator, canonical_source_hash
}
ArtifactProvenanceV1 {
  schema_version: 1, artifact_revision_id,
  completeness: complete | migrated_incomplete,
  source_links: [SourceProvenanceLinkV1]
}
```

Every canonical record has one `ArtifactRevisionId` that is also its
`SourceRevisionId`; its whole-body link spans that canonical revision. Every
caller-derived artifact has its own artifact revision and zero or one link to
a canonical source revision in 0.8.25. The wire/storage field remains a list so
0.8.26 can relax the bound additively for reviewed multi-source provenance;
0.8.25 rejects more than one link. Slice 20 represents the matching core
dependency. New source-backed writes require one complete link.

Canonical bytes are exact stored UTF-8 with no normalization. Whole-body is
`[0, byte_len)`. Ranges are ordered, in bounds, and code-point aligned. SHA-256
covers the complete named canonical source revision. Supersession creates new
artifact/source revisions; reindex copies all IDs and links.

## Closed revision-ID validators

All revision IDs share one database-wide, cross-artifact unique index, are
opaque/non-PII ASCII, and are at most 128 bytes. The stored column accepts
exactly this union:

- **Caller ID:** `[A-Za-z0-9][A-Za-z0-9._:-]{0,127}` and no `_fdb:` prefix.
- **Engine runtime ID:** `_fdb:r:<26-char uppercase Crockford ULID>`.
- **Engine migration ID:** `_fdb:m:<64 lowercase hexadecimal characters>`.

Callers can submit only the caller form. Engine runtime/migration forms are
constructible only internally; any other leading underscore, reserved prefix,
case/length/character mismatch fails `revision_id_invalid`. A duplicate in any
artifact/source class fails `revision_id_conflict`; a source revision must name
a canonical source record. Rust/Python/TypeScript/wire validators share frozen
positive and negative fixtures for all three forms.

## Total legacy migration

Migration is additive, transactional per bounded page, restart-idempotent, and
uses this class matrix:

| Existing class | Migration result |
| --- | --- |
| Canonical row with known `source_id` | Mint stored artifact/source revision; source-scoped version; whole-body locator/hash; `complete`. |
| Caller-derived row with an exact registered canonical link | Mint artifact revision and preserve/migrate the exact source revision link; `complete`. |
| Caller-derived row without an exact source revision link | Mint artifact revision only; `migrated_incomplete`; no fabricated locator/hash. |
| Canonical/legacy row with missing source identity | Mint artifact revision; `migrated_incomplete`; source fields remain absent. |
| Engine-owned projection row | No canonical/source identity minted; projection remains rebuildable and inherits owner revision through projection work. |

Migrated IDs use `_fdb:m:<sha256>` over a domain tag, durable database
identity, table/artifact class, immutable physical-row identity, and existing
source ID or missing marker. Source version also hashes source ID, physical
identity, and content hash, preventing equal-byte cross-source collapse. Stored
mapping values win on restart; collision/input drift fails open as
`legacy_identity_conflict`.

`migrated_incomplete` stays readable under existing rules but cannot produce
source-complete Slice 50 evidence; resolution returns
`legacy_provenance_incomplete`. Known-source rows remain source-erasable;
unknown-source rows retain row/logical purge handles and doctor/open counts
until an external repair creates a new revision. Migration never fabricates
complete provenance.

## Shared 0.8.25 wire/SDK rules

Every new public/persisted object in active Slices 15–60 carries integer
`schema_version` and canonical fixtures. Requests/persisted writes reject
unknown fields/variants/enums, duplicate keys, and unsupported versions.
Responses accept the supported major, ignore additive unknown object fields,
and reject unknown required variants/newer majors. Opaque IDs are strings; u64
offsets are decimal strings in JSON. Typed errors share code/field path across
SDKs. Each owner proves Windows CPU/native and locally packaged parity; Slice
75 only audits. Bare `SearchHit` remains unchanged.

## Persistence, flow, failures, and tests

Persist immutable artifact revision columns plus normalized source-link rows
and canonical-source unique indexes. Prepare bytes/links, validate or mint IDs,
validate ranges/hashes, then commit row and synchronous projections in the
writer transaction. Async work carries artifact revision.

The Rust facade exports the same revision/source/provenance types and codecs as
the Python and TypeScript bindings while preserving the existing `IdSpace`
surface. No SDK may collapse a revision ID into a logical ID or write cursor.

Tests cover the validator union, cross-class uniqueness, namespace/length/
non-PII rules, canonical versus derived identities, zero/one source link and
multi-source rejection,
restart/reindex/supersession, every migration row, equal bytes across sources,
collision/open failure, incomplete evidence, UTF-8/hash tamper, erasure, wire/
u64/error fixtures, three SDKs, Windows, and locally packaged artifacts. Run
fast, heavy, all, all-feature, Windows and packaged-artifact routes;
operator/CUDA/model and pre-publication registry routes are N/A. A formal
independent READY review remains required after Slice 7 and Slice 10 complete.
