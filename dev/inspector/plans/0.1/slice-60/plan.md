---
title: Inspector 0.2.0 Slice 60 — draft plan
date: 2026-10-28
product: FathomDB Inspector
target_release: 0.2.0
status: PROPOSED
desc: Integrated investigations and v0.1 qualification
---

# Slice 60 — Integrated investigations and v0.1 qualification

HISTORICAL INPUT ONLY. The [new 60 comparison/interactions](../../0.2.0/slice-60/plan.md)
and [new 70 final qualification](../../0.2.0/slice-70/plan.md)
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
15, 16, 20, 30, 40, 50 complete. Reconcile from this baseline before execution; later commits do
not make this draft automatically current.

A cohesive, accessible and packaged Inspector with cross-view investigations, available outcome evidence, bounded-resource behavior and complete v0.1 qualification.

Allocated scope: Complete INS-04/05/06/08 and cross-view INS-09; all INS-01–INS-09 and V01-N01–N07/S01–S04 final coverage; INS-X01–X11; F-01–F-07; T-01–T-05.

- Complete integrated record → FTS/BM25 → vector → graph investigations, result-set comparison, exact observation/revision preservation and available per-request/per-hit outcome evidence.
- Finish Inspector-wide navigation, keyboard accessibility, explicit scopes/unknown states, cancellation/detach, resource budgets and F-06 scale envelope.
- Package a locally runnable artifact for the declared target, publish accurate usage/limits documentation and qualify installed launch/attach/views/exit without publishing registries or tags.
- Run the documentation-assisted unseen transfer evaluation and all remaining High-depth/Need/Should checks; distinguish actual human evidence from automated simulations.

Exclusions: No Engine/MCP/SDK rollout, new schema/version migration, live/WAL/older-schema support, broader roadmap additions or automatic publication. This slice integrates prior complete features; it is not a dumping ground for their missing implementations.

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
   Reconcile the entire coverage map against implemented Inspector behavior and prior statuses. Define cross-view outcome/evaluation criteria, supported envelope and installed-artifact smoke. Resolve drift rather than waive required views or hide missing acceptance behind limitations.
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

- Use RED/GREEN for remaining cross-view comparison/navigation/outcome behaviors and bugs; reuse completed feature contracts instead of rewriting their layers.
- Complete request observation retention, honest unknown-cause handling, integrated accessibility, budgets and clean shutdown.
- Add local build/package/launch and installed-artifact smoke for the declared product target; update docs to tested behavior and keep optional work on ROADMAP.
- Have Sol review the final implementation and Terra independently qualify the artifact and relevant regressions; complete the human tasks when participants are available and report any unperformed required evaluation.

Keep all implementation under `src/inspector/` and its narrowly enumerated
Inspector test/build/docs wiring. Changes to Engine, MCP, SDK, schema and
agent-access code/contracts are outside this slice, even if earlier proposals
suggested shared refactoring. A genuine dependency gap is a blocker, not
permission to expand scope or downgrade acceptance.

## Testing and slice completion

- F-01–F-07 focused and integrated acceptance, including full browser document → lexical → vector → graph → retained observation navigation with ambiguous/equal-content identities.
- F-06 scale, large payload, high-degree graph, cancellation/late-result disposal, finite resource ownership and detach/exit cleanup; verify no Inspector mutation commands and no model/cache download or repair; qualify Engine lifecycle effects and observation contexts separately.
- Installed artifact on the declared environment: launch, attach, exercise every view, close/exit; appropriate final code-phase regression gate and strict public-doc build.
- T-01–T-05 with agreed rubric and ordinary docs, unseen fixture, actual participant results and High INS-07/08 scenarios; unavailable participants are a remaining qualification gap, not a fabricated pass.

Run focused lint/type/behavior checks first. For an individual finding, rerun
the reproducer and affected/adjacent tests; full regressions require a broad
blast radius, unresolved risk or an explicit required final gate. Broader
Slice 60 qualification does not excuse missing tests or an unusable feature
here. Follow repository TDD, review, environment and retry rules.

**Completion:** Every Inspector v0.1 requirement/action is mapped to executed evidence; design/code reviews and Terra verification are addressed, installed artifact and actual evaluation claims are truthful, and slice-owned temporary resources are cleaned up.

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

Assigned matrix rows (also M39): **M01–M39 final integration; primary M14, M35, M37–M38**. Audit all prior local evidence, qualify exact installed producer/backend/platform composition and F-06 aggregate budgets; run actual cross-view/keyboard flows and offline package launch/all views/close. Execute T-01–T-05 and separate High session with fixed rubric/assistance, report missing participants honestly; four-of-five stays diagnostic.

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

Owned components: **K installed qualification + R aggregate + U/O integrated**. Qualify the exact Linux offline tar/browser/build pair, all views and eight policy combinations with conditional retained-erasure export contracts and actual Engine absence/flag integration; measure aggregate memory/jobs/bytes/time and real close/kill/reap/lock release. Run unseen Critical tasks and separate High investigation with honest participant records. Produce actual supported producer/reader/query/browser evidence; architecture review is not an installed/runtime pass.

Assigned architectural verification: **M01–M39 integrated, especially M14, M28, M31, M35, M37–M39**, using existing fixtures/oracles
in the [single matrix](../verification-plan.md), including this feature's native
UI, keyboard, security, partial/error and resource paths. Test names/RED evidence
remain feature work. Feed semantic changes back to canonical requirements/AC/HLD
and affected matrix rows before implementing; preserve Inspector-only scope.

Postcompletion Astra N16-A01: Repeat hostile-parent header/candidate/cache/network/log cases against the installed artifact and declared environment-policy build revision. The [canonical environment contract](../../../design/high-level-design.md#process-environment-is-part-of-the-selected-composition) and shared matrix own the one selected allowlist.
