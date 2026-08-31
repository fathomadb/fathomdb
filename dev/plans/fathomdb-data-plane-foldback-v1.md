---
title: FathomDB data-plane foldback plan v1
status: ACTIVE
plan_version: 1
target_release: 0.8.25
architecture: dev/design/fathomdb-data-plane-architecture-v1.md
---

# FathomDB data-plane foldback plan v1

## Outcome

Fold the completed performance program's data-plane findings into FathomDB
without moving semantic policy into the Engine. The four workstreams below are
independent decision slices; each may land only after its own requirements,
acceptance criteria, reviewed design, TDD implementation, implementation
review, and verification evidence are complete.

## Common delivery contract

Each workstream follows this sequence:

1. Write numbered requirements and falsifiable acceptance criteria. Map every
   criterion to a named test or measurement before implementation.
2. Write a design grounded in
   [`fathomdb-data-plane-architecture-v1.md`](../design/fathomdb-data-plane-architecture-v1.md)
   and the current ADR/interface contracts.
3. Obtain an independent design review. Allow at most three documented FIX-n
   cycles. Unresolved P1/P2 findings stop the workstream.
4. Implement with TDD: commit or stage the RED test first, then make it GREEN,
   then refactor without changing the test oracle.
5. Obtain an independent implementation review. Allow at most four documented
   FIX-n cycles, but stop and redesign after the same failure mode recurs twice.
6. Run focused tests, binding parity tests where applicable, lifecycle/erasure
   tests, and the repository verification gate. Store a concise verification
   record with exact commands and commit identity.

Historical benchmark receipts are evidence, not generated test oracles. No
workstream may rewrite a prior result to make a new implementation pass.

## Workstream 1 — measurement classification

**Goal:** prevent end-to-end memory-system measurements from being reported as
Engine retrieval measurements.

Requirements and acceptance criteria must distinguish data-plane,
semantic-control-plane, and end-to-end metrics; record whether `Engine.search`
was exercised; identify shared and differing components in compared arms; and
reject a retrieval-only claim when answer generation or caller-side evidence
planning differs.

The design should extend experiment receipt metadata and validation, not the
Engine API. RED/GREEN tests must cover the two GLOBAL-01 path shapes and an
invalid mixed-layer claim. Verification includes receipt-schema tests and a
read-only reclassification note for existing GLOBAL-01 evidence. No benchmark
rerun is required.

## Workstream 2 — source-complete evidence

**Goal:** let an external semantic component resolve a search result to the
exact visible source revision without bloating every `SearchHit`.

Requirements and acceptance criteria must cover opaque `EvidenceRef` creation,
explicit `ReadView` resolution, exact source revision/text/span, lifecycle and
integrity metadata, not-visible behavior, erasure, superseded historical
visibility, retrieval contribution, and zero change to bare-hit behavior.

The design must decide the resolver carrier, typed failures, binding shapes,
and reference lifetime. It must preserve the existing single-source provenance
decision and explicitly exclude multi-source semantic entailment. RED/GREEN
tests begin with invalid construction, current/superseded/erased resolution,
and binding parity. Verification includes real-database lifecycle tests and a
small source-resolution performance probe.

## Workstream 3 — constrained graph expansion

**Goal:** bring combined search expansion up to the control level already
available through direct graph reads.

Requirements and acceptance criteria must enumerate canonical `edge_kind`,
direction, target kind, `ReadView`, bounds, stable continuation, seed/path
explanation, access/validity enforcement, and deterministic fallback. Consumer
labels and slots are not Engine vocabulary.

The design must extend the existing combined-expansion contract and reuse the
graph store. RED/GREEN tests begin with wrong-direction, wrong-edge-kind,
expired-edge, continuation, and mixed seed/graph ordering cases. Verification
includes graph lifecycle tests, binding parity, and bounded latency/storage
checks; it does not repeat GRAPH-01's rejected exact-anchor treatment.

## Workstream 4 — governed filters and pagination

**Goal:** replace large client-side over-fetch/filter loops with native,
allowlisted predicates and stable pagination when a declared projection can
support them.

Requirements and acceptance criteria must retain equality/range behavior,
define any membership/existence additions, require indexed eligibility, define
typed unsupported-predicate behavior, apply native validity, and specify a
stable continuation contract across search/list operations.

The design must extend the existing predicate grammar rather than introduce a
second filter language. RED/GREEN tests begin with unsupported attributes,
mixed predicate operators, expiration, empty pages, duplicate-free
continuation, and mutation/read-view boundaries. Verification includes query
plan evidence proving eligible predicates are native and a matched performance
test against the former over-fetch pattern.

## Order and gates

1. Complete Workstream 1 first; its classification contract governs all later
   verification reports.
2. Complete Workstream 2 next because graph and filtered results need a stable
   evidence boundary.
3. Workstreams 3 and 4 may then proceed independently in isolated worktrees.
4. Run one compact cross-workstream validation after all accepted slices land:
   constrained retrieval → evidence resolution → lifecycle mutation → stable
   continuation.

The [architecture review](fathomdb-data-plane-architecture-review-v1.md)
reached final approval after two corrections: clarify
the two GLOBAL-01 paths; define the evidence-reference/`ReadView` distinction;
preserve the existing filter grammar and single-source provenance; and frame
graph direction as a combined-expansion gap. Reopen architecture review only
if implementation requirements contradict those decisions.

## Stop conditions

Stop a workstream for an unresolved P1/P2 review finding, a public-surface
change without ADR/interface grounding, a stale/uneraseable derived record, an
unbounded query path, binding drift, or evidence that the capability belongs
to the external semantic component rather than the FathomDB data plane.
