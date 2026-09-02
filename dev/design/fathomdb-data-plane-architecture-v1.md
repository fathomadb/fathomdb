---
title: FathomDB data-plane architecture v1
status: ACTIVE
architecture_version: 1
implementation_status: planned
target_release: 0.8.25
approved_successor: dev/design/fathomdb-data-plane-architecture-v2.md
successor_activation_gate: 0.8.25 Slice 7 completion
---

# FathomDB data-plane architecture v1

> **Successor notice:** architecture v2 is approved by the 0.8.25 Slice 6 HITL
> decisions and becomes active when Slice 7 completes. Until that gate closes,
> v1 remains the active historical baseline. New Slice 10+ design drafts target
> v2 and may not become READY under v1.

## Purpose and boundary

FathomDB is the durable, provenance-preserving data plane for an agent-memory
system. It owns mechanisms and invariants. A separate semantic component owns
task intent and semantic policy.

FathomDB owns canonical records, identity, source linkage, validity, lifecycle
transitions, erasure, projection readiness, retrieval primitives, governed
mutation, and structural explanation. The semantic component owns extraction,
entity resolution, contradiction judgments, query decomposition, evidence
planning, synthesis, answer verification, model choice, spend, and HITL
policy.

The boundary is complete when a semantic component can implement its policy
through governed FathomDB APIs without raw SQL, private shadow indexes,
duplicated liveness rules, or manual projection cleanup.

## Invariants

1. Canonical records remain the only source of truth. Extracted facts, graph
   edges, vectors, chunks, and summaries are rebuildable projections.
2. Every derived item retains the canonical dependency needed for validity,
   supersession, erasure, and audit.
3. Retrieval returns evidence, not a user-facing semantic conclusion.
4. Model, provider, network, GPU, and paid execution remain explicit caller
   policy.
5. New query capability extends typed, governed contracts; it does not add a
   parallel ad hoc query language.
6. Existing single-source provenance is preserved. Multi-source causal
   provenance requires a separate design and is not implied by this version.

## Measurement boundary

GLOBAL-01 exercised two different paths that must not be conflated:

- The first small witness used FathomDB-backed storage and source linkage but
  bypassed `Engine.search`; it measured an end-to-end memory-system treatment.
- The held-out comparison used `Engine.search` in both arms, but still measured
  retrieval plus caller-side coverage planning and answer generation.

Neither run is a retrieval-only comparison. Future reports must classify each
metric as data-plane, semantic-control-plane, or end-to-end system evidence.
The existing receipts remain valid under their original scope.

## Planned governed capabilities

### Constrained graph expansion

`graph.neighbors` already supports bounded traversal, direction, depth, and a
`ReadView`. The combined search-expansion path lacks equivalent controls. A
successor must extend combined expansion rather than invent another graph
store.

The governed request must support canonical `edge_kind`, direction, target
kind, validity/read view, a bounded result limit, and continuation. Consumer
labels or slots map to `edge_kind` outside the Engine. The response must expose
which seed and edge path contributed each result and must apply the same
visibility rules as direct graph reads.

### Governed filters and pagination

The existing allowlisted predicate grammar remains authoritative. Equality and
range predicates are retained; membership and existence may be added only for
declared indexed attributes. The backend must compile a predicate to an
eligible native projection or return a typed unsupported-predicate outcome.

Search and list operations need stable pagination under the existing ordering
contract. No API may silently over-fetch to a large ceiling and filter JSON
client-side when an equivalent governed native predicate exists. Expiration
and validity remain native lifecycle concerns, not client post-filters.

### Opt-in source-complete evidence

`SearchHit` remains compact and backward compatible. An opt-in evidence
sidecar returns an Engine-owned opaque, immutable `EvidenceRef`; callers do not
construct it from a hit. A resolver uses that reference plus an explicit
`ReadView` to return the exact recorded source revision, canonical text or
span, source identity, owner/access scope when stored, lifecycle state,
retrieval contribution, and integrity hashes.

If the referenced revision is visible in the requested view, the resolver may
return a superseded historical revision. If it is not visible, the resolver
returns a typed not-visible outcome. A caller-owned source-version identifier
is separate from FathomDB's immutable evidence reference.

This contract is source-complete for one recorded source and its derived
projection chain. It does not claim multi-source claim attribution, semantic
stance, or answer entailment.

## Compatibility and safety

- Bare search results retain their current shape and cost.
- New evidence resolution and expansion fields are opt-in.
- Unknown filter, graph, or evidence fields fail closed with typed outcomes.
- Every new public surface requires an ADR or interface-document update and
  Rust/Python/TypeScript parity under the governed facade contract.
- Erasure, supersession, access, and `ReadView` tests precede performance
  claims.

## Release allocation

Release 0.8.25 owns the requirements, acceptance criteria, designs,
implementations, and verification for measurement classification,
source-complete evidence, constrained graph expansion, and governed filtering
with pagination. The execution plan is
[`fathomdb-data-plane-foldback-v1.md`](../plans/fathomdb-data-plane-foldback-v1.md).
The independent architecture review is recorded in
[`fathomdb-data-plane-architecture-review-v1.md`](../plans/fathomdb-data-plane-architecture-review-v1.md).

Semantic extraction, query decomposition, global synthesis, answer generation,
and semantic answer verification remain outside FathomDB's product boundary.
