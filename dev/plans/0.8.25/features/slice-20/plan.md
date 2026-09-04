---
title: 0.8.25 Slice 20 — core dependency registration
status: COMPLETE_ON_RELEASE_BRANCH
depends_on: 15
design: design.md
design_status: READY_REVIEW_PASS_CYCLE_5
---

# Slice 20 plan

## Outcome and carried obligations

Implement the core subset of R25/AC25-20 and Memex need 4 under the approved
[scope adjustment](../../scope-adjustment-2026-09-02.md). Add caller-named
dependency identity and bounded reciprocal lookup over Slice 15's existing
pinned canonical-source-to-derived provenance relation. Registration validates
that relation instead of creating a second provenance authority. The Engine
enforces references, roles, uniqueness, and lifecycle shape; it never infers a
dependency or semantic truth.

Multi-source sets, general derived-to-derived graphs, configurable liveness,
and relationship retargeting remain allocated to 0.8.26. Findings that require
one of those mechanisms must be written into that release's scope/design notes
instead of being implemented as an implicit Slice 20 extension.

## Requirements and acceptance

- **S20-R1 — authoritative relation.** A dependency can register only between
  a complete canonical source revision and a complete derived revision whose
  immutable Slice 15 source link names that exact source revision.
- **S20-AC1.** Missing, incomplete, wrong-role, self, or provenance-mismatched
  endpoints return a typed refusal and leave canonical, provenance,
  dependency, projection, and cursor state unchanged.
- **S20-R2 — stable identity and replay.** One caller dependency ID and one
  derived revision identify at most one pinned relation; identical register
  replays are no-op successes.
- **S20-AC2.** Conflicting ID or endpoint reuse rejects atomically; retirement
  is not part of the contract, and the immutable relation survives close/reopen
  without retargeting on supersession.
- **S20-R3 — reciprocal bounded reads.** The Engine exposes source-to-derived
  and derived-to-source lookup without an application shadow index.
- **S20-AC3.** Reads use deterministic order, return no partial result, and fail
  when more than 100 matches exist.
- **S20-R4 — lifecycle-safe storage.** Dependency rows remain consistent with
  the Slice 15 artifact/source registries and cannot become searchable orphan
  authority through purge or source erasure.
- **S20-AC4.** Real-database purge, erasure, corruption, restart, and reindex
  tests either remove the complete affected dependency state or fail closed
  with rollback.
- **S20-R5 — additive public contract.** New persisted and public objects are
  versioned, closed on request, strictly parsed, and equivalent across Rust,
  Python, and TypeScript.
- **S20-AC5.** Canonical conformance fixtures prove version/type/unknown-field
  precedence, RFC 6901 paths, no-write behavior, public exports, and Windows
  build compatibility.
- **S20-AC6.** Dependency mutation uses its own monotonic generation, advances
  on registration or hard-erasure removal, survives restart, and neither
  consumes nor wedges the canonical projection/write cursor.

## Verification routes

Selected: focused schema/Engine/property tests; Python and TypeScript
conformance; lifecycle/erasure regressions; facade exports; migration policy;
`agent-verify --tier=fast`; applicable all-feature tests; Windows
Rust/Python/Node compile and smoke jobs; and locally packaged public-contract
smokes. `--tier=heavy` is required only when focused or fast verification
indicates a cross-cutting failure. GPU/CUDA, live-model, and pre-publication
registry-installed routes are N/A. Operator is selected for existing purge and
erasure paths touched by the implementation; projection rebuild needs only a
non-regression assertion because dependency state does not share its cursor.

## Draft-to-ready and delivery

1. Reconcile the design with the landed Slice 15 schema and lifecycle code.
2. Obtain an independent design review; resolve at most five FIX cycles. No
   unresolved implementation-shaping finding may enter RED.
3. Implement through preserved RED then GREEN commits. Each review correction
   receives its own RED witness before production changes. Implementation
   review allows at most five FIX cycles.
4. Obtain independent verification; resolve at most two verifier FIX cycles.
5. Write `status.md`, update release state/board, and commit closure.
6. Compact reusable lessons into the external memory store and index before
   Slice 25 starts.

Stop on duplicate provenance authority, unbounded lookup, client shadow-index
ownership, missing no-write proof, or ambiguous erasure consequences.
