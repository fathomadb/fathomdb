---
title: 0.8.25 Slice 35 status
status: COMPLETE_ON_RELEASE_BRANCH
slice: 35
updated: 2026-09-05
---

# Slice 35 status

## Current state

Slice 35 is complete on `release/0.8.25`. Schema step 31, allowlisted eligibility
before lexical/vector/graph truncation, an optional Engine-minted frozen read
context, visibility-generation invalidation, typed cross-SDK failures, and
serving-index mutation coupling are implemented. Ordinary reads remain
unchanged. Full retained snapshots and leases remain allocated to 0.8.27.

Independent design review passed after one corrective cycle. Independent
implementation review passed at FIX-7 with no actionable P1/P2/material P3
finding. Separate verification found and corrected one evidence-boundary wording
error, then returned VERIFIED with no remaining factual blocker.

## Evidence

- Measured executable/native candidate: `0aff1cb0`.
- Post-measurement public Python type-stub correction: `67f708fb`.
- Final reviewed closeout state: `74c191b5`.
- Design and corrective amendment: [`design.md`](design.md) and
  [`design-amendment-fix-6.md`](design-amendment-fix-6.md).
- Final design review: [`design-review-fix-1.md`](design-review-fix-1.md).
- Final implementation review:
  [`implementation-review-fix-7.md`](implementation-review-fix-7.md).
- Verification matrix and independent audit:
  [`verification-matrix.md`](verification-matrix.md) and
  [`verification-review.md`](verification-review.md).
- Authoritative clean receipt:
  `scale-02-slice35-20260905T0404Z-659c38ea`.
- Performance result: 1.623% p50 and 2.121% p95 95-percent upper relative
  regression, under the preregistered 3% boundary; 5,500 measured
  `Engine.search_text_only` calls per arm; zero errors/timeouts.
- Final unconfined fast verifier: 103/103 suites passed, zero skipped/excluded.
- Focused final correction suite: 46/46 passed; clean-clone RED/GREEN
  reproduction: three expected failures at RED, then 7/7 passes at GREEN.
- Portable measurement-classification validation: pass.
- CUDA dense-eligibility route on RTX 3090 GPU 0: pass.
- Fresh wheel and offline npm/native installed smokes: pass.

The four historical false-operation-label receipts remain immutable and are
quarantined by policy version 3. The 0347 truthful receipt remains valid but is
superseded by the 0404 rerun. The separate bulk-ingest latency signal remains
assigned to Slice 75 and does not weaken this narrow legacy-search claim.

Windows runtime was unavailable, so no Windows execution result is claimed.

## Next

Compact the durable boundary, then begin dependency-ordered Slice 40: core
projection generation and readiness correlation.
