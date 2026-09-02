---
title: 0.8.25 Slice 65 — deterministic candidate selection qualification
status: REALLOCATED_EXPERIMENTAL
depends_on: 60
design: design.md
design_status: REVIEWED_EVIDENCE_NOT_0.8.25_AUTHORITY
---

# Slice 65 plan

> **Reallocated:** This reviewed plan is preserved as experimental evidence.
> It is not part of the active 0.8.25 ladder. Manual profile-contract work is
> reconsidered in 0.8.28; entity/alias, complementary/coverage, and new
> MMR/diversity treatments are reviewed when planning 0.8.29. See the
> [scope adjustment](../../scope-adjustment-2026-09-02.md).

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
