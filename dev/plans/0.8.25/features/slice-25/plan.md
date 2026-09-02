---
title: 0.8.25 Slice 25 — atomic semantic actuation
status: DRAFT
depends_on: 20
design: design.md
design_status: REVIEWED_MAX_ENVELOPE_SCOPE_RECONCILIATION_REQUIRED
---

# Slice 25 plan

## Outcome and carried obligations

Implement the core subset of R25/AC25-25 and Memex needs 3/17/18 under the
approved [scope adjustment](../../scope-adjustment-2026-09-02.md). Add one
bounded typed, model-free, idempotent batch for caller-decided canonical and
derived records, core dependencies, and lifecycle actions. Return a compact
committed or whole-batch-refused receipt with operation identity, affected
IDs, resulting boundary, and readiness/closure references. Broader operation
coverage and exhaustive consequence receipts are allocated to 0.8.26.

## Verification routes

Selected: fast, heavy, all, all-feature, Windows CPU/native Rust/Python/Node,
and packaged batch/receipt smokes. Operator is selected if lifecycle verbs use
that feature. GPU/CUDA, live-model, and pre-publication registry-installed are
N/A.

## Draft-to-ready and delivery

Specify atomicity, boundedness, idempotency, compact receipt, whole-batch
refusal, codec, parity, and Windows criteria; design transaction and receipt
semantics; review; preserve RED fault-injection/replay tests through GREEN;
review; verify selected routes; and record status. Stop on partial commit,
semantic decision-making by FathomDB, or receipt ambiguity.
