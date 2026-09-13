---
title: FathomDB 0.8.26 Slice 4 — initial architecture alignment
status: COMPLETE
---

# Slice 4 design — initial architecture alignment

## Initial as-built findings to verify

| Area | Alignment finding | Proposed direction |
| --- | --- | --- |
| frozen explanations | Ordinary search finalizes correlation identity; frozen paths can return before equivalent finalization. | Repair the shared completion boundary; do not substitute non-frozen search or alter ranking. |
| evidence substrate | Exact source resolution and edge artifact revision identity already exist internally, while the public resolved shape includes ranked-search contribution fields. | Factor intrinsic artifact evidence from ranked contribution; add exact frozen artifact resolution without invented ranks. |
| graph results | Graph targets expose logical/kind/body/cursor/origin but insufficient immutable evidence identity for exact follow-up. | Expose target and terminal-edge revision identity separately and bind resolution to frozen eligibility. |
| integrity | Engine and CLI already implement bounded `data-plane-integrity`; distribution and consumer contract are incomplete. | Package the existing read-only operator route before considering any in-process SDK method. |
| edge writes | `ProvenancedEdgeV1` and the ordinary prepared-write/projection path exist; the actuation grammar is a closed four-operation V1 set. | Change `ActuationBatchV1` in place with `PutDerivedEdge`, reusing canonical edge storage and adding no V2 pair. |
| replay/receipts | Actuation digest, replay identity, and compact receipt are persisted contract-sensitive state. | Define one fresh-database changed-in-place V1 receipt/digest/integrity contract; do not retain historical compatibility. |

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
- V1 remains the only functional actuation grammar and changes in place; no V2
  types, methods, parser, router, or redirect path are introduced.
- 0.8.26 accepts fresh databases only and does not migrate prior databases.
- FathomDB stores caller-declared provenance and relation semantics but does
  not decide supports/refutes/corrects policy for Memex.

## Decisions required before READY

1. In-place graph V1 fields versus a first-generation V1 evidence sidecar.
2. Exact artifact-evidence request/response version and expiry semantics.
3. CLI artifact distribution targets and version-match rule.
4. Current V1 digest semantics and endpoint policy.
5. The exact changed-in-place V1 receipt; no omnibus consequence manifest and
   no historical receipt compatibility.
6. How fresh bootstrap reuses internal schema construction while any existing
   earlier database is refused before mutation.

## Completed code-alignment findings

### Frozen explanation

Both frozen entry points return before the ordinary search observability
finalizer; initial result assembly leaves the correlation identity empty.
Slice 10 should call the existing finalization boundary exactly once per
result and avoid duplicate telemetry. Equivalence should normalize only the
requested observability fields rather than demand byte identity.

### Exact graph evidence

Traversal already has target-node and terminal-edge rows within one reader
transaction, and the evidence resolver already enforces context, eligibility,
commitments, and nondisclosure. The lowest-risk architecture is an additive
first-generation V1 graph evidence sidecar or an in-place V1 result carrying
both immutable revisions,
plus a context-bound point resolver that reuses those checks. Logical-ID search
and full-path evidence remain out of scope.

### Operator integrity

The CLI command, engine method, bounded scan, JSON output, public reference,
crates.io install route, registry smoke, and governed-SDK exclusion already
exist. Slice 30 is qualification and hardening, not greenfield distribution.
Its unresolved risk is that the CLI calls normal `Engine::open`; the query is
read-only, but the complete process has not proved non-mutation. First confirm
that Memex can deploy the crates.io CLI. Require exact tool identity plus
schema compatibility, not exact database package-patch equality.

### Derived-edge actuation

`ProvenancedEdgeV1`, canonical edge write/projection, dependency closure, and
edge revision identity already exist. Only the closed actuation grammar lacks
the operation. Existing edge writes intentionally flag and count dangling
edges, and batch validation considers all writes, including later entries.
Slice 8 must choose whether derived-edge actuation preserves that policy or
introduces a stricter documented governed-edge rule; it must not accidentally
make operation order semantically significant.

The receipt table and integrity checker hard-check schema version 1, while the
request digest is domain-separated as V1. They are replacement surfaces, not
compatibility constraints. Slice 35 defines one current V1
receipt/storage/integrity shape for a fresh database and proves that no
historical compatibility or parallel V2 code remains.

## Stop conditions

Stop for parallel functional V1/V2 APIs, migration of an earlier database, implicit
edge-policy change, invented ranking facts, duplicate telemetry, process
mutation in a claimed read-only operator route, or unreviewed changed V1 persisted
state.
