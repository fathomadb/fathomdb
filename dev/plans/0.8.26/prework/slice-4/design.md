---
title: FathomDB 0.8.26 Slice 4 — initial architecture alignment
status: DRAFT
---

# Slice 4 design — initial architecture alignment

## Initial as-built findings to verify

| Area | Alignment finding | Proposed direction |
| --- | --- | --- |
| frozen explanations | Ordinary search finalizes correlation identity; frozen paths can return before equivalent finalization. | Repair the shared completion boundary; do not substitute non-frozen search or alter ranking. |
| evidence substrate | Exact source resolution and edge artifact revision identity already exist internally, while the public resolved shape includes ranked-search contribution fields. | Factor intrinsic artifact evidence from ranked contribution; add exact frozen artifact resolution without invented ranks. |
| graph results | Graph targets expose logical/kind/body/cursor/origin but insufficient immutable evidence identity for exact follow-up. | Expose target and terminal-edge revision identity separately and bind resolution to frozen eligibility. |
| integrity | Engine and CLI already implement bounded `data-plane-integrity`; distribution and consumer contract are incomplete. | Package the existing read-only operator route before considering any in-process SDK method. |
| edge writes | `ProvenancedEdgeV1` and the ordinary prepared-write/projection path exist; the actuation grammar is a closed four-operation V1 set. | Prefer `ActuationBatchV2` with `PutDerivedEdge`, reusing canonical edge storage. |
| replay/receipts | Actuation digest, replay identity, and compact receipt are persisted contract-sensitive state. | Use a V2 digest domain and add the smallest truthful receipt delta; preserve V1 bytes and replay. |

## Architectural invariants

- One reader transaction owns frozen authorization and evidence materialization.
- One writer transaction owns all batch validation, dependency registration,
  edge/node writes, receipt creation, and projection scheduling.
- Point evidence lookup is not search and must not synthesize ranking facts.
- Logical ID is not immutable revision identity.
- A terminal edge proves only that edge, not the traversed path.
- Operator diagnostics remain read-only; recovery remains separately
  authorized and absent from governed SDKs.
- Endpoint validation considers the prospective batch state before any write.
- V1 request encoding, digest, replay, and receipt remain unchanged.
- FathomDB stores caller-declared provenance and relation semantics but does
  not decide supports/refutes/corrects policy for Memex.

## Decisions required before READY

1. Additive graph-response fields versus a successor response type.
2. Exact artifact-evidence request/response version and expiry semantics.
3. CLI artifact distribution targets and version-match rule.
4. `ActuationBatchV2` naming, digest domain, endpoint policy, and binding
   coexistence.
5. The exact minimum receipt extension; no omnibus consequence manifest.
6. Whether any proposal unexpectedly requires a schema migration. A positive
   finding is a release stop pending separate approval.
