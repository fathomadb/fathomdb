---
title: FathomDB 0.8.26 Slice 3 — draft contract register
status: COMPLETE
---

# Slice 3 design — draft contract register

## Draft user needs

| ID | Draft need | Owner |
| --- | --- | ---: |
| N26-01 | A frozen evidence query can return validated query and per-hit explanation, including a non-empty FathomDB correlation identity. | 10 |
| N26-02 | A constrained graph target or terminal edge can be resolved to exact evidence by immutable artifact revision under the same frozen authority. | 20 |
| N26-03 | An operator can qualify and safely run the existing versioned, read-only, machine-readable integrity inspection without Memex issuing raw SQL or gaining repair authority. | 30 |
| N26-04 | A caller can atomically commit a derived node, its canonical dependency, and a provenance-bearing semantic edge. | 40 |
| N26-05 | A clean installed-artifact profile proves the combined P0–P2 contract before release publication. | 50 |

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
  in Slice 8.

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
| graph response identity | Update wire/Rust/Python/TypeScript interfaces with a compatible additive or successor response selected in Slice 8 | 20 |
| operator integrity distribution | Update CLI, packaging, release, and operator documentation; preserve the SDK recovery denylist | 30 |
| actuation grammar | Create a successor ADR for V2 batch semantics and prospective endpoint validation | 40 |
| mutation receipt | Update interfaces only for the minimum accepted V2 fields and digest domain | 40 |
| release conformance | Update release/package documentation and artifact test contract | 50 |

Receipt sufficiency is a requirement under N26-04, not an independent user
need. Release conformance extends existing `NEED-006`/`REQ-052` rather than
creating another global need. Nothing in this register is accepted until
Slice 8 records the HITL ruling.

## Draft requirements, acceptance criteria, and evidence

| Requirement | Acceptance criteria | Evidence owner |
| --- | --- | ---: |
| R26-10A: finalize valid, non-empty correlation IDs for both frozen explanation paths | AC26-10A: `search_frozen(explain=true)` and `search_with_evidence(include_explanation=true)` validate with telemetry on and off | 10 |
| R26-10B: preserve frozen ranking, identity, projection, and evidence semantics | AC26-10B: normalized explanation-on/off results preserve ordered hits, scores, identities, projections, and evidence sidecars | 10 |
| R26-10C: document the canonical frozen evidence workflow and diagnostic-only role of `search_frozen` | AC26-10C: API reference and executable recipe cover freeze, evidence search, resolution, expiry, and drift | 10 |
| R26-10D: prove the public contract from a clean installed artifact | AC26-10D: freshly built wheel black-box probe passes without importing worktree sources | 10, 50 |
| R26-20A: expose immutable target and terminal-edge artifact revisions without changing V1 incompatibly | AC26-20A: additive sidecar or successor result returns distinct exact identities across Rust, Python, and TypeScript | 20 |
| R26-20B: resolve an exact artifact revision under the same frozen context and reader transaction | AC26-20B: success, restart, expiry, mismatch, ineligible, revoked, superseded, and nonexistent cases preserve nondisclosure | 20 |
| R26-20C: return intrinsic evidence without ranking fiction | AC26-20C: point evidence contains no fabricated rank or contribution and a terminal edge makes no full-path claim | 20 |
| R26-20D: specify lifecycle, disclosure, and binding behavior | AC26-20D: interfaces, successor ADR, shared fixtures, and bindings agree | 20 |
| R26-30A: qualify the published CLI operator route for Memex deployment | AC26-30A: named exact artifact is installable/discoverable and accepts the selected database schema | 30, 50 |
| R26-30B: make the whole integrity invocation observably non-mutating and bounded | AC26-30B: before/after process-level witness, lock/quiescence behavior, exit codes, and output bounds pass | 30 |
| R26-30C: preserve the governed SDK boundary | AC26-30C: doctor/recovery remain absent from Python and TypeScript public SDKs | 30, 50 |
| R26-40A: preserve V1 while adding versioned derived-edge actuation | AC26-40A: V1 encoding/digest/replay remain stable and V2 construction is cross-binding compatible | 40 |
| R26-40B: commit node, dependency, and provenance-bearing edge all-or-none | AC26-40B: transactional fault, duplicate, restart, endpoint, dependency, lifecycle, erasure, and projection tests pass | 40 |
| R26-40C: define endpoint semantics explicitly | AC26-40C: approved dangling policy is documented and tested against complete prospective batch state unless order sensitivity is explicitly approved | 40 |
| R26-40D: evolve receipts only when existing V1 fields cannot truthfully represent the request, with explicit shared operation-ID semantics | AC26-40D: receipt/storage compatibility, integrity, replay, and cross-version operation-ID collision behavior are proved | 40 |

## CRUD allocation and compatibility gates

- Update existing global `NEED-006`, `NEED-010`–`013`, `NEED-021`, and
  `NEED-024`–`026`, plus `REQ-036`, `REQ-039`, `REQ-050`, and
  `REQ-052`–`054`, only where the accepted wording is incomplete. Create the
  release-local R26/AC26 trace in owning feature slices.
- Slice 20 should create an additive evidence sidecar or successor response;
  do not add required fields directly to `GraphTargetV1`.
- Slice 30 updates the incomplete CLI verb inventory and qualifies the route
  already shipped. It does not create an SDK operator API by default.
- Slice 40 needs a successor actuation decision. A receipt schema/storage
  change is conditional, not presumed absent.
- Slice 50 owns integrated artifact evidence, not feature contract authoring.
