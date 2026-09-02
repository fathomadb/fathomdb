---
title: 0.8.25 Slice 10 — executable measurement classification
status: DRAFT
depends_on: 7
design: design.md
design_status: SCOPE_RECONCILED_FORMAL_REVIEW_REQUIRED
---

# Slice 10 plan

## Outcome and carried obligations

Implement R25/AC25-10 and Memex need 24: receipts classify data-plane,
semantic-control-plane, and end-to-end metrics; state whether `Engine.search`
ran; identify shared and differing components; reject invalid mixed-layer
claims; and reclassify GLOBAL-01 without rewriting measured evidence.

## Verification routes

Selected: fast, heavy, all, and a native retrieval-only `Engine.search`
classification fixture. The final installed release-candidate witness belongs
to Slice 75. Windows CPU/native, all-feature/operator, GPU/CUDA, live-model,
packaged, and registry-installed are N/A unless the design introduces a public
executable or binding surface; the readiness review must confirm that judgment.

## Draft-to-ready and delivery

Refine numbered requirements and negative receipt-schema criteria; design the
classification and migration path; review it; preserve RED fixtures for
misclassified receipts and GREEN implementation; review implementation; run
the selected routes; and write the status/receipt record. Stop on ambiguous
metric ownership or any attempt to alter historical measurements.
