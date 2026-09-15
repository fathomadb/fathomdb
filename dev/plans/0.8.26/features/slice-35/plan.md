---
title: FathomDB 0.8.26 Slice 35 — breaking V1 actuation spike
status: APPROVED_FOR_IMPLEMENTATION
---

# Slice 35 plan — breaking V1 actuation spike

## Purpose

Resolve implementation uncertainty before Slice 40 by building and measuring a
bounded changed-in-place V1 prototype under accepted
[`ADR-0.8.26-breaking-v1-contract-and-fresh-database-boundary.md`](../../../../adr/ADR-0.8.26-breaking-v1-contract-and-fresh-database-boundary.md).
This is an implementation spike with requirements, acceptance, design review,
RED/GREEN discipline, code review, independent verification, status, and
explicit prototype retain/discard disposition.

## Entry reconciliation and disposition

The draft was last changed at `7ae1905f` on 2026-09-13. The implementation
baseline is the clean `release/0.8.26` worktree at `93aaecf8`, after Slice 30.
The following changes and assigned surfaces were reviewed before approval:

1. Slices 10, 15, 20, and 30 completed. Slice 20 added graph-evidence binding
   fields and projection-generation receipt correlation; Slice 30 changed only
   the operator integrity route. Neither added derived-edge actuation or a V2
   actuation path. The nullable `projection_generation_id` is now part of the
   receipt baseline and must also cover edge projection work.
2. D26-01 was ruled for graph evidence and is unrelated to this slice. D26-03,
   D26-04, and D26-05 remain the complete actuation authority: changed-in-place
   V1, complete prospective endpoints, and a compact receipt (`seq-283` through
   `seq-285`). No open HITL decision remains.
3. The assigned engine surface is still `actuation.rs`, reusing
   `ProvenancedEdgeV1`, `validate_write`, and `apply_batch_in_transaction` from
   `lib.rs`. The engine currently has four operations; its simulation and
   committed loops, digest encoder, source-reference inventory, receipt
   integrity checks, and write counters are node-only where edge support is
   required.
4. Python/PyO3 and TypeScript/N-API still parse closed four-variant unions.
   Their exact operation parsers, exported types, shared conformance fixture,
   native stubs, and interface documents are allocated to this slice.
5. The schema remains version 33 and `Engine::open` still migrates earlier
   databases. Enabling the release-wide fresh-database cutover in this spike
   would consume Slice 40 and invalidate unrelated migration tests. This slice
   therefore proves the boundary with a read-only pre-open classifier parameterized
   by prototype schema 34, a test-only schema-34 bootstrap assembled from the
   current migrations plus one no-op marker step, and an exact no-mutation
   fixture; Slice 40 owns the real step 34 and public-open activation.
6. Slice 3–5 allocations were checked. Requirements R26-40A, R26-40D, and
   R26-40E and acceptance AC26-40A, AC26-40D, and AC26-40E are represented
   below. No receipt field beyond the existing affected-revision and pending-
   cursor collections is justified by an edge.

Disposition: approve the draft with three adjustments. Use the existing
receipt shape rather than assuming new edge fields, treat the fresh-database
check as a non-mutating prototype rather than a release-wide activation, and
measure a fixed in-tree pre-edge control in addition to the current candidate
instead of requiring installation of published 0.8.25. These changes keep the
spike decisive without consuming Slice 40 or Slice 50 work.

## Needs, requirements, and acceptance

Need N26-35: a Memex caller must be able to commit its derived node, dependency,
and semantic edge as one governed, replayable unit without a crash boundary or
an alternate API generation.

### Requirements

- The changed-in-place V1 actuation contract is the only functional grammar.
  No parallel V2 request/receipt types, parser, router, or method exists.
- Fresh 0.8.26 bootstrap succeeds. One representative earlier database is
  refused before mutation; no historical migration matrix or converter exists.
- Current V1 implements the inherited operation capabilities plus derived edge
  in one atomic batch and one changed-in-place V1 receipt/replay contract.
- Derived-edge endpoints must exist in the complete prospective batch state,
  including endpoints introduced later in the batch; missing endpoints refuse
  atomically.
- Prospective endpoint existence uses the final active state after all
  same-batch node and lifecycle effects. The edge's position is irrelevant;
  existing node/lifecycle ordering rules determine the final state.
- The changed-in-place V1 receipt stays compact and adds only edge fields
  proven necessary; it has no dangling count or graph-consequence manifest.
- Current V1 validates its own provenance, lifecycle, dependency, source-reference,
  erasure, projection, replay, and integrity invariants without interpreting
  earlier-version rows.

### Acceptance criteria

- **AC26-35A — closed grammar:** Rust, Python, and TypeScript expose exactly one
  V1 actuation method and a five-variant grammar containing `put_derived_edge`;
  source scans find no functional actuation V2/router.
- **AC26-35B — prospective endpoints:** edge-before-endpoints succeeds when
  both endpoints are active in the final simulated batch; missing `from` wins
  over missing `to`, and a later lifecycle delete makes that endpoint absent.
  Every refusal is terminal and leaves no domain rows.
- **AC26-35C — atomic effects:** derived node, dependency, and edge commit under
  one operation ID/transaction. Edge identity, supersession, traversal,
  projection, source erasure, and replay use the canonical machinery.
- **AC26-35D — receipt and integrity:** the current V1 digest is deterministic,
  order-sensitive, and edge-sensitive. Existing compact receipt collections
  truthfully include the new edge and every canonically superseded edge revision,
  bounded to the existing 256-revision schema limit. A prospective 257th
  revision refuses terminally with no committed domain effect; replay survives restart, changed
  bytes conflict, erased IDs remain reserved, and corrupt edge-bearing state
  fails closed.
- **AC26-35E — rollback and concurrency:** invalid provenance, revision
  collision, projection failure, injected post-operation/commit failure, and
  missing endpoints do not partially commit. Eight unique callers complete;
  eight callers sharing exact bytes converge on one identical receipt.
- **AC26-35F — binding conformance:** one shared Unicode/NUL fixture produces
  the same digest and receipt across Rust, Python, and TypeScript. Closed shape,
  error precedence, and canonical RFC 6901 edge paths are exact.
- **AC26-35G — fresh boundary prototype:** fresh/nonexistent and empty database
  candidates are admitted; a representative schema-33 database is refused by
  the prototype before any file or sidecar byte changes. No migration matrix,
  converter, or historical replay path is added.
- **AC26-35H — bounded performance:** record the three-operation unit, a
  128-operation edge-heavy batch, 1,000 sequential calls, and eight concurrent
  callers, including exact API invocation counts, p50/p95, throughput,
  writer/lock failures, slow events, response bytes, pending cursors, and
  checkpointed database growth. Results characterize; they are not a
  compatibility gate.

## TDD RED/GREEN plan

1. Obtain independent review of [`design.md`](design.md), record it, and resolve
   every material finding before code changes.
2. RED-1: add Rust tests for the digest/property contract, prospective endpoint
   matrix, atomic graph unit, rollback/replay/concurrency, receipt integrity,
   erasure/projection/traversal, and fresh-boundary prototype.
   Include positive domain-effect controls for canonical-node put, derived-node
   put, dependency registration, and lifecycle transition before adding the
   fifth operation.
3. GREEN-1: add `PutDerivedEdge` to the engine grammar, digest, simulation,
   committed application, receipt/source-reference handling, and counters by
   reusing canonical edge validation/application. Use one post-simulation
   indexed final-state endpoint pass; operation position stays non-semantic.
4. RED-2/GREEN-2: update the shared conformance fixture and Python/TypeScript
   closed-shape, precedence, Unicode/NUL, and no-V2 tests; extend the existing
   PyO3/N-API parsers and exported unions in place. Update interface docs.
5. RED-3/GREEN-3: add the deterministic measurement harness, prove its scenario
   and call counts, then run the four required workloads against the fixed
   control and current candidate. Refactor only for a measured bounded defect.
6. Obtain independent code review of the actual diff, resolve behavioral
   findings with visible RED/GREEN evidence, then have a different read-only
   subagent run focused verification and the proportionate repository gate.

## Exit and handoff

Write `status.md` with exact RED/GREEN/review/evidence commits. State which
prototype changes Slice 40 may retain and which were discarded. Update the
release ladder with the completed position between Slices 30 and 40. Stop for
HITL if a parallel V2 surface appears, an earlier database mutates,
current-contract integrity is weakened, or performance evidence indicates
an architectural change rather than a bounded implementation correction.
