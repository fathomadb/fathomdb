---
title: 0.8.25 Slice 50 — source-complete evidence design
status: REVIEWED_MAX_ENVELOPE_SCOPE_NARROWED
design_version: 3
target_release: 0.8.25
depends_on: 45
readiness_blocked_on: Slice 7 architecture activation
architecture: dev/design/fathomdb-data-plane-architecture-v2.md
---

# Slice 50 — source-complete evidence design

> **0.8.25 implementation boundary:** Retain a compact opt-in evidence
> identity and eligibility-bound resolver without default-hit growth.
> Persisted evidence leases and replayable receipt retention are deferred by
> the [scope adjustment](../../scope-adjustment-2026-09-02.md). Reconcile this
> maximum-envelope design before READY review.

## Authority and comparison

Owns R25/AC25-50, Memex needs 10/11 and its share of 12, N25-02, and
A25-02/A25-05/A25-07. It adds opt-in evidence without growing default
`SearchHit` or performing semantic answer verification.

| Need | Existing | Decision |
| --- | --- | --- |
| Complete evidence | Compact hit fields | Versioned keyed sidecar plus resolver. |
| Exact locator | No universal source revision locator | Slice 15 source revision + UTF-8 range + SHA-256. |
| Safe replay | No stale-handle contract | Exact originating frozen context and current-fence recheck. |
| Contribution durability | Explain/telemetry is not a retained evidence receipt | Engine-owned privacy-minimal receipt commits before refs escape. |

Identity/provenance, `ReadView`, erasure, `SearchHit`, and accepted explanation
designs are reused. This is a new architecture-v2 design; multi-source identity
comes from Slice 20.

## Public and wire contract

```text
EvidenceRefV1(string)
EvidenceSearchRequestV1 { schema_version: 1, query/options,
                          context: ReadContextV1 }
EvidenceSidecarEntryV1 { schema_version: 1, result_index,
  hit_id: IdSpace, artifact_revision_id, evidence_ref }
EvidenceSidecarV1 { schema_version: 1, retrieval_event_id, entries }
EvidenceSearchResultV1 { schema_version: 1,
  search_result: SearchResult, evidence: EvidenceSidecarV1 }
ResolvedEvidenceV1 { schema_version: 1, logical_id, artifact_revision_id,
  source_id, source_version_id, source_revision_id, locator, canonical_bytes,
  canonical_hash, effective_valid_as_of, lifecycle_state, projection_origin,
  retrieval_contributions, dependencies, supersession_status,
  dependencies_complete: true, contributions_complete: true }
```

Evidence uses a separate opt-in method/response. Default `SearchResult` and
`SearchHit` remain field-, byte-, and allocation-identical. Entries are a
strict equal-length vector: entry `i` has `result_index=i`, the public hit ID,
and artifact revision for result `i`. Any missing/mismatched/uncreatable handle
fails the whole request `EvidenceUnavailable`; no partial sidecar escapes. The
public hit ID is not copied to persistent evidence-receipt storage.

`resolve_evidence(ref, context)` requires the exact originating canonical
`ReadContextV1`; subset contexts are unsupported in v1. Opaque refs carry an
internal version. All objects inherit Slice 15 wire/unknown/u64/error/SDK/
Windows/registry rules and canonical fixtures.

## Evidence receipt and reference persistence

Add `_fathomdb_evidence_receipts` and `_fathomdb_evidence_receipt_hits`.
`EvidenceReceiptV1` stores only schema version, Engine-random retrieval-event
ID, Engine snapshot ID, eligibility SHA-256, Engine projection-generation IDs,
created/expiry, result count, and canonical request-**shape** digest. It stores
no query or request values.

Each hit row stores exactly:

- retrieval-event ID and numeric result index;
- Slice 15 opaque non-PII `ArtifactRevisionId`;
- zero or more Slice 15 opaque non-PII `SourceRevisionId` values;
- zero or more Slice 20 opaque `dependency_set_revision_id` values;
- Engine-generated projection-generation/event IDs;
- closed retrieval-arm enum, numeric rank, bounded numeric contributions, and
  closed fallback/degradation codes.

Receipt tables do **not** store free-form `source_id`, `SourceVersionId`,
logical/natural IDs, public `hit_id`, projection names, path/session/owner
identifiers, payload-derived IDs, query/predicate values, source bytes, or body.
Resolution reaches current governed provenance by opaque artifact/source
revision ID only after authorization succeeds. Maximum results are 1,000,
source revisions and dependency set revisions are each capped at 256 per hit,
and contribution components at 64; overflow fails before creation.

After reader search, the Engine validates all handles and writes receipt/hit
rows atomically in one writer transaction. References are created only after
commit. Crash/failure before commit returns no sidecar; committed rows survive
restart. Receipt expiry equals snapshot expiry and cannot renew. Slice 35
bounded pruning removes expired rows. Erasure locates receipts through opaque
source revision indexes, deletes every affected receipt atomically, invalidates
leases, and retains only authorized non-content erasure audit.

The HMAC reference contains version/database, opaque artifact/source revision
digests, locator digest, snapshot/eligibility/generation, and event ID; no
private bytes or free-form identity are embedded.

## Resolution and non-disclosure precedence

Resolution performs a fixed bounded sequence:

1. authenticate/version/database/expiry;
2. require exact context and recheck current access/lifecycle visibility;
3. verify identity authorization from current governed provenance;
4. only then distinguish erased, stale generation, missing receipt, corrupt
   revision/hash, or other integrity state;
5. load exact source bytes, validate locator/hash, and return complete data.

Steps 1–3 collapse to identical `EvidenceUnavailable` and fixed indexed probes.
Specific authorized outcomes are `EvidenceErased`, `EvidenceStale`,
`EvidenceCorrupt`, and `EvidenceReceiptUnavailable`. No error returns identity
or locator. Dependencies are complete up to 256 and contributions up to 64;
overflow refuses. There is no partial evidence or continuation in v1.

## Invariants, tests, and verification

A reference identifies but never authorizes. UTF-8 ranges bind source revision,
not derived artifact revision. Erasure prevents resolution and scrubs receipts.
Multi-source evidence is complete or fails. Default search does no evidence
allocation/write/retention.

Tests cover sidecar association/rollback, atomic receipt/crash/restart,
expiry/pruning, exact context, UTF-8/hash, all authorized and indistinguishable
unauthorized outcomes, caps, concurrent supersession/erasure, and default-cost
pins. A privacy fixture uses path-like source IDs and session/owner/logical IDs,
then scans receipt tables/WAL to prove none appear; only opaque revision/set/
generation/event IDs may remain. Rust/Python/TypeScript/wire, Windows
CPU/native, and registry evidence smokes pass.

Routes: fast/heavy/all/all-feature, Windows Rust/Python/Node, registry evidence.
Operator/CUDA/live-model are N/A. Status remains `REVIEWED_BLOCKED_ON_SLICE_7`, blocked on
Slice 7 and 15/20/35/40/45; no implementation-shaping decision remains.
