---
title: FathomDB 0.8.25 feature-slice plans
status: ACTIVE_DESIGNS_SCOPE_RECONCILED_FORMAL_REVIEWS_PENDING
target_release: 0.8.25
---

# 0.8.25 feature-slice plans

These plans turn the release-level allocation into one owned draft per feature
slice. They do not authorize implementation. Each plan must become READY only
after its requirements, falsifiable acceptance criteria, design, exact test
commands, fixtures, receipt paths, and stop conditions are reviewed.

Every slice uses the delivery loop in
[`fathomdb-data-plane-foldback-v2.md`](../../fathomdb-data-plane-foldback-v2.md):
requirements and acceptance criteria, architecture-grounded design,
independent design review with at most five FIX-n cycles, TDD RED/GREEN,
independent implementation review with at most five FIX-n cycles, verification,
and a durable status record. After that status closes, compact the external
FathomDB memory store and reconcile its index before starting the next slice.

The [design-documentation matrix](../design-documentation-matrix.md) records
the completed maximum-envelope campaign: 21 logical needs mapped exactly once
into fourteen slice-owned design records. The later owner-approved
[scope adjustment](../scope-adjustment-2026-09-02.md) is the implementation
boundary. The [coherence review](../design-coherence-review-2026-09-02.md)
reconciled the twelve active designs to that boundary; removed design work is
preserved in
[`0.8.x-after-0.8.25-design-notes.md`](../../../design/0.8.x-after-0.8.25-design-notes.md).

Independent review is complete for the former maximum-envelope designs. The final review
records are [Slices 10–30](design-review-10-30-cycle2.md),
[Slices 35–55](design-review-35-55-cycle3.md), and
[Slices 60–75](design-review-60-75-cycle2.md). All prior P1/P2 findings are
resolved. The active contracts changed materially during scope reconciliation,
so they remain DRAFT pending their slice-local formal independent review as
well as Slice 7 and sequential gates. Slice 65/70 designs remain historical
reviewed evidence, not 0.8.25 implementation authority.

## Verification route vocabulary

- **fast:** `bash scripts/agent-verify.sh --tier=fast`.
- **heavy:** `bash scripts/agent-verify.sh --tier=heavy`.
- **all:** `bash scripts/agent-verify.sh --tier=all` at slice closure.
- **Windows CPU/native:** applicable `windows-latest` Rust, Python, and Node
  jobs in `.github/workflows/ci.yml` or `.github/workflows/release.yml`.
- **all-feature/operator:** focused Cargo tests with the actual feature set and
  the repository all-feature route where compatible.
- **GPU/CUDA:** `cuda-contract-preflight` plus the applicable CUDA
  package-rehearsal route.
- **live-model:** an explicitly budgeted, receipt-producing route when a
  treatment requires a model or provider.
- **packaged:** isolated consumers install locally built wheel, npm/native
  tarball, crate/CLI artifacts with no source-tree fallback.
- **registry-installed:** post-publication fresh-machine smokes only. This route
  requires separate publication authorization and never blocks pre-publication
  slice READY.

Every selected route must name its exact command or workflow job and receipt
path before the slice becomes READY. Every unselected route must be recorded as
`N/A` with a reason. Windows CUDA is postponed beyond 0.8.25; Windows
CPU/native remains feature-local for public and persisted contracts.

Memex need 23 and A25-05 are carried by every active Slice 15–60 plan that creates or
changes a public or persisted contract. Such a slice must land Rust, Python,
TypeScript, versioned-wire, unknown-field/variant, and Windows CPU/native proof
locally; Slice 75 only audits the combined installed surface.

## Plan and design index

| Slice | Draft plan | Consolidated design |
| ---: | --- | --- |
| 10 | [Measurement classification](slice-10/plan.md) | [Design](slice-10/design.md) |
| 15 | [Identity and canonical provenance](slice-15/plan.md) | [Design](slice-15/design.md) |
| 20 | [Core dependency registration](slice-20/plan.md) | [Design](slice-20/design.md) |
| 25 | [Atomic semantic actuation](slice-25/plan.md) | [Design](slice-25/design.md) |
| 30 | [Lifecycle and erasure closure](slice-30/plan.md) | [Design](slice-30/design.md) |
| 35 | [Eligibility and optional frozen reads](slice-35/plan.md) | [Design](slice-35/design.md) |
| 40 | [Projection generation and readiness](slice-40/plan.md) | [Design](slice-40/design.md) |
| 45 | [Minimal pagination and operational state](slice-45/plan.md) | [Design](slice-45/design.md) |
| 50 | [Source-complete evidence](slice-50/plan.md) | [Design](slice-50/design.md) |
| 55 | [Basic tracing and integrity](slice-55/plan.md) | [Design](slice-55/design.md) |
| 60 | [Minimal constrained graph parity](slice-60/plan.md) | [Design](slice-60/design.md) |
| 75 | [Integrated closure](slice-75/plan.md) | [Design](slice-75/design.md) |

The active slices are strictly sequential; Slice 75 now depends directly on
Slice 60. A later plan may be drafted but cannot become READY while its
dependency is incomplete.

## Preserved reallocated designs

| Former slice | Reviewed evidence | Durable allocation |
| ---: | --- | --- |
| 65 | [Candidate selection plan](slice-65/plan.md) and [design](slice-65/design.md) | Candidate-selection experiments reviewed at 0.8.29; manual profile contract reconsidered for 0.8.28. |
| 70 | [Temporal/associative plan](slice-70/plan.md) and [design](slice-70/design.md) | Temporal profile reconsidered for 0.8.28; associative diffusion reviewed at 0.8.31. |

See the [odd-micro experimental review schedule](../../0.8.29-0.8.33-experimental-review-schedule.md).
