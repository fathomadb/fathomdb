---
title: Memex collaboration Slice 30 — atomic derived-edge actuation decision
status: DRAFT
depends_on: 20
date: 2026-09-12
---

# Slice 30 plan

## Outcome and framing

Determine how FathomDB should advance governed graph-edge mutation so Memex can
atomically commit a derived claim or observation, its canonical-source
dependency, and semantic edges such as `supports`, `refutes`, or `corrects`.

The premise is narrow and code-grounded:

- “FathomDB chose nodes instead of edges” is incorrect.
- FathomDB 0.8.25 deliberately shipped a narrower closed actuation operation
  set.
- That set cannot satisfy Memex's atomic graph-authoring invariant because an
  edge written separately through `Engine.write` creates a crash boundary and
  a second receipt.

Edges already carry body, provenance, temporal state, confidence,
logical/revision identity, retrieval projections, and traversal semantics. The
open question is how those existing capabilities enter the governed atomic
actuation protocol safely.

This is a decision and risk-reduction slice. It does not authorize an API or
implementation until the alternatives and compatibility costs are resolved.

## Consumer invariant and benefits

The candidate contract must commit all three effects or none:

1. derived semantic node or observation revision;
2. immutable canonical-source dependency; and
3. one or more derived semantic edge revisions.

Benefits to measure and document:

- removal of the node/dependency-to-edge crash boundary;
- one idempotency key, digest, terminal receipt, and replay outcome;
- atomic provenance and dependency closure for graph-authored memory;
- immediate enablement of Memex graph authoring without a shadow journal;
- preservation of FathomDB's single-writer and canonical-log invariants; and
- a reusable mechanism for other consumers that author derived fact-edges.

## Decision work

### 1. Establish the exact existing edge contract

Trace `PreparedWrite::Edge`, canonical-edge supersession, provenance storage,
FTS/vector projection, temporal invalidation, confidence, graph traversal,
erasure, dependency closure, and receipt generation. Build a field-by-field
map from the existing `ProvenancedEdgeV1`/edge write path to the proposed
governed operation. Any field that cannot be carried without loss is a blocker.

### 2. Compare API shapes

Evaluate at least:

- **A — `PutDerivedEdge` in `ActuationOperationV1`.** Smallest caller surface,
  but adding a variant to a documented closed/exhaustive V1 enum can break Rust
  matches, strict decoders, request digests, and cross-SDK conformance.
- **B — `ActuationBatchV2` with `PutDerivedEdge`.** Clean closed-version
  evolution, but risks duplicating validation, replay, receipts, and bindings.
- **C — a new graph-actuation batch that can include nodes, dependencies, and
  edges.** Domain-clear, but risks a second transaction authority and divergent
  semantics.
- **D — reify relations as nodes.** Valid only for genuinely n-ary or
  independently embedded facts; it does not solve first-class binary semantic
  edge authoring and must not become a compatibility workaround.
- **E — Memex journal plus separate `Engine.write`.** Rejected because it
  preserves the crash boundary and splits receipts/authority.

The working hypothesis is a provenance-required `put_derived_edge`, not an
unqualified `put_edge`, but the slice must earn that decision.

### 3. Resolve implementation-shaping questions

- What exact edge carrier includes endpoint IDs, logical/revision identity,
  source provenance, body, valid interval, confidence, and projection policy?
- Are both endpoints required to exist before the batch, or may earlier
  operations in the same ordered batch create them?
- Does a missing endpoint refuse the batch, or preserve today's dangling-edge
  flag-and-count behavior? Governed actuation should default to refusal unless
  compatibility evidence proves otherwise.
- Can a registered dependency target an edge revision today? If so, prove
  closure, erasure, and lifecycle behavior; if not, scope the required generalization.
- How are edge supersession, temporal invalidation, and lifecycle transition
  composed without producing two active revisions or resurrecting invalid data?
- How are source-set bounds, affected-revision bounds, pending projection
  cursors, and work limits counted when one operation creates an edge?
- Does a batch containing multiple revisions or endpoints have an unambiguous
  validation and execution order?
- What receipt fields identify committed edge revisions and their projection
  readiness without changing replay semantics?

### 4. Threat- and failure-model review

Analyze:

- rollback at every fault-injection point, including after canonical edge write,
  provenance mapping, dependency registration, FTS projection, and vector
  scheduling;
- identical replay after restart and conflict on operation-ID reuse with a
  different edge payload;
- closed-enum compatibility in Rust, Python, TypeScript, N-API, PyO3, and wire
  codecs;
- canonical request hashing and historical receipt verification across the
  version change;
- source erasure and receipt redaction when receipts name edge revisions;
- concurrent supersession, endpoint deletion, dependency closure, and
  projection completion;
- malformed endpoint identities, self-loops, duplicate edges, cycles, and
  same-batch conflicts;
- edge temporal representation and invalid-interval hazards;
- projection latency and amplification for embedded edge bodies; and
- migration pressure. Reuse the existing schema; any data rewrite is a stop
  gate.

### 5. Produce the decision package

Deliver:

- an as-built edge-capability and transaction-boundary map;
- an alternatives matrix scored for correctness, compatibility, complexity,
  performance, and consumer utility;
- a recommended public contract with exact versioning rationale;
- a successor ADR or explicit no-go decision;
- interface diffs for Rust, Python, TypeScript, and wire;
- a RED-test matrix and fault-injection plan; and
- a separately estimated implementation slice, including package verification.

## Acceptance criteria

- **S30-AC1:** The decision package demonstrates from code that FathomDB is not
  node-only and isolates the missing transaction composition precisely.
- **S30-AC2:** At least the five alternatives above are compared, and rejection
  reasons are tied to invariants rather than preference.
- **S30-AC3:** Closed-V1 evolution versus V2 is resolved explicitly for every
  binding and the canonical request digest.
- **S30-AC4:** Endpoint, supersession, temporal, dependency, lifecycle,
  erasure, replay, receipt, and projection semantics are specified.
- **S30-AC5:** Fault tests can prove exact pre-state after any refused or failed
  batch and exact replay after restart.
- **S30-AC6:** The proposal preserves ontology neutrality: FathomDB does not
  assign the meaning of `supports`, `refutes`, or `corrects`.
- **S30-AC7:** The decision identifies measurable implementation and runtime
  risks, mitigations, stop gates, and a bounded follow-on delivery slice.

## Likely implementation risks and mitigations

| Risk | Why it matters | Required mitigation |
| --- | --- | --- |
| Adding to a closed V1 enum | Exhaustive matches and strict decoders can break | Choose V2 unless a complete compatibility proof supports additive V1 evolution |
| Partial edge side effects | Canonical, provenance, dependency, and projections can diverge | One writer transaction plus fault injection after every internal step |
| Ambiguous endpoint ordering | Same-batch nodes may not exist at validation time | Define two-phase validation and ordered visibility explicitly |
| Edge revision not accepted as dependency target | Closure and erasure may miss derived relationships | Generalize revision-class validation and test edge closure end to end |
| Receipt/replay drift | Same operation ID could produce a different historical meaning | Version the canonical digest and preserve exact stored terminal receipts |
| Projection amplification | Edge body may schedule FTS and vector work | Bound operations/cursors and expose projection readiness in the one receipt |
| Temporal or supersession mismatch | A new edge can coexist incorrectly or resurrect prior facts | Reuse one canonical supersession path and pin interval validation with RED tests |
| Ontology leakage | Engine could hard-code Memex relation policy | Keep kinds opaque; validate mechanics only |

## Stop gates

Stop if the proposal requires a second writer, a Memex-side journal, partial
commit, unversioned digest change, silent dangling endpoints, fabricated
provenance, weakened erasure/closure, a data migration, or semantic policy in
the engine. Stop also if `put_derived_edge` cannot faithfully represent every
field and invariant already supported by the ordinary edge write path.
