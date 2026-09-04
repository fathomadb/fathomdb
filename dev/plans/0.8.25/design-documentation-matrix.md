---
title: 0.8.25 design-documentation work matrix
status: ACTIVE_DESIGNS_SCOPE_RECONCILED_FORMAL_REVIEWS_PENDING
target_release: 0.8.25
architecture: dev/design/fathomdb-data-plane-architecture-v2.md
---

# 0.8.25 design-documentation work matrix

## Purpose

This is the one-to-one authority map for the 21 logical design needs projected
by Slices 10–75. The needs are consolidated into fourteen slice-owned design
records. A slice design may be reviewed before its predecessor implements, but
cannot become READY until the preceding slice and Slice 7 are complete.

This matrix records the completed maximum-envelope design campaign and its
later reconciliation. The owner-approved
[scope adjustment](scope-adjustment-2026-09-02.md) narrowed 0.8.25. The twelve
active designs now contain only that executable subset; removed design work is
preserved in
[`0.8.x-after-0.8.25-design-notes.md`](../../design/0.8.x-after-0.8.25-design-notes.md).
Slice 65/70 designs remain reallocated historical evidence.

Disposition vocabulary is mechanical:

- **new:** no direct design exists; write an architecture-grounded design;
- **successor:** preserve accepted/locked history and write the 0.8.25 delta;
- **amend:** review a living canonical design and version its accepted change;
- **adopt:** reuse a current decision without rewriting it;
- **experimental input:** retain an unreviewed/experimental design as evidence
  and write a product design without implicitly promoting it;
- **reject:** record why an existing candidate cannot satisfy the approved
  requirement. Rejection never deletes historical evidence.

## Work matrix

| # | Slice | Logical design need | Authority | Current design inputs | Required record | Planned disposition | Review status |
| ---: | ---: | --- | --- | --- | --- | --- | --- |
| 1 | 10 | Measurement classification and receipt migration | N25-04; R25/AC25-10; Memex 24 | `earp.md`; performance PROGRAM and GLOBAL-01 receipts | `features/slice-10/design.md` | New; adopt EARP receipt discipline | [PASS cycle 2](features/design-review-10-30-cycle2.md) |
| 2 | 15 | Revision identity, source versions, UTF-8 locators, hashes, migration | N25-01/02; R25/AC25-15; Memex 1/2; A25-02 | `engine.md`; canonical-identity ADR; erasure design; extraction span protocol | `features/slice-15/design.md` | Successor; preserve historical identity decisions | [PASS cycle 2](features/design-review-10-30-cycle2.md) |
| 3 | 15–70 | Wire evolution, unknown behavior, SDK and Windows parity | Memex 23; A25-05 | `bindings.md`; `interfaces/wire.md`; JSON-schema policy | Slice 15 shared rules; delta in every later public/persisted design | Adopt generic policy; write 0.8.25 shared delta | [PASS cycle 2](features/design-review-10-30-cycle2.md); [PASS cycle 3](features/design-review-35-55-cycle3.md); [PASS cycle 2](features/design-review-60-75-cycle2.md) |
| 4 | 20 | Dependency graph, source sets, cycles, lookup, liveness grammar | N25-01; R25/AC25-20; Memex 4/14; A25-04 | lifecycle protocol; nested-source projections | `features/slice-20/design.md` | New; predecessors remain substrate | [READY PASS cycle 5](features/slice-20/design-review-cycle5.md) |
| 5 | 25 | Atomic semantic batch, validation, idempotency, refusal, receipts | N25-01; R25/AC25-25; Memex 3/17/18; A25-05 | typed-write and PreparedWrite ADRs; Engine batch; consolidation ADR | `features/slice-25/design.md` | Successor to current write boundary | [PASS cycle 2](features/design-review-10-30-cycle2.md) |
| 6 | 30 | Dependency-aware lifecycle/erasure propagation, fencing, recovery | N25-01/02; R25/AC25-30; Memex 5/6; A25-04/05 | lifecycle protocol; erasure design; nested-source projections | `features/slice-30/design.md` | Successor; preserve existing lifecycle verbs | [PASS cycle 2](features/design-review-10-30-cycle2.md) |
| 7 | 35 | Engine-minted frozen snapshot | N25-02/03; R25/AC25-35; Memex 7/21; A25-01/05 | Engine reader model; `ReadView`; op-store snapshot path | `features/slice-35/design.md` | New cross-operation contract | [PASS cycle 3](features/design-review-35-55-cycle3.md) |
| 8 | 35 | Eligibility grammar and pre-truncation execution | N25-03; R25/AC25-35; Memex 8; A25-05/06 | filter grammar and unification ADRs | `features/slice-35/design.md` | Successor to current filter compilation | [PASS cycle 3](features/design-review-35-55-cycle3.md) |
| 9 | 40 | Projection-generation identity and mutation/readiness correlation | N25-01/04; R25/AC25-40; Memex 13; A25-05 | projection model; runtime-state and status designs | `features/slice-40/design.md` | Successor; preserve current readiness meanings | [PASS cycle 3](features/design-review-35-55-cycle3.md) |
| 10 | 45 | Opaque pagination and governed `operational_state` reads | N25-02; R25/AC25-45; Memex 9 plus current-state need; A25-03/05 | op-store design/ADR; nested-source projections | `features/slice-45/design.md` | New unified continuation contract | [PASS cycle 3](features/design-review-35-55-cycle3.md) |
| 11 | 50 | `EvidenceRef`, encoding, authorization, resolution, non-disclosure | N25-02; R25/AC25-50; Memex 10/11/12; A25-02/05/07 | provenance, `ReadView`, compact `SearchHit` designs | `features/slice-50/design.md` | New opt-in evidence contract | [PASS cycle 3](features/design-review-35-55-cycle3.md) |
| 12 | 55 | Retrieval explanation and privacy-safe correlation | N25-02/04; R25/AC25-55; Memex 12/19; A25-05/06 | 0.8.8 explanation and telemetry designs | `features/slice-55/design.md` | Successor amendment; preserve ratified history | [PASS cycle 3](features/design-review-35-55-cycle3.md) |
| 13 | 55 | Bidirectional provenance tracing, integrity checks, governed repair | N25-01/02; R25/AC25-55; Memex 19/20; A25-05/06 | erasure, orphan-provenance, projection, doctor designs | `features/slice-55/design.md` | New dependency-aware section | [PASS cycle 3](features/design-review-35-55-cycle3.md) |
| 14 | 60 | Constrained graph expansion, continuation, path evidence | N25-03; R25/AC25-60; Memex 12/15; A25-05/06 | retrieval design; graph model/traversal ADRs | `features/slice-60/design.md` | Successor to combined expansion | [PASS cycle 2](features/design-review-60-75-cycle2.md) |
| 15 | 65–70 | Named profile, configuration, qualification, promotion contract | N25-03/04; R25/AC25-65/70; Memex 16; A25-05 when shipped | planner/router PSD; EARP | Slice 65 shared contract; Slice 70 reference/delta | Experimental input; new bounded profile contract | [PASS cycle 2](features/design-review-60-75-cycle2.md) |
| 16 | 65 | Fusion, duplicate suppression, diversity selection | R25/AC25-65; Memex 16 | RRF ADR; MMR hypothesis/designs | `features/slice-65/design.md` | Adopt RRF baseline; qualify new opt-in treatments | [PASS cycle 2](features/design-review-60-75-cycle2.md) |
| 17 | 65 | Entity/alias, complementary-evidence, coverage-aware selection | R25/AC25-65; Memex 16 | GLOBAL/GRAPH/REASON hypotheses and receipts | `features/slice-65/design.md` | Experimental input; new treatment design | [PASS cycle 2](features/design-review-60-75-cycle2.md) |
| 18 | 70 | Time-scoped and changed-fact temporal retrieval | R25/AC25-70; Memex 16 | temporal graph/storage designs; TEMPORAL-01 evidence | `features/slice-70/design.md` | Experimental input; new temporal profile | [PASS cycle 2](features/design-review-60-75-cycle2.md) |
| 19 | 70 | Associative graph diffusion | R25/AC25-70; Memex 16 | M1 PPR-fusion harness; GRAPH/REASON results | `features/slice-70/design.md` | Experimental input; product successor or durable rejection | [PASS cycle 2](features/design-review-60-75-cycle2.md) |
| 20 | 75 | Integrated concurrency, lifecycle, latency, resource, rebuild workload | N25-04; R25/AC25-75; Memex 21/22/24 | Scale v2; EARP; performance PROGRAM | `features/slice-75/design.md` | Adopt measurement framework; new release matrix | [PASS cycle 2](features/design-review-60-75-cycle2.md) |
| 21 | 75 | Installed SDK/wire/Windows/CUDA/receipt conformance | R25/AC25-75; Memex 23/24 | release, bindings, interface, workflow designs | `features/slice-75/design.md` | Adopt release mechanisms; new 0.8.25 fixture matrix | [PASS cycle 2](features/design-review-60-75-cycle2.md) |

## Current executable allocation

The table above is the historical maximum-envelope map and review record. The
current allocation is intentionally split:

| Original rows | 0.8.25 design authority | Moved design authority |
| --- | --- | --- |
| 1–3 | Slices 10/15, with zero/one-source cardinality | Multi-source cardinality in the post-0.8.25 notes |
| 4 | Slice 20 one-source relation | 0.8.26 source sets, derived dependencies, liveness |
| 5 | Slice 25 bounded core actuation and compact receipt | 0.8.26 broader operations/consequences; omnibus journal Parked |
| 6 | Slice 30 direct-dependent closure | 0.8.26 recursive/multi-source closure |
| 7–8 | Slice 35 eligibility plus optional self-contained context | 0.8.27 persisted snapshot leases |
| 9 | Slice 40 generation/readiness core | Rich public work manifests Parked |
| 10 | Slice 45 canonical/state keyset pages | 0.8.27 full cursor leases and generalized graph pages |
| 11 | Slice 50 stateless one-source evidence | 0.8.27 persisted replay; 0.8.26 multi-source evidence |
| 12–13 | Slice 55 compact inclusion/degradation, one-call trace/checks | 0.8.28 exclusion tracing; 0.8.33 jobs/repair |
| 14 | Slice 60 deterministic one-page constrained expansion | 0.8.28 graph continuation/full path replay |
| 15–19 | No 0.8.25 implementation authority | 0.8.28 and odd-micro experimental reviews |
| 20–21 | Slice 75 representative release closure | 0.8.33 exhaustive matrices |

“Mapped exactly once” therefore describes the original campaign. Current
implementation authority is the active slice design for retained work and
[`0.8.x-after-0.8.25-design-notes.md`](../../design/0.8.x-after-0.8.25-design-notes.md)
for every moved item.

## Review closure

- Slices 10–30: cycle 0 review, FIX-1, cycle 1 review, FIX-2, and final
  [cycle 2 PASS](features/design-review-10-30-cycle2.md).
- Slices 35–55: cycle 0 review, FIX-1, cycle 1 review, FIX-2, cycle 2
  review, FIX-3, and final
  [cycle 3 PASS](features/design-review-35-55-cycle3.md).
- Slices 60–75: cycle 0 review, FIX-1, cycle 1 review, FIX-2, and final
  [cycle 2 PASS](features/design-review-60-75-cycle2.md).

All maximum-envelope implementation-shaping P1/P2 findings were resolved. The
later [scope-alignment review](design-coherence-review-2026-09-02.md) resolved
all plan/design coherence findings and rewrote the active contracts. Because
those normative designs changed materially, every active slice still requires
its formal independent READY review after Slice 7 and sequential dependencies.
Slices 65/70 cannot become READY for 0.8.25.

## Post-review implementation disposition

| Matrix rows | Disposition |
| --- | --- |
| 1–14, 20–21 | Active 0.8.25 portions are scope-reconciled; formal slice-local review remains required before READY. |
| 15, 18 | Manual profile/temporal work reconsidered for 0.8.28. |
| 16–17 | Experimental candidate-selection review at 0.8.29. |
| 19 | Experimental associative/diffusion review at 0.8.31. |
| Portions of 13 and 20 | Full repair/integrity orchestration and exhaustive matrices reviewed at 0.8.33. |
| Database-owned semantic reasoning | Parked outside the approved data-plane architecture. |

## Completion rules

The original matrix is complete because every row names an independently
reviewed maximum-envelope record with no unresolved P1/P2 finding. The active
scope-reconciled designs are DRAFT and require a new formal slice-local review.
Review completion never overrides dependencies or grants implementation
authority.
