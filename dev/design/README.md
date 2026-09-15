---
status: UNREVIEWED
---

# Design Docs

> **Current vs historical (2026-06-26 ledger-prune).** The **live** design surface is the
> cross-cutting *topic* specs (`engine.md`, `retrieval.md`, `vector.md`, `op-store.md`,
> `migrations.md`, `errors.md`, `embedder.md`, `lifecycle.md`, `scheduler.md`, `recovery.md`,
> `projections.md`, `bindings.md`, `orchestration.md`, `perf-gates.md`, `ann-index-vec0.md`,
> `release.md`, `worktree-branch-consolidation.md`, `worktree-branch-consolidator.md`,
> `worktree-branch-consolidator-requirements.md`,
> `worktree-branch-consolidator-acceptance-criteria.md`) plus the live experiment decision-tree (`0.8.x-portfolio-features-and-experiment-tree.md`,
> `0.8.x-parity-portfolio-strategy.md`) and the in-flight `0.8.5-*` slice design.
> The per-slice memos (`slice-*-design.md`, `0.6.x/0.7.x/0.8.0–0.8.4-*`) are **historical
> records of closed slices — may be STALE**; their results are distilled in
> `dev/experiments-ledger.md` and their decisions live in `dev/adr/`. Frozen pre-registrations
> (`0.8.2-m1-multihop-harness.md`, `0.8.3-mem0-parity.md`) and `ir-recall-measure.md` are kept
> as load-bearing REFERENCE. See `dev/DOC-INDEX.md` for the authoritative per-doc map.

## Purpose

Detailed subsystem design documents that elaborate on accepted requirements and
ADRs.

## Keep here

- engine, retrieval, scheduler, vector, recovery, and release design docs
- cross-cutting design constraints

## Do not keep here

- public user/operator guidance
- architectural decision records
- disposable ideation notes

## Canonicality

Canonical for internal subsystem design, subject to ADR and requirement owners.

The active architecture is the reconciled 0.8.26 v2.2 profile in
[`fathomdb-data-plane-architecture-v2.md`](fathomdb-data-plane-architecture-v2.md).
Its 0.8.25 feature-level design records and predecessor dispositions are indexed by
[`../plans/0.8.25/design-documentation-matrix.md`](../plans/0.8.25/design-documentation-matrix.md).
The maintained interface documents remain section-status-qualified contract
owners. Slice 46 owns the broader classification and reconciliation of this
directory; this README remains `UNREVIEWED` until that work completes.

## Lifecycle

Living until design freeze; update when contracts materially change.
