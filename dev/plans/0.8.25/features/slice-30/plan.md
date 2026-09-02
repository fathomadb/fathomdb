---
title: 0.8.25 Slice 30 — lifecycle and erasure closure
status: DRAFT
depends_on: 25
design: design.md
design_status: REVIEWED_BLOCKED_ON_SLICE_7
---

# Slice 30 plan

## Outcome and carried obligations

Implement R25/AC25-30; Memex needs 5 and 6; and A25-04 and A25-05. Propagate
lifecycle and erasure through registered dependencies, fence incomplete work,
support idempotent restart/resume, and prove no active or searchable orphan.

## Verification routes

Selected: fast, heavy, all, all-feature/operator, Windows CPU/native
Rust/Python/Node, and registry-installed lifecycle smokes. GPU/CUDA and
live-model are N/A.

## Draft-to-ready and delivery

Define the transition/liveness matrix, crash/restart, fencing, projection/WAL,
erasure receipt, parity, and Windows criteria; design transactional and
resumable closure; review; preserve RED injected-failure/orphan tests through
GREEN; review; verify selected routes; and record status. Stop on searchable
orphans, unverifiable erasure, or non-idempotent recovery.
