---
title: 0.8.25 Slice 40 — projection generation and readiness
status: DRAFT
depends_on: 35
---

# Slice 40 plan

## Outcome and carried obligations

Implement R25/AC25-40; Memex need 13; and A25-05. Add durable projection
generation identity and correlate mutations with ready, degraded, blocked, and
deferred projection work across restart.

## Verification routes

Selected: fast, heavy, all, all-feature/operator, Windows CPU/native
Rust/Python/Node, GPU/CUDA for dense readiness, and registry-installed status
smokes. Live-model is N/A unless readiness proof requires model acquisition.

## Draft-to-ready and delivery

Specify generation identity, wrong-generation, restart, mutation-to-ready,
typed readiness, parity, Windows, and CUDA criteria; design persistence and
correlation; review; implement preserved RED/GREEN property and failure tests;
review; verify; and record status. Stop on generation reuse, false readiness,
or application-owned projection cleanup.
