---
title: FathomDB 0.8.25 feature-slice plans
status: DRAFT
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
independent design review with at most three FIX-n cycles, TDD RED/GREEN,
independent implementation review with at most four FIX-n cycles, verification,
and a durable status record.

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
- **registry-installed:** fresh-machine release-workflow Python, npm/native,
  and CLI smokes. Publication still requires separate HITL authorization.

Every selected route must name its exact command or workflow job and receipt
path before the slice becomes READY. Every unselected route must be recorded as
`N/A` with a reason. Windows CUDA is postponed beyond 0.8.25; Windows
CPU/native remains feature-local for public and persisted contracts.

Memex need 23 and A25-05 are carried by every Slice 15–70 plan that creates or
changes a public or persisted contract. Such a slice must land Rust, Python,
TypeScript, versioned-wire, unknown-field/variant, and Windows CPU/native proof
locally; Slice 75 only audits the combined installed surface.

## Plan index

| Slice | Draft plan |
| ---: | --- |
| 10 | [Measurement classification](slice-10/plan.md) |
| 15 | [Identity and canonical provenance](slice-15/plan.md) |
| 20 | [Dependency registration and liveness](slice-20/plan.md) |
| 25 | [Atomic semantic actuation](slice-25/plan.md) |
| 30 | [Lifecycle and erasure closure](slice-30/plan.md) |
| 35 | [Frozen reads and eligibility](slice-35/plan.md) |
| 40 | [Projection generation and readiness](slice-40/plan.md) |
| 45 | [Governed pagination and operational state](slice-45/plan.md) |
| 50 | [Source-complete evidence](slice-50/plan.md) |
| 55 | [Tracing, explanation, and integrity](slice-55/plan.md) |
| 60 | [Constrained combined graph expansion](slice-60/plan.md) |
| 65 | [Deterministic candidate selection](slice-65/plan.md) |
| 70 | [Temporal and associative retrieval](slice-70/plan.md) |
| 75 | [Integrated closure](slice-75/plan.md) |

Slices are strictly sequential. A later plan may be drafted but cannot become
READY while its dependency is incomplete.
