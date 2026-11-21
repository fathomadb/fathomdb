---
title: Inspector 0.2.0 Slice 40 — draft plan
date: 2026-10-28
product: FathomDB Inspector
target_release: 0.2.0
status: PROPOSED
desc: Vector representations and similarity neighborhoods
---

# Slice 40 — Vector representations and similarity neighborhoods

HISTORICAL INPUT ONLY. The [current Slice 40 draft](../../0.2.0/slice-40/plan.md) and
[design](../../0.2.0/slice-40/design.md)
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
15, 16 and 20; follows 30 in the sequential delivery order. Reconcile from this baseline before execution; later commits do
not make this draft automatically current.

A usable vector-representation and metric-faithful neighborhood workflow, with compatible cache-only query runtime and exact origin navigation.

Allocated scope: INS-04 vector associations; INS-05 vector observations; INS-06 vector readiness; INS-08 vector evidence; INS-09.3/5/6/7; V01-N04, V01-N07; vector V01-S02/S04; INS-X01–X11.

- Expose profile/model identity, dimensionality, stored representation metadata, generation/readiness and verified origin records without inferring links from text.
- Define and implement the selected neighborhood operation with metric, representation, candidate population, selection stages and relevant centering disclosed.
- Present actual distances/similarities and a labeled visual comparison with record selection; keep hybrid rank separate from vector metric.
- Initialize embedding and any supported reranker only from valid cached assets, at Engine startup where required or on explicit demand, with bounded ownership and no download/cache repair.

Exclusions: No embedding regeneration, model download, new ANN/index backend, Engine loader changes, 2D/3D map requirement or side-by-side binary/exact diagnostic dashboard. Optional reranking is supported only if the accepted Inspector operation includes it.

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
   Refine the operation contract selected by Slice 16 and revalidated by Slice 20: engine-style binary shortlist/f32 rerank or separately specified exact population, with no false global-neighbor claim. Specify cached asset validation and unavailable states; extend F-04/F-07 and V01-N04/N07.
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

- Confirm a cache-only/metric-capable route exists entirely in Inspector using unchanged dependencies and stored data; escalate a concrete gap rather than refactor the Engine loader.
- RED/GREEN representation/origin/readiness reads and independent small-vector distance/candidate facts.
- Implement cache-only request runtime, missing/partial/corrupt asset outcomes, measured neighborhoods, and Inspector-local request ownership.
- Deliver the vector detail/comparison UI and neighborhood-to-record navigation, then bound model work, handle cancel/detach and document the actual metric/population.

Keep all implementation under `src/inspector/` and its narrowly enumerated
Inspector test/build/docs wiring. Changes to Engine, MCP, SDK, schema and
agent-access code/contracts are outside this slice, even if earlier proposals
suggested shared refactoring. A genuine dependency gap is a blocker, not
permission to expand scope or downgrade acceptance.

## Testing and slice completion

- Independent small-vector numerical oracle and exact selected-population assertions; binary/float stage attribution if that path is selected; invalid metrics/dimensions and equal-distance identities.
- Network-denied valid/missing/partial/corrupt assets; no downloads, directory creation, repair or Inspector-requested persisted re-embedding; qualify normal Engine lifecycle/worker effects separately; optional reranker covered if used.
- F-04 ready/pending/failed/runtime-unavailable distinctions; browser vector selection → neighborhood → origin → back; limits, late-result disposal and detach cleanup.

Run focused lint/type/behavior checks first. For an individual finding, rerun
the reproducer and affected/adjacent tests; full regressions require a broad
blast radius, unresolved risk or an explicit required final gate. Broader
Slice 60 qualification does not excuse missing tests or an unusable feature
here. Follow repository TDD, review, environment and retry rules.

**Completion:** The supported neighborhood operation and vector UI work on real fixture representations with verified numerical evidence and no implicit model/cache mutation. A dimension count plus placeholder plot is insufficient.

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

Assigned matrix rows: **M09–M17 (vector), M21, M23, M26, M28–M34, M36**. Extend F-01/02/04/07 with independently calculated f32 L2/bit distances, centering, population/stages and excluded candidates. Verify network-denied valid cached CPU assets and missing/corrupt negatives, actual metric comparison/selection/origin navigation and bounded native completion.

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

Owned components: **S stored neighborhood + Q hybrid + R model helper + U metric**. Implement scoped exact stored-f32 L2 inspection over the declared bounded policy-visible population, explicitly separate from Engine hybrid binary-shortlist/f32 serving ranks. Distances are lower-is-nearer, never fused scores. Use the public LoadedWeights CPU constructor through the owned helper and Caller identity; generated Inspector build facts verify cached assets read-only. Optional CE stays uncompiled. Missing cache must preserve text/records; qualify real positive model work and finite helper termination.

Assigned architectural verification: **M09–M17, M20–M21, M23, M26, M28–M34, M36**, using existing fixtures/oracles
in the [single matrix](../verification-plan.md), including this feature's native
UI, keyboard, security, partial/error and resource paths. Test names/RED evidence
remain feature work. Feed semantic changes back to canonical requirements/AC/HLD
and affected matrix rows before implementing; preserve Inspector-only scope.

Postcompletion Astra N16-A01: Verify clean helper CPU/model/cache behavior and hybrid candidate attribution under hostile parent settings; no ambient experiment can select another composition. The [canonical environment contract](../../../design/high-level-design.md#process-environment-is-part-of-the-selected-composition) and shared matrix own the one selected allowlist.
