---
title: Inspector 0.2.0 Slice 50 — draft plan
date: 2026-10-28
product: FathomDB Inspector
target_release: 0.2.0
status: PROPOSED
desc: Graph, provenance and retained revisions
---

# Slice 50 — Graph, provenance and retained revisions

HISTORICAL INPUT ONLY. The [current Slice 50 draft](../../0.2.0/slice-50/plan.md) and
[design](../../0.2.0/slice-50/design.md)
under the [current master](../../0.2.0/plan.md) and
[workflow](../../0.2.0/slice-workflow.md) replace this future delivery
draft as the proposed commission authority. The adopted
[scope](../../0.2.0/scope.md) carries every existing Need/Should/INS/V01/X/F/T
obligation and resource/qualification target. Original baselines, instructions
and slice labels below are retained as historical reuse inputs, not active
execution instructions or runtime evidence. Comparison INS-05.7–05.10/M40
belongs to new 60; final installed/integrated qualification belongs to new 70.

## Draft and feature scope

Draft baseline: `037a8522e`. State: DRAFT, not executed. Dependencies:
15, 16, 20, 30, 40. Reconcile from this baseline before execution; later commits do
not make this draft automatically current.

Interactive FathomDB graph navigation with edge detail, class-correct lifecycle, bounded provenance/dependency tracing and retained canonical revision navigation.

Allocated scope: INS-03 graph relationships; INS-04 graph participation; INS-05 cross-view identities; INS-06 class-specific eligibility; INS-07; INS-08 graph/trace evidence; INS-09.4/5/6; V01-N03 and completion of N01/N02; graph/provenance V01-S04; INS-X01–X11.

- Deliver directed node/edge payloads and an interactive bounded graph canvas: select, filter, expand/collapse, inspect edge content/endpoints, and keyboard-accessible synchronized list/detail.
- Keep graph participation, source attribution, registered dependency, projection origin and supersession as distinct relationship types.
- Deliver retained revision-chain and bounded dependency/source navigation, including incomplete/unavailable targets and policy-governed historical data.
- Apply node and edge lifecycle/temporal rules separately and preserve exact identities when navigating from text/vector observations into the graph.

Exclusions: No graph inference from embedding proximity/text similarity, new engine graph algorithm, repair/reactivation, erased-content reconstruction, operational latest-state history fabrication or global full-graph loading.

Owner delta, 2026-10-29: Engine startup is permitted when required. Follow
[the current attachment/effect contract](../../../requirements.md#engine-startup-allowance-2026-10-29).
External writers remain stopped; Engine-owned workers may progress. Qualify
normal Engine effects and per-read context rather than promise byte preservation.
Cache-only/no-download, exact identity and Inspector-only source scope remain.

## Release-wide prerequisites

Complete [Slice 15](../slice-15/plan.md) and [Slice 16](../slice-16/plan.md)
before this feature's implementation. Consume their recorded requirements/AC,
shared verification matrix, architecture/capability decisions and assigned
fixture/oracle/test obligations. Reconcile the draft against those baselines;
refine feature-local design/tests rather than repeat central planning. Feed
shared changes back to canonical documents/matrix and affected owners. Feature
correctness and tests remain slice-complete; Slice 60 is additional qualification.

## Required execution sequence

The [common slice workflow](../slice-workflow.md) is binding in full: reconcile
changes → update needs/req+AC → finish/review design → implement RED/GREEN →
Sol code review → Terra verification → status/integration/cleanup. Execute this
slice after planning it; a plan-only result is not slice completion.

1. Enumerate changes to this draft, related in-repo work, assigned features/
   functions and allocated findings/items since `037a8522e`. Read predecessor
   statuses and inspect actual implementation, not just documentation.
2. Evaluate each delta and keep/approve, adjust, reject or mark already done
   with a short evidence/reason row in this plan. Keep scope complete without
   speculative extension. An optional `change-plan.md` may guide a substantial
   revision; no duplicate decision paperwork is required.
3. Complete the slice's needs, requirements, acceptance and fixtures:
   Complete graph payload/traversal, trace/revision, disclosure and class-specific lifecycle contracts. Add directed multi-kind edges, node inactive state, edge temporal boundary, ambiguous identities and unavailable origins to F-01/F-02/F-04/F-07 as appropriate.
4. Expand [draft design](design.md) to the needed implementable contracts and
   have a directly spawned **gpt-6.1-sol-high** subagent review additions and
   changes. Resolve material findings before affected implementation.
5. Implement the feature using the steps below and per-behavior failing RED
   tests followed by separate GREEN/refactor work. Use real databases.
6. Have a fresh directly spawned **gpt-6.1-sol-high** code-review subagent inspect
   the implemented diff, then a direct **Terra** (`gpt-6.1-terra`) subagent
   independently test/verify the final revision and blast radius.
7. Update `status.md`, address material findings, integrate and clean up only
   extra slice-owned checkout/branch resources if any were created. Normally
   use the existing task checkout. Do not use Steward, Orchestrator or MCP.

## Planned implementation

- Resolve existing-interface versus Inspector-local read feasibility for exact edges/endpoints, retained revisions and dependencies before claiming a canvas adapter complete.
- RED/GREEN node/edge/trace/revision payloads and bounded expansion using independently seeded relationships and actual stored identities.
- Implement interactive canvas plus accessible list/detail, then origin/trace/revision navigation and class-correct state labels.
- Connect text/vector selections to graph records, implement exact historical observations, preserve unavailable/erased handling and finish slice-specific guides.

Keep all implementation under `src/inspector/` and its narrowly enumerated
Inspector test/build/docs wiring. Changes to Engine, MCP, SDK, schema and
agent-access code/contracts are outside this slice, even if earlier proposals
suggested shared refactoring. A genuine dependency gap is a blocker, not
permission to expand scope or downgrade acceptance.

## Testing and slice completion

- F-07 directed multi-kind edges, edge-owned content, repeated labels/different IDs, expansion caps/collapse and node selection without layout-based similarity claims.
- F-02 retained revisions and exact original observation, F-04 inactive node versus expired edge, missing/erased targets, policy scope and unknown provenance.
- Real browser/keyboard graph and trace/revision flows from earlier text/vector views; truncation, high-degree bounds, stale references and back navigation.

Run focused lint/type/behavior checks first. For an individual finding, rerun
the reproducer and affected/adjacent tests; full regressions require a broad
blast radius, unresolved risk or an explicit required final gate. Broader
Slice 60 qualification does not excuse missing tests or an unusable feature
here. Follow repository TDD, review, environment and retry rules.

**Completion:** Graph, provenance and retained revisions are usable and verified as vertical features; correct backend payloads without a working canvas or trace navigation do not close the slice.

The executor writes a short `status.md` with reconciled scope/revision,
requirements/design changes, Sol dispositions, RED/GREEN references, Terra
results, remaining gaps and cleanup. No status currently claims implementation
or verification. See the [master plan](../../plan-0.1.md) for full scope ownership.

## Slice 15 execution handoff — S15-2026-10-30

Consume [canonical acceptance](../../../acceptance.md),
[requirements/envelope](../../../requirements.md#verification-and-operating-envelope-slice-15-2026-10-30)
and [the single verification matrix](../verification-plan.md), source baseline
`9486c5ba9`. Check [Slice 15 status](../slice-15/status.md) before starting:
D-01 is adopted on 2026-10-30; implement its exact launch flags, independent
overlap gates and narrow retained-erasure export. This handoff does not waive
predecessor completion. Original draft baselines remain
for later delta reconciliation.

Assigned matrix rows (also M39): **M08–M11, M15, M17–M21, M27–M29, M31–M34, M38**. Extend F-01/02/04/07 with explicit directed/self/parallel/cyclic edges, full edge content, class-specific temporal/closure cases, source/dependency/supersession links and exact retained revisions. Deliver actual bounded canvas with keyboard select/expand/collapse; D-01 governs history/erasure reads.

This adjusts the draft by adding exact fixture/oracle/layer obligations,
INS-X07–X11 and explicit aggregate resource/security/accessibility/installed
contracts. It does not implement them. Preserve feature-local TDD and review;
replace planned methods with named tests/commands and executed evidence as the
feature is built. Required positive gaps block completion, even where an
unavailable/unsupported negative case works. No competing acceptance list or
Engine/MCP/SDK/schema/pin work is authorized.

D-01 final handoff: apply [the adopted policy](../../../requirements.md#adopted-operator-disclosure-policy--d-01-2026-10-30)
and matrix M20/M39. Slice 20 owns launch parsing/help/defaults, common gate
enforcement and narrow local export; 30/40 apply gates to their representations/
results/evidence; 50 extends history/graph/provenance and overlap cases; 60
qualifies all eight flag combinations and export in the installed artifact.
No flag alters Engine eligibility/ranking, restores physically absent content
or bypasses another applicable gate. No general export tooling is added.

## Slice 16 architecture handoff — S16-2026-10-31

Consume the [canonical architecture](../../../design/high-level-design.md) and
[Slice 16 status](../slice-16/status.md) before implementation. Central composition,
operation/reference/policy/resource/protocol contracts are selected there; local
refinement cannot silently replace them. This feature draft remains unexecuted. B16-01 is resolved by the 2026-10-31 owner ruling: retained-erasure content
is conditional on legitimate existence. Physical deletion remains unchanged.
Consume completed Slice16 before starting; typed supplied-content export
contracts are distinct from real Engine deletion/absence/flag integration.

Owned components: **S graph/history/lifecycle + Q trace + U directed SVG**. Read full exact edge payloads/endpoints and retained revisions via S, never infer edges from graph_neighbors node output. Provide actual keyboard/pointer directed SVG expansion/collapse and synchronized list; preserve parallel/self/cycle identities and scope. Public trace/evidence retains its own context. Extend independent inactive/erasure/telemetry gates across closure links, history, graph labels and narrow export.

Assigned architectural verification: **M08–M13, M15, M17–M21, M27–M29, M31–M34, M38–M39**, using existing fixtures/oracles
in the [single matrix](../verification-plan.md), including this feature's native
UI, keyboard, security, partial/error and resource paths. Test names/RED evidence
remain feature work. Feed semantic changes back to canonical requirements/AC/HLD
and affected matrix rows before implementing; preserve Inspector-only scope.

Postcompletion Astra N16-A01: Use the same selected process composition; do not add graph/trace-specific environment overrides or bypass inherited-config tests. The [canonical environment contract](../../../design/high-level-design.md#process-environment-is-part-of-the-selected-composition) and shared matrix own the one selected allowlist.
