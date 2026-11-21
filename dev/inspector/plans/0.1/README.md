---
title: Inspector legacy slice allocation
date: 2026-10-28
product: FathomDB Inspector
target_release: 0.2.0
status: PROPOSED
desc: Seven-slice ladder with release-wide preparation and five mod-10 features
---

# Inspector legacy slice allocation

These paths retain the original v0.1 draft allocation and source history.
[Adopted 0.2.0](../0.2.0/scope.md) carries unfinished 30/40/50/60 and remaining
foundational qualification, plus comparison INS-05.7–05.10/M40 under owner 60.
The [single verification matrix](verification-plan.md) remains authoritative;
no source version bump, status completion or execution commission follows. The
[current 0.2.0 index](../0.2.0/README.md), [master plan](../0.2.0/plan.md) and
[workflow](../0.2.0/slice-workflow.md) own the proposed commission ladder.
Legacy 60 splits into new 60 comparison/interactions and new 70 final
installed/integrated qualification; historical statuses below are unchanged.

Two release-wide preparation slices precede five feature slices:
**15 → 16 → 20 → 30 → 40 → 50 → 60**. The five mod-10 feature allocations stay
intact; the owner added 15 and 16 as planning/design prerequisites.

| Slice | Feature | Plan | High-level draft design | Depends on |
| --- | --- | --- | --- | --- |
| 15 | Release-wide requirements/AC and verification planning | [Plan](slice-15/plan.md) | [Guidance](slice-15/design.md) | — |
| 16 | Release-wide architecture/design and evolution through FathomDB 1.x | [Plan](slice-16/plan.md) | [Guidance](slice-16/design.md) | 15 |
| 20 | Foundations and record inspection | [Plan](slice-20/plan.md) | [Design](slice-20/design.md) | 15, 16 |
| 30 | FTS/BM25 inspection and lexical retrieval | [Plan](slice-30/plan.md) | [Design](slice-30/design.md) | 15, 16, 20 |
| 40 | Vector representations and similarity neighborhoods | [Plan](slice-40/plan.md) | [Design](slice-40/design.md) | 15, 16, 20; deliver after 30 |
| 50 | Graph, provenance and retained revisions | [Plan](slice-50/plan.md) | [Design](slice-50/design.md) | 15, 16, 20, 30, 40 |
| 60 | Integrated investigations and carried qualification | [Plan](slice-60/plan.md) | [Design](slice-60/design.md) | 15, 16, 20–50 |

Execute sequentially by default in the existing task checkout. The
[common workflow](slice-workflow.md) binds every plan, including change
reconciliation, complete req/AC/design, Sol-high design and code reviews,
per-behavior RED/GREEN, Terra verification, lean status and cleanup. Agents
are commissioned directly, not through MCP; no Steward/Orchestrator is used.

Feature drafts start from `037a8522e`; Slice 15/16 drafts start from
`d05912c99`. Slice 15 has prepared [S15-2026-10-30](verification-plan.md); its
[status](slice-15/status.md) records completion and executed document checks. Slice 16 and record-foundation execution have their own historical statuses;
remaining feature/integrated work remains unexecuted. Preparation slices finish documents,
traceability and reviewed handoffs; only feature slices implement runnable UI.
The shared verification output records planned methods, not application passes. At execution, create one concise `status.md` in the slice directory. No review transcript, giant
receipt bundle or extra worktree is required. The [master plan](../plan-0.1.md)
owns the all-scope coverage map and open product decisions.

The boundary is Inspector-only. Existing Engine/MCP/SDK/schema/agent-access
code is read-only reference/dependency material. Missing capabilities must
be surfaced without silently weakening the required FathomDB-native GUI.

## Slice 16 execution baseline

[S16-2026-10-31](slice-16/status.md) selects architecture in the
[canonical HLD](../../design/high-level-design.md); feature drafts 20–60 consume
its components, contracts, capability evidence and M01–M39 mapping. B16-01 is
resolved by the 2026-10-31 owner ruling: physical deletion remains correct;
retained-erasure content/export is conditional on legitimate existence, with
selected Engine absence explicitly qualified. The status
owns gate/completion evidence. Application/runtime qualification remains planned.
