---
title: 0.8.25 Slice 20 — core dependency registration
status: DRAFT
depends_on: 15
design: design.md
design_status: REVIEWED_MAX_ENVELOPE_SCOPE_RECONCILIATION_REQUIRED
---

# Slice 20 plan

## Outcome and carried obligations

Implement the core subset of R25/AC25-20 and Memex need 4 under the approved
[scope adjustment](../../scope-adjustment-2026-09-02.md). Add queryable
canonical-source-to-derived dependency identity, bounded forward/reverse
lookup, structural validation, and cycle rejection. Multi-source sets,
general derived-to-derived dependency graphs, and configurable liveness are
allocated to 0.8.26. The Engine enforces structure, never semantic truth.

## Verification routes

Selected: fast, heavy, all, all-feature, Windows CPU/native Rust/Python/Node,
and packaged public-contract smokes. Operator, GPU/CUDA, live-model, and
pre-publication registry-installed are N/A unless the design changes an
operator surface.

## Draft-to-ready and delivery

Define reference, cycle, lookup, source-removal, codec, parity, and Windows
criteria; design core dependency persistence and bounded lookup; review;
implement RED/GREEN property and real-database tests; review; verify selected
routes; and record status. Stop on unbounded lookup, shadow-index ownership,
or ambiguous source-removal consequences.
