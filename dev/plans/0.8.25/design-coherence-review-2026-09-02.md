---
title: 0.8.25 design coherence and scope-alignment review
status: COMPLETE
reviewed_on: 2026-09-02
scope_authority: dev/plans/0.8.25/scope-adjustment-2026-09-02.md
result: ALIGNED_FORMAL_SLICE_REVIEWS_PENDING
---

# 0.8.25 design coherence and scope-alignment review

## Review question

Do architecture v2 and the active Slice 10–75 designs form one coherent,
dependency-ordered implementation of the approved 0.8.25 release plan and each
corresponding slice plan?

This is a documentation/scope review. It does not replace the independent
formal design review required when each slice advances from DRAFT to READY.

## Result

Yes, after the corrections recorded below. Every active design now specifies
only its retained 0.8.25 mechanism, uses the same linear dependency ladder, and
leaves semantic policy outside FathomDB. No unresolved plan-to-design or
cross-slice coherence finding remains.

Design work removed from active contracts is preserved in
[`0.8.x-after-0.8.25-design-notes.md`](../../design/0.8.x-after-0.8.25-design-notes.md)
with target release, re-entry condition, and design inputs. The original
maximum-envelope reviews and Git history remain historical evidence.

## Findings resolved

| ID | Finding | Resolution |
| --- | --- | --- |
| D25-C01 | Architecture v2 read as both a multi-release destination and a 0.8.25 delivery promise. | Versioned it as architecture 2.1 and added an explicit 0.8.25 implementation profile. |
| D25-C02 | Slice 15 allowed unbounded multi-source links while Slice 20 deferred multi-source dependencies. | Retained an additive list encoding but enforced zero/one source link in 0.8.25. |
| D25-C03 | Slices 20/25/30 still normatively specified dependency sets/liveness, omnibus actuation journals, and recursive closure. | Replaced them with one pinned source dependency, a 128-operation one-transaction batch, and direct-dependent closure. |
| D25-C04 | Slice 35 imposed persisted expiring snapshot leases on ordinary downstream designs. | Defined an optional self-contained reproduce-or-fail read context with no lease row, TTL, or held transaction. |
| D25-C05 | Slice 40 exposed the richer public projection-work manifest excluded by scope. | Kept immutable generation identity, truthful readiness, and compact mutation status only. |
| D25-C06 | Slice 45 included graph pagination and full lease-bound cursor semantics. | Limited it to canonical and `operational_state` keyset pages with compact stale-or-reproduce continuation. |
| D25-C07 | Slice 50 required persisted replay receipts and multi-source evidence. | Made evidence opt-in, stateless, one-source, context-bound, and non-disclosing. |
| D25-C08 | Slice 55 specified persisted BFS traces, frozen integrity jobs, and repair/index orchestration. | Limited it to bounded one-call reciprocal traces, inclusion/degradation explanation, and synchronous read-only checks. |
| D25-C09 | Slice 60 specified graph continuation and replayable full path evidence. | Limited it to honored constraints, deterministic one-page targets, compact graph origin, and a hard work bound. |
| D25-C10 | Slice 75 retained an exhaustive scale/platform/feature matrix. | Reduced it to installed parity, two representative concurrency cells, focused 10k overhead/lifecycle checks, and one packaged native search witness. |
| D25-C11 | Slice 10 and Slice 75 both appeared to own the same native witness. | Slice 10 owns the small classification fixture; Slice 75 owns the packaged-candidate release witness. |

## Slice alignment

| Slice | Plan outcome | Reconciled design boundary | Result |
| ---: | --- | --- | --- |
| 10 | Executable measurement classification | Sidecar schema, execution witnesses, historical reclassification, small native fixture | Aligned |
| 15 | Identity and canonical provenance | Immutable revision/source identity, exact locator/hash, zero/one-source provenance, shared wire rules | Aligned |
| 20 | Core dependency registration | One pinned canonical-source-to-derived relation with bounded reciprocal lookup | Aligned |
| 25 | Core atomic actuation | Canonical/derived/dependency/lifecycle operations, 128-op atomic transaction, compact idempotent receipt | Aligned |
| 30 | Core lifecycle/erasure closure | Direct-dependent barrier, propagation, restart/resume, and zero-orphan proof | Aligned |
| 35 | Eligibility plus optional frozen reads | All-arm pre-truncation eligibility and opt-in self-contained read context | Aligned |
| 40 | Core projection readiness | Immutable generation, contiguous readiness, restart-safe work status, compact correlation | Aligned |
| 45 | Minimal pagination/state reads | Canonical and `operational_state` point/keyset reads; stale-or-reproduce continuation | Aligned |
| 50 | Compact evidence | Stateless opt-in one-source reference, exact resolution, non-disclosure, unchanged default hit | Aligned |
| 55 | Basic trace/integrity | One-call reciprocal trace, compact inclusion/degradation, bounded read-only checks | Aligned |
| 60 | Minimal graph parity | Honored seed/direction/edge/target/eligibility/bounds in one deterministic page | Aligned |
| 75 | Trimmed release closure | Installed parity, representative concurrency/lifecycle/performance, packaged native witness | Aligned |

## Cross-slice invariants checked

- The dependency chain is exactly
  `7 -> 10 -> 15 -> 20 -> 25 -> 30 -> 35 -> 40 -> 45 -> 50 -> 55 -> 60 -> 75`.
- Slice 15 owns shared wire/SDK evolution; later slices define only their delta.
- Slice 20's one-source relation matches Slice 15 provenance and Slice 50
  evidence cardinality.
- Slice 25 creates Slice 30 barriers/intents atomically; Slice 30 owns their
  propagation and proof.
- Slice 35 eligibility and current fences govern pagination, evidence,
  tracing, and graph expansion before truncation.
- Slice 40 generation identity is referenced by optional read/evidence
  contracts without requiring a persisted lease.
- Slice 45 cursors never enter ranked top-K or Slice 60 graph results.
- Slice 55 is diagnostic/read-only; lifecycle mutation remains in Slices 25/30.
- Slice 60 adds a constrained primitive without changing A0/default retrieval.
- Slice 75 audits owner-slice evidence and cannot compensate for a missing RED,
  GREEN, design review, platform route, or lifecycle test.

## Readiness effect

The documents are scope-aligned DRAFT designs, not READY designs. Their earlier
independent reviews covered the maximum envelope; the normative designs changed
materially during narrowing. Each active slice must therefore complete its
normal requirements/acceptance-criteria step and an independent formal design
review (maximum three FIX-n cycles) after its dependencies are satisfied.

No product implementation, benchmark execution, publication, or registry
mutation was performed by this review.
