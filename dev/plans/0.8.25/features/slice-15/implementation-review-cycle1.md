---
title: 0.8.25 Slice 15 implementation review — cycle 1
status: COMPLETE
review_cycle: 1
reviewed_on: 2026-09-03
reviewed_commit: 18f029c96cc55cb85cb93f405d1e5529368e7d5a
verdict: FAIL
---

# Slice 15 independent implementation review — cycle 1

## Verdict

**FAIL.** One P1 and two P2 findings remain.

## Findings

| ID | Severity | Finding | Required correction |
| --- | --- | --- | --- |
| I15-06 | P1 | `erase_artifact_identity_for_cursors` discovers dependent source links through an inner join to the artifact registry. A raw, `CHECK`-valid `_fathomdb_source_links` row can therefore reference an erased canonical `source_revision_id` while lacking an artifact owner; it is invisible to dependency discovery and deletion, and the current canary only checks known artifact revision IDs. Purge or source erasure can report success while that link survives. | Add RED raw-corruption cases for purge and source erasure. Account for links from both the source and artifact sides, then either fail closed with rollback or erase the complete source bucket as required by design v5. |
| I15-07 | P2 | TypeScript's `validateWriteFfiTree` recognizes only top-level `provenance`. Wrapped node/edge forms still map NUL-bearing provenance identifiers to `WriteValidationError`; lone-surrogate provenance identifiers are also preempted instead of producing field-specific `ProvenanceError`. | Add direct and wrapped node/edge parity tests for embedded NULs and lone surrogates. Make write validation provenance-aware at every accepted write shape while preserving the closed reason and pointer mapping. |
| I15-08 | P2 | The design-v5 acceptance matrix remains incomplete. | Add raw-corruption, `source_revision_missing`, `source_mismatch`, `hash_mismatch`, locator encoding, signed-64 and Unicode-boundary, pre-step-27 legacy identity lookup, complete role, string-only-body, and shared binding parity coverage where applicable. |

## Confirmed corrections

Cycle 1 confirmed that FIX-1 made vector-kind enrolment atomic with rejected
provenance writes, established role-error precedence, escaped dynamic RFC 6901
tokens, consumed the shared fixture in both SDKs, and added Unix and Windows
packaged provenance smokes.

## Legacy lookup scope interpretation

Slice 15 proves that opening a pre-step-27 database performs no identity
backfill and that the internal `_fdb:m:` derivation is deterministic and
tuple-sensitive. It does not add a public or test-only lookup verb: the reviewed
design keeps `NodeRecord` and default hits unchanged and assigns external legacy
identity resolution to the opt-in Slice 50 evidence contract.
