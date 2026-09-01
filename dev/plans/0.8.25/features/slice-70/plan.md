---
title: 0.8.25 Slice 70 — temporal and associative retrieval qualification
status: DRAFT
depends_on: 65
---

# Slice 70 plan

## Outcome and carried obligations

Implement R25/AC25-70 and the second half of Memex need 16. A25-05 applies if a
treatment ships a public or persisted profile. Evaluate
named, bounded, opt-in temporal retrieval and associative graph-diffusion
treatments against the accepted default, preserving lifecycle and provenance.

## Verification routes

Selected: fast, heavy, all, all-feature, Windows CPU/native for any shipped
surface, GPU/CUDA for dense/rerank/graph treatments, and registry-installed
smokes for any promoted profile. Live-model is selected only when a
preregistered treatment requires it; operator is N/A.

## Draft-to-ready and delivery

Preregister changed-fact, time-scoped, supersession, multi-hop, held-out,
uncertainty, efficiency, and default-regression gates; design deterministic
temporal/diffusion execution and receipts; review; implement RED/GREEN;
review; run selected routes; and record accepted/rejected evidence. Stop on
invalid time leakage, lifecycle drift, or unsupported default promotion.
