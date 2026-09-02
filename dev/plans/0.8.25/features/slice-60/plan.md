---
title: 0.8.25 Slice 60 — minimal constrained graph parity
status: DRAFT
depends_on: 55
design: design.md
design_status: REVIEWED_MAX_ENVELOPE_SCOPE_RECONCILIATION_REQUIRED
---

# Slice 60 plan

## Outcome and carried obligations

Implement the minimal subset of R25/AC25-60; Memex need 15 and graph-origin
portions of 12; and A25-05/A25-06 under the approved
[scope adjustment](../../scope-adjustment-2026-09-02.md). Make combined
expansion honor query or explicit seeds, direction, edge kind, target kind,
indexed eligibility, bounds, and one read context with deterministic one-page
results. Rich continuation and replayable full path evidence are allocated to
0.8.28.

## Verification routes

Selected: fast, heavy, all, all-feature, Windows CPU/native Rust/Python/Node,
and packaged graph/search smokes. GPU/CUDA is selected only if implementation
changes dense/rerank dispatch; otherwise it is N/A because graph constraint
parity is device-independent. Operator, live-model, and pre-publication
registry-installed are N/A.

## Draft-to-ready and delivery

Define constraint-before-truncation, direction/kind matrix, deterministic
one-page result, compact graph origin, fallback, parity, Windows, and conditional
CUDA criteria; design
against existing BFS/graph-arm contracts; review; implement RED/GREEN matrix
tests; review; verify; and record status. Stop on ignored constraints,
unbounded expansion, or reintroduction of rejected exact-anchor treatment.
