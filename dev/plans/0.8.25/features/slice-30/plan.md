---
title: 0.8.25 Slice 30 — lifecycle and erasure closure
status: DRAFT
depends_on: 25
design: design.md
design_status: REVIEWED_MAX_ENVELOPE_SCOPE_RECONCILIATION_REQUIRED
---

# Slice 30 plan

## Outcome and carried obligations

Implement the core subset of R25/AC25-30; Memex needs 5/6; and A25-05 under the
approved [scope adjustment](../../scope-adjustment-2026-09-02.md). Propagate
lifecycle and erasure through the Slice 20 canonical-source-to-derived
dependency form, fence incomplete work, support idempotent restart/resume, and
prove no active or searchable orphan. A25-04 multi-source liveness moves to
0.8.26 with that dependency form.

## Verification routes

Selected: fast, heavy, all, all-feature/operator, Windows CPU/native
Rust/Python/Node, and packaged lifecycle smokes. GPU/CUDA, live-model, and
pre-publication registry-installed are N/A.

## Draft-to-ready and delivery

Define the core transition/dependency matrix, crash/restart, fencing,
projection/WAL, erasure receipt, parity, and Windows criteria; design
transactional and resumable closure; review; preserve RED injected-failure/
orphan tests through GREEN; review; verify selected routes; and record status.
Stop on searchable orphans, unverifiable erasure, or non-idempotent recovery.
