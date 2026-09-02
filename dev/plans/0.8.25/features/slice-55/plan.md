---
title: 0.8.25 Slice 55 — basic tracing and integrity
status: DRAFT
depends_on: 50
design: design.md
design_status: REVIEWED_MAX_ENVELOPE_SCOPE_RECONCILIATION_REQUIRED
---

# Slice 55 plan

## Outcome and carried obligations

Implement the basic subset of R25/AC25-55; Memex needs 19/20 and its share of
12; and A25-05/A25-06 under the approved
[scope adjustment](../../scope-adjustment-2026-09-02.md). Add bounded
reciprocal provenance tracing, dependency-orphan/projection checks, and compact
inclusion/degradation explanation without exposing private content. Persisted
trace pages and expanded exclusion tracing are deferred; frozen integrity jobs
and sophisticated repair orchestration remain experimental.

## Verification routes

Selected: fast, heavy, all, all-feature/operator, Windows CPU/native
Rust/Python/Node, and packaged explanation/operator smokes. GPU/CUDA,
live-model, and pre-publication registry-installed are N/A.

## Draft-to-ready and delivery

Define reciprocal trace, compact inclusion/degradation reasons, privacy,
injected orphan/projection fault, codec, parity, and Windows criteria; design
bounded one-request traversal and checks; review; implement RED/GREEN
fault/property tests; review; verify; and record status. Stop on unbounded tracing, semantic
correctness claims, or content-bearing telemetry.
