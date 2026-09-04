---
title: 0.8.25 Slice 15 — identity, provenance, and wire evolution design
status: READY_REVIEW_PASS_CYCLE_3
design_version: 5
review_fix: 3
depends_on: 10
architecture: dev/design/fathomdb-data-plane-architecture-v2.md
---

# Slice 15 design

## Requirements and acceptance

This slice implements R25/AC25-15, N25-01/N25-02, Memex needs 1/2 and the
feature-local portion of 23, plus A25-02/A25-05. It preserves `IdSpace`, the
logical-ID-alone currentness rule, mandatory `SourceId`, existing read
visibility, and the compact `SearchHit` shape.

Acceptance requires:

1. every post-Slice-15 node or edge write has an immutable artifact revision;
2. every complete source-backed write names a caller source version and exact
   canonical source revision;
3. revision identity survives restart, supersession, and projection rebuild;
4. invalid IDs, versions, locators, hashes, or references fail atomically with
   the same typed reason and field path in each SDK;
5. legacy rows and writes remain usable but never claim complete provenance;
6. Rust, Python, TypeScript, canonical fixtures, and Windows CPU/native agree;
   and
7. migration rewrites neither canonical content nor legacy logical identity.

## What exists and what changes

Today, inline `PreparedWrite::Node`/`Edge` variants require `SourceId`, and
`WriteReceipt` returns positional cursors. Canonical tables have no immutable
revision, source version, locator, or hash. `source_id` is an erasure handle,
not proof that source bytes are stored. TypeScript may serialize an object body
before the Engine sees it. Schema migrations apply one transaction per step.

Slice 15 adds versioned per-entity write variants and one additive schema step.
Existing variants and binding envelopes remain source compatible: they receive
internal immutable revisions but stay `migrated_incomplete` because they do not
name a caller source version. Existing `WriteReceipt`, `NodeRecord`,
`ExtractDocument`, and `SearchHit` remain unchanged. The slice adds no verb,
does not backfill rows, and does not add multi-source provenance.

## Public types and prepared writes

Rust adds closed newtypes `ArtifactRevisionId`, `SourceRevisionId`, and
`SourceVersionId`, plus `CanonicalHash`, `SourceLocator`,
`ProvenanceCompleteness`, `WriteProvenanceV1`, `ProvenancedNodeV1`, and
`ProvenancedEdgeV1`. Following the accepted prepared-write ADR, the enum gains
per-entity newtype variants:

```text
PreparedWrite::ProvenancedNode(ProvenancedNodeV1)
PreparedWrite::ProvenancedEdge(ProvenancedEdgeV1)
```

Each newtype contains the same entity fields as its legacy counterpart plus a
required `provenance: WriteProvenanceV1`. Existing `Node`/`Edge` construction
continues to work, mints an internal runtime artifact revision, and stores no
source link. A complete versioned write requires a caller-supplied artifact
revision ID, so its caller already knows the durable identity.

`WriteProvenanceV1` has private fields and two constructors:

```text
WriteProvenanceV1::canonical(artifact_revision_id,
                             source_version_id)
WriteProvenanceV1::derived(artifact_revision_id,
                           source_version_id,
                           source_revision_id,
                           locator,
                           canonical_source_hash)
```

Both carry `schema_version = 1`. Canonical sources are node-only:
`ProvenancedNodeV1` accepts canonical or derived provenance;
`ProvenancedEdgeV1` requires derived provenance. A canonical node's source
revision equals its artifact revision, its locator is `WholeBody`, and the
Engine computes its hash. A derived artifact references one already-stored
canonical node revision. The Engine verifies source ID/version, locator, and
hash before mutation. Its `source_id` must equal the canonical source's.

Python and TypeScript keep accepting the old node/edge shape as incomplete. A
`provenance` member selects the versioned shape. Python uses snake-case member
names; TypeScript uses camel case:

```json
{
  "schemaVersion": 1,
  "role": "canonical",
  "artifactRevisionId": "record-revision-1",
  "sourceVersionId": "source-v1"
}
```

A derived object additionally requires `sourceRevisionId`, `sourceLocator`,
and `canonicalSourceHash`. `role` is `canonical | derived` for nodes and only
`derived` for edges. `artifactRevisionId` is required; omission and `null`
reject.
The versioned object is closed: unknown fields, wrong casing, missing role
fields, wrong types, unknown roles, and unsupported versions reject before
mutation. Outer legacy aliases are unchanged.

`WriteReceipt` and `NodeRecord` remain byte-for-byte public-shape compatible.
Complete-write callers supplied the revision ID, so the receipt need not echo
it. Internal legacy revisions become externally resolvable only through the
opt-in Slice 50 evidence contract; no default result is bloated.

`ExtractDocument` and `ingest_with_extractor` remain unchanged. Their generated
entity/edge rows use legacy variants, receive internal runtime revisions, and
remain `migrated_incomplete`: `source_doc_id` is an erasure handle but does not
prove the document bytes are stored. A future versioned provider input may
produce complete links; Slice 15 does not fabricate them.

## Identity grammar and ownership

The stored registry accepts exactly:

- caller ID: `[A-Za-z0-9][A-Za-z0-9._:-]{0,127}`, excluding `_fdb:`;
- runtime ID: `_fdb:r:<64 lowercase hexadecimal characters>`; and
- legacy ID: `_fdb:m:<64 lowercase hexadecimal characters>`.

Complete-write callers must submit caller IDs. Runtime IDs hash a
length-prefixed tuple of
domain tag, artifact class, reserved write cursor, source ID, optional source
version, and artifact body. Edge bodies use distinct `body:none` and
`body:some` tags before the latter's byte length and bytes; null and empty
cannot collide. Legacy IDs hash a different domain tag, class, physical write
cursor, source-ID-or-missing marker, and body. Scope is one database. Stored
values win; a different owner with the same value is `revision_id_conflict`.
The grammar does not detect PII: non-PII is a caller obligation, and telemetry
does not emit raw revision/source-version values.

`SourceVersionId` uses caller-ID grammar. `(source_id, source_version_id)` maps
to exactly one canonical source revision; reuse for a different revision is
`source_version_conflict`. Derived artifacts repeat the pair only by linking to
that revision. Artifact/source revision are distinct roles. A canonical owner
is addressable under both roles through one registry row; this is an alias, not
a duplicate registration.

## Persistence, integrity, and legacy rows

Schema step 27 atomically creates:

```text
_fathomdb_artifact_revisions(
  schema_version INTEGER NOT NULL CHECK(schema_version=1),
  revision_id TEXT PRIMARY KEY,
  artifact_class TEXT NOT NULL CHECK(artifact_class IN ('node','edge')),
  write_cursor INTEGER NOT NULL,
  artifact_role TEXT NOT NULL
    CHECK(artifact_role IN ('canonical_source','derived_semantic','legacy')),
  completeness TEXT NOT NULL
    CHECK(completeness IN ('complete','migrated_incomplete')),
  UNIQUE(artifact_class, write_cursor))

_fathomdb_source_versions(
  schema_version INTEGER NOT NULL CHECK(schema_version=1),
  source_id TEXT NOT NULL, source_version_id TEXT NOT NULL,
  source_revision_id TEXT NOT NULL UNIQUE,
  PRIMARY KEY(source_id, source_version_id))

_fathomdb_source_links(
  schema_version INTEGER NOT NULL CHECK(schema_version=1),
  artifact_revision_id TEXT PRIMARY KEY,
  source_id TEXT NOT NULL, source_version_id TEXT NOT NULL,
  source_revision_id TEXT NOT NULL,
  locator_kind TEXT NOT NULL CHECK(locator_kind IN ('whole_body','utf8_bytes')),
  start_byte INTEGER, end_byte INTEGER,
  hash_algorithm TEXT NOT NULL CHECK(hash_algorithm='sha256'),
  hash_digest TEXT NOT NULL)
```

The migration carries the accretion exemption, performs no backfill or
`INSERT ... SELECT`, and adds no partial post-open state. Existing rows have no
registry entry. Identity lookup derives their stable `_fdb:m:` ID and reports
`migrated_incomplete`; it never invents source version, locator, or hash.
`_legacy:pre-0.8.20` is only an erasure handle.

This slice deliberately does not enable SQLite foreign keys on every existing
connection. Integrity is Engine-enforced. A complete canonical transaction
inserts the row, revision owner, source-version mapping, and self source link.
A derived transaction validates the existing canonical owner/version/hash,
then inserts row, owner, and link. Raw `CHECK` constraints reject bad persisted
versions/enums. Any error rolls back the batch.

Supersession creates a new revision and leaves prior revision/link immutable.
Projection rebuild carries the owner cursor and never remints. Purge first
collects every target-node version and every touching edge cursor that the
existing purge removes. It rejects a canonical revision with a surviving
derived link outside that affected set; otherwise it deletes all affected
edge/node links, canonical source-version rows when applicable, revision
owners, projections, edges, then nodes in one transaction. The raw orphan
canary covers the complete affected set. Source erasure collects the whole
source bucket, then deletes links, versions, owners, projections, and canonical
rows in that order. A raw orphan canary executes before commit.
Slice 20/30 later replaces the temporary `provenance_in_use` purge refusal with
dependency-aware closure.

## Canonical bytes, locators, and hashes

Versioned node bodies and non-null edge bodies are strings in every SDK;
TypeScript does not serialize objects on this path. A derived edge may have no
artifact body, distinct from empty string in the revision digest. Canonical
bytes are the exact stored UTF-8 bytes after binding validation. There is no
Unicode, JSON, whitespace, or newline normalization. Cross-SDK fixtures use
string bodies.

`WholeBody` resolves to `[0, byte_len)`. `Utf8Bytes` has inclusive start and
exclusive end. Bounds fit signed 64-bit SQLite integers, are ordered and in
bounds, and fall on UTF-8 code-point boundaries. Exact wire objects are:

```text
Python:     {"kind": "whole_body"}
TypeScript: {"kind": "whole_body"}
Python:     {"kind": "utf8_bytes", "start_inclusive": "0",
             "end_exclusive": "5"}
TypeScript: {"kind": "utf8_bytes", "startInclusive": "0",
             "endExclusive": "5"}
```

Offsets are canonical decimal strings in both bindings. Whole-body rejects
offset members; byte ranges require exactly both. Null, numeric, boolean,
floating-point, signed, zero-padded, and overflow encodings reject.

`CanonicalHash` is `{algorithm: "sha256", digestHex: <64 lowercase hex>}` in
TypeScript and snake case in Python. SHA-256 covers the entire canonical source
revision, not only its span. The Engine hashes the exact stored source bytes.
Uppercase, wrong length, unknown algorithm, or mismatch rejects.

## Typed failures and wire evolution

Rust adds `ProvenanceError { reason, field_path }` and wraps it as
`EngineError::Provenance(ProvenanceError)`, per the accepted error taxonomy.
`ProvenanceErrorReason` serializes to this closed lower-snake-case set:

```text
revision_id_invalid | revision_id_conflict | source_version_invalid |
source_version_conflict | source_revision_missing | source_mismatch |
locator_invalid | hash_invalid | hash_mismatch | unsupported_schema_version |
unknown_field | role_invalid | provenance_in_use
```

`field_path` is an RFC 6901 JSON Pointer over canonical camel-case wire names,
starting at `/provenance`; empty is reserved for cross-owner conflicts. Python
raises `ProvenanceError`; TypeScript uses code `FDB_PROVENANCE`. Both expose
the same reason and pointer (`field_path`/`fieldPath`). Errors never include
stored bodies or raw stored IDs. A whole batch rolls back on any error.
Unknown or illegal roles map to `role_invalid` at `/provenance/role`.

The checked-in v1 fixture records snake/camel success and failure objects,
locators, omission rules, reasons, and paths. Requests reject unknown fields,
roles, and versions. V1 responses ignore additive unknown object fields but
reject unknown required variants and newer versions. Persisted unknown values
fail closed. Later slices reuse this rule only for their new objects; existing
pre-0.8.25 envelopes are not silently made strict.

## TDD and verification

RED tests precede implementation and cover:

- caller/runtime/legacy grammar, collision, source-version scope, and
  property-based round trips;
- legacy and versioned writes, unchanged receipts/results, canonical/derived role
  matrix, source validation, supersession, restart, and projection rebuild;
- old rows without backfill or false completeness;
- Unicode boundaries, signed-64 offsets, exact hashes, null/empty distinction,
  tamper, and atomic rollback;
- erasure, referenced-source purge refusal, raw orphans, and invalid persisted
  versions/enums;
- strict v1 wire objects, typed reason/path parity, and string-only bodies; and
- schema step 27, migration idempotency, facade exports, Windows, and isolated
  locally packaged Rust/Python/npm/CLI behavior.

Close routes are `scripts/agent-verify.sh --tier=fast`,
`scripts/agent-verify.sh --tier=heavy`, focused engine/schema/property tests,
facade compile tests, Python/TypeScript parity, applicable all-feature tests,
Windows CPU/native jobs, and packaged smokes. Operator is selected only for
erasure/rebuild tests. CUDA, live-model, and pre-publication registry-installed
routes are N/A.

Stop on mutable identity, fabricated legacy completeness, ambiguous bytes,
partial commit, dangling complete links, or incompatible version behavior. An
independent design PASS is required before RED implementation begins.
