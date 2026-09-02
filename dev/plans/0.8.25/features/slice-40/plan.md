---
title: 0.8.25 Slice 40 — projection generation and readiness
status: DRAFT
depends_on: 35
design: design.md
design_status: SCOPE_RECONCILED_FORMAL_REVIEW_REQUIRED
---

# Slice 40 plan

## Outcome and carried obligations

Implement the core of R25/AC25-40, Memex need 13, and A25-05 under the approved
[scope adjustment](../../scope-adjustment-2026-09-02.md). Add durable
projection-generation identity, prevent false readiness, advance safely across
restart, and expose compact mutation-to-ready correlation. A richer public
work-manifest surface is not required in 0.8.25.

## Verification routes

Selected: fast, heavy, all, all-feature/operator, Windows CPU/native
Rust/Python/Node, GPU/CUDA for dense readiness, and packaged status smokes.
Live-model and pre-publication registry-installed are N/A; local model
acquisition/readiness is an environment preflight, not a live-model route.

## Draft-to-ready and delivery

Specify generation identity, wrong-generation, restart, mutation-to-ready,
typed readiness, parity, Windows, and CUDA criteria; design persistence and
correlation; review; implement preserved RED/GREEN property and failure tests;
review; verify; and record status. Stop on generation reuse, false readiness,
or application-owned projection cleanup.
