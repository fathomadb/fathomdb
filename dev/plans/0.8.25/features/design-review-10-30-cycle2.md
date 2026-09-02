---
title: 0.8.25 Slice 10–30 independent design re-review — cycle 2
status: COMPLETE
review_cycle: 2
reviewed_on: 2026-09-01
verdict: PASS
source_review: dev/plans/0.8.25/features/design-review-10-30-cycle1.md
source_fix: dev/plans/0.8.25/features/design-review-10-30-fix2.md
---

# Slice 10–30 independent design re-review — cycle 2

## Verdict

**PASS.** FIX-2 resolves C1-15-01, C1-20-01, C1-30-01, and C1-30-02. The
narrow regression scan found no new implementation-shaping P1/P2. All cycle-0
and cycle-1 findings are closed.

This PASS closes design review only. The Slice 10–30 designs correctly remain
`DRAFT_REVIEW` and cannot become READY until Slice 7 activates architecture v2
and the owning slice completes its remaining readiness gates.

## Finding verification

| Finding | Corrected contract and proof | Status |
| --- | --- | --- |
| C1-15-01 revision-ID grammar | Slice 15 defines disjoint caller, Engine-runtime, and Engine-migration validators; the stored column accepts their bounded union; callers cannot use reserved namespaces; shared Rust/Python/TypeScript/wire fixtures cover all forms. | RESOLVED |
| C1-20-01 validity boundary direction | Slice 20 queues only live→non-live boundaries. Active registration requires current liveness, `valid_from` neither queues closure nor auto-reactivates, `valid_until` rules are explicit for `all_required` and `any_surviving`, and tests cover both directions. | RESOLVED |
| C1-30-01 undiscovered-dependent reads | Slice 30 applies a transitive, cycle-safe `closure_visibility_guard` to every governed read before candidate/seed/frontier truncation. Missing index, resource exhaustion, or inability to prove absence fails the entire read; the guard lasts through zero proof. A greater-than-one-page last-descendant test is required. | RESOLVED |
| C1-30-02 semantic-operation journal erasure | Slice 25 indexes journal source/revision references and strips prepared bytes at terminal transition. Slice 30 deterministically resolves referencing nonterminal operations or remains barriered/incomplete, then includes journal payload/reference/receipt/WAL stores in zero proof and raw canaries. | RESOLVED |

## Regression scan

- **Identity/provenance:** artifact/source revisions remain distinct; the
  corrected validator does not weaken uniqueness, namespace, migration, or
  incomplete-provenance behavior.
- **Dependency semantics:** future activation remains caller-controlled;
  strict-current liveness is still independent of caller views and the queue
  remains deterministic for loss boundaries.
- **Atomic actuation:** journal indexing/payload stripping preserves admitted-
  plan recovery and idempotency. Erasure cannot delete required recovery state.
- **Lifecycle/erasure:** the visibility guard closes the paged-discovery window
  without relaxing write barriers, fixed-point propagation, projection drain,
  post-plan dependency proof, or fail-loud WAL behavior.
- **Sequencing and boundary:** Slices 25/30 still use their own write-boundary
  and projection-intent contracts; no future Slice 35/40 type leaked backward.
  Semantic policy remains external.

No new P1 or P2 finding was identified.

## P3 observations

None.
