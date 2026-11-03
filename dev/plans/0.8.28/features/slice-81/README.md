---
title: FathomDB 0.8.28 Slice 81 — erasure availability POC
status: IMPLEMENTED_PENDING_GATE
---

# Slice 81 — erasure availability POC

Determine whether deferred, durable physical cleanup can preserve foreground
availability and support a defensible deletion SLA. Produce a mechanism decision
or a bounded negative result, not a production erasure implementation.

| Document | Purpose |
| --- | --- |
| [Plan](plan.md) | Placement, reconciliation, scope, work order and review gates |
| [Requirements and acceptance](requirements.md) | Separate POC and experimental obligations; traceability |
| [Experiment design](experiment-design.md) | Hypotheses, fixtures, controls, budgets, measurements and decision rules |
| [Test plan](test-plan.md) | RED oracles, races, crash/reopen, physical proofs and independent verification |
| [Implementation design](design.md) | Candidate mechanisms, state ownership, Engine seams and limitations |
| [Status](status.md) | Execution state, final-gate status and evidence index |

Measured outcome: 34 complete and 38 incomplete replicas; native availability
and deletion SLA remain unqualified. See [results](../../../../experiments/erasure-availability/RESULTS.md).

## References

| Ref | Authority/evidence |
| --- | --- |
| S81-1 | Owner 2026-11-03: place the erasure-availability POC in new Slice 81 and write this planning package. This authorizes placement/planning, not production remediation or an unbounded campaign. |
| S81-2 | [Decision sequence](../../../../plans/current-history-followup-sequence.md): decision 3 approved maintenance POC; decision 1 production approach requires approval; physical erasure may be lazy with guaranteed completion/SLA. |
| S81-3 | [DBMS research](../../../../notes/dbms-erasure-availability.md): primary sources, deferred cleanup, SLA conditions and availability constraints. |
| S81-4 | [Follow-up results](../../../../experiments/current-history-search/followups/RESULTS.md): ordinary-Engine FTS ghost witness; candidate secure refresh 4.6 s; exact scope/acceptance limitations. |
| S81-5 | [Existing tools](../../../../experiments/current-history-search/followups/README.md): rerunnable erasure probe, current collector and measurement fixtures. Preserve their prior results. |
| S81-6 | [Dependency closure ADR](../../../../adr/ADR-0.8.25-dependency-lifecycle-closure.md), [offline completion ADR](../../../../adr/ADR-0.8.27-offline-physical-erasure-completion.md), [Rust contract](../../../../interfaces/rust.md): global pending-physical write fence, keyed retry and physical-success meanings remain shipped authority. |
| S81-7 | [Current/history candidate](../../../../design/current-history-search.md): corpus isolation, certificate/publication and erasure blast radius. |
| S81-8 | [Quality roadmap](../../../../notes/fathomdb-quality-testing-roadmap.md), [AGENTS.md](../../../../../AGENTS.md): real databases, RED/GREEN, boundary/fault tests, measured resource cleanup. |
| S81-9 | [FTS5 merge/optimize/secure-delete](https://www.sqlite.org/fts5.html#special_insert_commands), [core secure deletion](https://www.sqlite.org/pragma.html#pragma_secure_delete), [checkpoint](https://www.sqlite.org/c3ref/wal_checkpoint_v2.html), [interruption](https://www.sqlite.org/c3ref/interrupt.html): upstream mechanism boundaries, not deletion SLAs. |
| S81-10 | [Store format 35](../../../../adr/ADR-0.8.28-store-format-35.md): real bundled SQLite/layout, current-format-only admission, no migration. |
| S81-11 | [Release plan](../../../plan-0.8.28.md), [release state](../../../release-state-0.8.28.json), [Slice 80 status](../slice-80/status.md): 80 is complete; 81 precedes existing 90. |

New test/harness paths named in the design are planned artifacts, not existing
implementation evidence. No new canonical AC identifiers are introduced.
