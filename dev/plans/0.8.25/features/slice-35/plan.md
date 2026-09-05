---
title: 0.8.25 Slice 35 — eligibility and optional frozen reads
status: COMPLETE_ON_RELEASE_BRANCH
depends_on: 30
design: design.md
design_status: READY
---

# Slice 35 plan

## Outcome and carried obligations

Implement the eligibility-first core of R25/AC25-35; Memex needs 7/8 and the
contract half of 21; and A25-01/A25-05/A25-06 under the approved
[scope adjustment](../../scope-adjustment-2026-09-02.md). Apply indexed,
allowlisted eligibility before lexical, vector, or graph truncation and offer a
compact Engine-minted frozen read context when requested. Ordinary reads do
not require it; full lease/retention machinery is allocated to 0.8.27.

## Verification routes

Selected: fast, heavy, all, applicable combined feature families (not the
mutually exclusive CUDA-plus-Metal `--all-features` aggregate), Windows
CPU/native Rust/Python/Node, GPU/CUDA because dense eligibility is affected,
and isolated wheel plus offline npm pack/install search smokes.
Operator, live-model, and pre-publication registry-installed are N/A.

## Draft-to-ready and delivery

The implementation-ready design defines the exact retained grammar, per-arm
lowering, stateless token, monotonic visibility generation, failure precedence,
race linearization, public methods, and executable TDD matrix. Complete up to
four independent design-review FIX cycles; then implement real-database and
property RED tests before GREEN code, complete up to seven code-review FIX
cycles, independently verify, and write `status.md`. Stop on post-truncation
filtering, a mutable visibility path missing from the generation, or ambiguous
snapshot boundaries.

## Closeout

Complete on `release/0.8.25`. The durable outcome and verification boundaries
are recorded in [`status.md`](status.md). Slice 40 is next.
