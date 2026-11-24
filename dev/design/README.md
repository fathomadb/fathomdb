---
status: ACTIVE
---

# Design documents

This directory holds current subsystem designs, bounded references and
experiments, proposals, and historical design records. The exact lifecycle
classification and current owner for every Markdown document is
[`document-lifecycle.json`](document-lifecycle.json), paired with semantic
authority and implementation witnesses in
[`current-owner-authority.json`](current-owner-authority.json). Both catalogs
are enforced by `scripts/check-design-lifecycle.py` in the local Markdown gate
and docs CI.

The active architecture profile is
[`fathomdb-data-plane-architecture-v2.md`](fathomdb-data-plane-architecture-v2.md).
For changes spanning multiple crates, language bindings, conformance manifests,
or repository gates, start with
[`cross-cutting-change-checklist.md`](cross-cutting-change-checklist.md).
Public contracts remain owned by `dev/interfaces/` and accepted decisions by
`dev/adr/`; those authorities override explanatory design prose.

## Live subsystem owner map (`"class": "maintained"`)

All 27 maintained design documents in [`document-lifecycle.json`](document-lifecycle.json):

| Subsystem / topic | Role | Maintained owner | Scope summary |
| --- | --- | --- | --- |
| Architecture (`data-plane`) | `architecture` | [`fathomdb-data-plane-architecture-v2.md`](fathomdb-data-plane-architecture-v2.md) | Current end-to-end data-plane architecture authority |
| Design index (`design-documentation`) | `index` | [`README.md`](README.md) | Directory lifecycle policy and live subsystem owner map |
| Cross-cutting changes (`cross-cutting-changes`) | `method` | [`cross-cutting-change-checklist.md`](cross-cutting-change-checklist.md) | Operational checklist across schema, SDK parity, MCP, module boundaries, and generated docs |
| Engine (`engine`) | `design` | [`engine.md`](engine.md) | Core engine runtime, write/read paths, and v1 boundary |
| Actuation (`actuation`) | `design` | [`actuation.md`](actuation.md) | Bounded atomic actuation semantics |
| Lifecycle (`lifecycle-observability`) | `design` | [`lifecycle.md`](lifecycle.md) | Record lifecycle transitions and observability |
| Operational store (`op-store`) | `design` | [`op-store.md`](op-store.md) | Same-file operational state collections and mutations |
| Migrations (`migrations`) | `design` | [`migrations.md`](migrations.md) | Schema versioning and fresh-database boundary policy |
| Errors (`errors`) | `design` | [`errors.md`](errors.md) | Engine and SDK typed error taxonomy |
| Retrieval (`retrieval`) | `design` | [`retrieval.md`](retrieval.md) | Eligibility, frozen reads, and ranked retrieval pipeline |
| Retrieval limits (`retrieval`) | `result-limits` | [`retrieval-result-limits.md`](retrieval-result-limits.md) | Result-limit bounds and truncation contracts |
| Projections (`projections`) | `design` | [`projections.md`](projections.md) | Eager projection runtime and commit visibility |
| Nested projections (`nested-source-projections`) | `design` | [`nested-source-projections.md`](nested-source-projections.md) | Declared literal member paths and projected scalar/text search |
| Scheduler (`scheduler`) | `design` | [`scheduler.md`](scheduler.md) | Engine-owned projection orchestration and scheduling |
| Vector (`vector`) | `design` | [`vector.md`](vector.md) | Same-file `vec0` storage and vector contracts |
| Embedder (`embedder`) | `design` | [`embedder.md`](embedder.md) | Embedder protocol, default embedder, and device policy |
| Bindings (`bindings`) | `design` | [`bindings.md`](bindings.md) | Rust, Python, and TypeScript binding architecture and packaging |
| Recovery (`recovery`) | `design` | [`recovery.md`](recovery.md) | CLI offline recovery and derived-maintenance boundaries |
| Projection rebuild (`recovery`) | `projection-generation` | [`recovery-0.8.25.md`](recovery-0.8.25.md) | Projection generation and rebuild semantics |
| Erasure contract (`explicit-maintenance-erasure`) | `contract` | [`explicit-maintenance-erasure/contract.md`](explicit-maintenance-erasure/contract.md) | Explicit erasure maintenance behavioral contract |
| Erasure design (`explicit-maintenance-erasure`) | `production-design` | [`explicit-maintenance-erasure/production-design.md`](explicit-maintenance-erasure/production-design.md) | Erasure queue, exclusive windows, and physical completion design |
| Performance gates (`performance-gates`) | `design` | [`perf-gates.md`](perf-gates.md) | Retrieval latency and throughput gate design |
| Performance regression (`performance-regression`) | `design` | [`perf-regression-detection.md`](perf-regression-detection.md) | Regression detection methodology and CLI harness |
| GPU evaluation (`gpu-evaluation`) | `policy` | [`gpu-eval-activities-policy.md`](gpu-eval-activities-policy.md) | GPU selection and driverless CUDA verification policy |
| Release (`release`) | `design` | [`release.md`](release.md) | Multi-registry release packaging, smoke, and publication flow |
| Dependency pin rot (`pinned-override`) | `policy` | [`pinned-override-rot-guard.md`](pinned-override-rot-guard.md) | Guardrail preventing stale workspace/package dependency overrides |
| Rust workspace gate (`temporary-serial-rust-workspace-release-gate`) | `gate-design` | [`temporary-serial-rust-workspace-release-gate.md`](temporary-serial-rust-workspace-release-gate.md) | Serialized workspace test gate design |

## Lifecycle rules

- `maintained` is a current topic/role owner and must stay consistent with code,
  interfaces, and accepted ADRs.
- `reference` and `experiment` remain useful inputs but do not own current
  product behavior.
- `proposal` and `deferred` describe unshipped work; they must not be read as
  current behavior.
- `historical` records a closed slice or superseded release context (for
  example, [`orchestration.md`](orchestration.md) for pre-0.8.31 runs).
- `superseded` must name its current successor, which is also its owner.

Adding, removing, moving, or reclassifying a design document requires updating
[`document-lifecycle.json`](document-lifecycle.json) and
[`current-owner-authority.json`](current-owner-authority.json) in lockstep in
the same change. Historical records are preserved; a new current owner
supersedes them without rewriting their original decisions.

## Placement

Keep internal subsystem and cross-cutting design here. Put public operator or
SDK guidance under `docs/`, accepted decisions under `dev/adr/`, and disposable
ideation outside the maintained design surface.
