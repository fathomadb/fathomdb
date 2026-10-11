---
title: FathomDB 0.8.27 prework Slice 4 - architecture and code alignment
status: COMPLETE
target_release: 0.8.27
observed_on: 2026-09-21
---

# Slice 4 - architecture and code alignment

## Plan and delta reconciliation

Outcome: ground Slice 3 in the shipped substrate and distinguish existing
modules from net-new moves. The old 17,910-line map is rejected as an execution
map; it remains historical evidence pinned to `ff4f07a0`.

## Requirements and acceptance

| ID | Requirement | Acceptance signal |
| --- | --- | --- |
| PW27-4A | Structural work reuses shipped semantic modules and preserves accepted topology. | Every Slice 40-90 map names existing destinations first; no parallel subsystem, field-visibility widening, or new crate appears. |
| PW27-4B | Refactor proof covers the feature routes actually affected. | Slice plans enumerate relevant default/operator/test-hooks, migration-test-hooks, benchmark, embedder/reranker, CUDA/Metal, and slice-specific routes without using conflicting `--all-features`. |
| PW27-4C | Public roots and writer/transaction authority remain unchanged. | Surface comparator and focused ordering/locking tests pass after each move. |

## Exists today versus net-new

Exists today:

- one engine crate with root-defined `Engine` and root exports;
- ten substantial semantic modules: `actuation`, `data_plane_integrity`,
  `dependency_closure`, `dependency_trace`, `evidence`, `frozen_read`,
  `graph_expand`, `lifecycle`, `pagination`, and `projection_generation`;
- TypeScript `binding`, `errors`, `evidence-validation`, `platform`, `read`, and
  `validation` modules; and
- Python `admin`, `config`, `errors`, `filter`, `graph`, `read`, and `types`
  modules.

Net-new work is relocation of remaining root-owned code, correction of
dependencies that currently point back to monolithic roots, a complete public-
surface comparison instrument, and focused tests needed to protect newly
isolated boundaries. It is not a new engine, public facade, SDK, wire protocol,
schema, or transaction model.

Current source sizes are 34,184 engine Rust lines, 5,943 PyO3 lines, 5,406 NAPI
lines, 4,439 TypeScript SDK lines, and 2,846 Python SDK lines. These figures
justify re-anchoring but are advisory; they are not correctness thresholds.

Accepted constraints:

- preserve the monolithic engine crate and root `Engine` from
  `ADR-0.6.0-crate-topology.md`;
- preserve the single-writer/locking model from
  `ADR-0.6.0-single-writer-thread.md`;
- child modules may host inherent `impl Engine` blocks because they can access
  ancestor-private fields; sibling module designs must not widen fields to
  compensate for a wrong tree; and
- Slice 40-90 must reuse existing modules and move in 300-1,200-line batches,
  with the affected feature set stated per batch.

F27-01 remains compositional and implementation-neutral. The current evidence
proves supersession/correction and erasure independently but does not yet prove
the production root cause. No specific fix is pre-authorized.

## Implementation, review, verification, and status

Implementation is this exists-vs-net-new record only. TDD and code review are
not applicable. An independent code-grounded audit checked the current files,
ADRs, module inventory, and feature surfaces. Independent package design
review and closeout verification passed.

Status is `COMPLETE`. No architecture authority or code changed. Next: Slice 5
verification adequacy.
