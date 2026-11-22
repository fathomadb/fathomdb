---
title: Inspector 0.2.0 Slice 10 — conditional change design
date: 2026-11-19
status: APPROVED
execution: COMPLETE
desc: Per-approved-item execution contracts with no predetermined implementation bundle
---

# Conditional design

Execution commissioned separately on 2026-11-19; [current status](status.md)
records completed owned work and the owner-approved external metadata correction. The ratified planning text below
records authority and original not-yet-executed conditions; it does not override
the later commission or imply a new ratification gate.

## Authority and baseline

This design refines the [Slice 10 plan](plan.md), [adopted scope](../scope.md)
and [HLD](../../../design/high-level-design.md). At the read-only planning
baseline `609dd85c41fb7980e05faa1132a248d376d8ec89`, no individual Slice 9
implementation decisions existed. Actual responses 1/2 now include S9-004–007 and 010;
the plan records their exact paths/acceptance. USER final response 3 on 2026-11-19
ratified S10-DRAFT-2 at 2604013b736ad655ac5dfa0aa32c1b67208ec63b, including
001/002/004–008/010, postponing 003/011–014 and retaining 009 in 40. Its approved
local change-plan specifies the
single-head-window scope/AC/method/roadmap amendment; no multiwindow positive
blocker is carried after that approved application. Cache-only Engine serving
stays mandatory/unqualified; no implementation is performed. Preparation
must enumerate source, plan and allocated-item drift, then keep/adjust/reject or
recognize already-completed items against actual evidence before execution.

The implemented [worker](../../../../../src/inspector/src/worker.rs) has record
operations, conditional narrow export and close. [Package wiring](../../../../../src/inspector/Cargo.toml)
is unpublished 0.1.0 with Engine defaults off; [packaging](../../../../../src/inspector/tools/package.sh)
builds that existing artifact. Text/vector/interactive graph and comparison
remain new feature work at 30/40/50/60, with 20 foundation and 70 integration.
Historical HLD allocation numbers do not supersede the current master mapping.

## Per-item change boundary

The sole writer derives a bounded implementation contract from each actual
Slice 9 owner decision: exact proposed outcome, allowed paths/actions, Inspector
owner, dependencies, existing INS/V01/X/fixture trace, independent observable
acceptance, design delta, checks and rollback. An unspecified item is not
executable. Approval of one item never imports neighboring recommendations.
A changed outcome returns for owner disposition; technical reviewers resolve
implementation details but cannot change scope or waive positive acceptance.

No new need/AC or architecture mechanism follows merely from this generic
executor. Needs/requirements/AC amendments stay LOCAL DRAFT until approved and
then update their Inspector authority once. The locked global register is
untouched. Source evidence outside Inspector may establish an external blocker;
it cannot authorize Engine/SDK/MCP/schema/ranker/API or public pin changes.

## Genuine feasibility constraints and fit

Preserve the existing admission/storage/query/policy/observation/resource/UI/
package boundaries. Approved refactoring must identify an actual defect or
approved outcome and demonstrate preserved public behavior, not create a
parallel reader/ranker or framework. Current-format reads need actual layout and
lifecycle facts; ordinary compiled schema equality alone does not qualify them.

[Production admission](../../../../../src/rust/crates/fathomdb-engine/src/open.rs)
and the [accepted provider ADR](../../../../adr/ADR-0.8.28-vector-compatibility-open-policy.md)
contradict the legacy HLD helper-to-Caller route: Default/None are production,
Caller test/evaluation only. Investigate existing public supported cache-only
admission/query paths at the assigned 9/40 gate; do not silently replace the
required semantic positive with unavailable or claim impossibility from Caller
removal. New snippet methods similarly require source/cfg qualification rather
than the HLD's historical absence claim. Feature designers own those designs.

If an approved item requires web evidence during execution, first state the
question and primary-source approach: pinned official library release notes/API
contracts for a disputed capability; official packaging/runtime documentation
for build compatibility; official browser documentation for a changed interaction.
No research runs now. Research findings propose choices; they do not approve an
upgrade, global setup, driver change or external implementation. Slice 0's formal
environment/CPU-only CUDA review remains authoritative without changing support.

## Verification, rollback and reviewer readiness

Docs-only items verify the exact corrected claims, links and scoped Markdown/
docs gates; no artificial RED/GREEN. Behavioral items have independent real-DB
or contract assertions with per-behavior failing RED commit then separate GREEN,
Sol-high design/code review and Terra execution in the owned environment.
Use existing harnesses, bounded fault/cancel/resource and browser/keyboard cases
where affected; human acceptance stays human. Keep reusable measurement tools
committed and indexed. Do not manufacture goldens or copy expected output from
the implementation. Broad engine fuzzing/MC/DC campaigns are not commissioned.

Prerequisites for reviewers: actual item rulings and dependencies; current source
and callable/test-only classification; exact acceptance/test commands; approved
release-checkout baseline from Slice 0; affected docs/interfaces; finite rollback plan.
Rollback restores only item-owned state or normally reverts dedicated commits,
reversing affected dependents first and checking the blast radius. Never migrate
product data, reset unrelated work, clean shared caches or publish a rollback.

The [owner ruling](../slice-0/checkout-decision.md) assigned Slice 0 to
create and enter the release checkout; that action is complete at the approved
final planning HEAD,
preserving Engine ancestry and Inspector port/scope/plans. Reports for 0–9 use
it; 10 reuses it with no initial bootstrap action. No additional checkout creation or Slice 10 execution is authorized by final
ratification. The actual response 3 [ruling](../slice-9/plan.md#final-human-ratification--response-3)
is settled; do not ask again for unchanged approved scope. An approved empty set changes no proposed product/environment state;
only concise status/handoff still occur. Mandatory environment/feature/installed/human
blockers survive no-op closure and pass to their named 20–70 owners.
