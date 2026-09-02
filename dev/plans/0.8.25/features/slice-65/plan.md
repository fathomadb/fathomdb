---
title: 0.8.25 Slice 65 — deterministic candidate selection qualification
status: DRAFT
depends_on: 60
design: design.md
design_status: REVIEWED_BLOCKED_ON_SLICE_7
---

# Slice 65 plan

## Outcome and carried obligations

Implement R25/AC25-65 and the first half of Memex need 16. A25-05 applies if a
treatment ships a public or persisted profile. Evaluate
named, bounded, opt-in entity/alias, duplicate suppression, diversity,
complementarity, coverage, and fusion treatments without changing defaults
unless a preregistered quality/lifecycle/efficiency gate passes.

## Verification routes

Selected: fast, heavy, all, all-feature, Windows CPU/native for any shipped
surface, GPU/CUDA for dense/rerank/fusion, and registry-installed smokes for any
promoted profile. Live-model is selected only for a preregistered treatment
that requires model acquisition; operator is N/A.

## Draft-to-ready and delivery

Preregister treatments, corpus splits, metrics, uncertainty, latency/resource
boundaries, default-promotion and rejection policy; design deterministic
selection and receipts; review; preserve RED invariants and implement GREEN;
review; run held-out and selected platform routes; and record accepted and
rejected evidence. Stop on test-set tuning or quality/grounding regression.
