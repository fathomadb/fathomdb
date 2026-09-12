---
title: FathomDB 0.8.26 Slice 3 — draft contract register
status: DRAFT
---

# Slice 3 design — draft contract register

## Draft user needs

| ID | Draft need | Owner |
| --- | --- | ---: |
| N26-01 | A frozen evidence query can return validated query and per-hit explanation, including a non-empty FathomDB correlation identity. | 10 |
| N26-02 | A constrained graph target or terminal edge can be resolved to exact evidence by immutable artifact revision under the same frozen authority. | 20 |
| N26-03 | An operator can run a version-matched, read-only, machine-readable integrity inspection without Memex issuing raw SQL or gaining repair authority. | 30 |
| N26-04 | A caller can atomically commit a derived node, its canonical dependency, and a provenance-bearing semantic edge. | 40 |
| N26-05 | A mutation receipt can truthfully identify the derived-edge mutation and directly computed transaction effects without carrying Memex semantic policy. | 40 |
| N26-06 | A clean installed-artifact profile proves the combined P0–P2 contract before release publication. | 50 |

## Draft requirement and acceptance allocation

### Slice 10

- Finalize a non-empty correlation ID for both explanation-enabled frozen
  search methods through the same completion path as ordinary search.
- Preserve ranking, eligibility, evidence handles, and explanation-disabled
  behavior byte-for-byte where the contract requires it.
- Document when to use `search_frozen`, `search_with_evidence`, and
  `resolve_evidence`, including snapshot expiry and drift behavior.
- Prove the supported one-source flow from a clean installed artifact.

### Slice 20

- Carry immutable artifact-revision identity for graph targets and terminal
  edges without conflating the two.
- Resolve that revision under a frozen context in one reader transaction.
- Refuse stale, ineligible, undisclosed, or unauthorized evidence without
  leaking existence.
- Return intrinsic evidence fields without fabricating ranked-search fields or
  implying that a terminal edge proves a complete path.

### Slice 30

- Distribute and document `fathomdb doctor data-plane-integrity --json` as a
  version-matched operator route.
- Define quiescence, exit-code, schema-version, bounded-output, privacy, and
  no-mutation guarantees.
- Keep doctor/recovery methods out of the governed Python and TypeScript SDKs.
- Prove the route from release artifacts on supported target classes selected
  in Slice 6.

### Slice 40

- Preserve `ActuationBatchV1` and introduce a versioned batch containing
  `put_derived_edge(ProvenancedEdgeV1)` if compatibility review confirms that
  a V1 extension is unsafe.
- Validate endpoints against the prospective transaction state, commit all
  operations or none, and preserve deterministic digest/replay behavior.
- Add only receipt fields that are directly known and required to represent
  the edge mutation, lifecycle transition, dependency boundary, or projection
  work correlation.
- Prove restart, duplicate-key, fault-injection, endpoint, dependency,
  lifecycle, erasure, and projection invariants across bindings.

### Slice 50

- Execute the integrated installed wheel, npm/native, CLI, restart,
  cross-SDK, and supported-platform matrix without publishing.
- Produce a reproducible artifact/evidence manifest and stop on any contract
  mismatch.

## Draft architecture CRUD

| Area | Proposed operation | Allocation |
| --- | --- | ---: |
| frozen explanation completion | Update interface and public guide; add a defect-closure note if no ADR decision changes | 10 |
| artifact evidence resolution | Create a successor ADR or accepted addendum defining immutable-revision authorization and non-disclosure | 20 |
| graph response identity | Update wire/Rust/Python/TypeScript interfaces with a compatible additive or successor response selected in Slice 6 | 20 |
| operator integrity distribution | Update CLI, packaging, release, and operator documentation; preserve the SDK recovery denylist | 30 |
| actuation grammar | Create a successor ADR for V2 batch semantics and prospective endpoint validation | 40 |
| mutation receipt | Update interfaces only for the minimum accepted V2 fields and digest domain | 40 |
| release conformance | Update release/package documentation and artifact test contract | 50 |

No schema migration is presently expected. Slice 4 must challenge that claim.
Nothing in this register is accepted until Slice 6 records the HITL ruling.
