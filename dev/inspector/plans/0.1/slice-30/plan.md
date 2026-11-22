---
title: Inspector 0.2.0 Slice 30 — draft plan
date: 2026-10-28
product: FathomDB Inspector
target_release: 0.2.0
status: PROPOSED
desc: FTS/BM25 inspection and lexical retrieval
---

# Slice 30 — FTS/BM25 inspection and lexical retrieval

HISTORICAL INPUT ONLY. The [current Slice 30 draft](../../0.2.0/slice-30/plan.md) and
[design](../../0.2.0/slice-30/design.md)
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
15, 16 and 20. Reconcile from this baseline before execution; later commits do
not make this draft automatically current.

A complete indexed-text and lexical-search workflow linked to the exact records and request observations from Slice 20.

Allocated scope: INS-03 supported text constraints; INS-04 lexical associations; INS-05 lexical observations; INS-06 lexical readiness; INS-08 lexical evidence; INS-09.2/5/6; V01-N05; lexical V01-S02/S04; INS-X01–X11.

- Inventory the actual supported lexical routes, indexed fields, tokenizers and scoring families; distinguish configuration, indexed content, canonical body and observed matches.
- Deliver lexical query controls, bounded ranked document/passage results, actual available match evidence and method-appropriate score/rank display.
- Link selected index entries/results to verified origin records/revisions and preserve exact request context and back navigation.
- Reuse qualified Engine lexical execution and reconcile Inspector scratch/request ownership without editing Engine or importing a sibling branch as an assumed dependency.

Exclusions: No historical/inactive index explorer, new FathomDB tokenizer/index, Engine BM25F changes or model service. Neither absent highlight metadata nor a generic SQL result grid substitutes for the required lexical view.

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
   Specify every supported lexical operation and its route/evidence contract, including unavailable metadata, parser failures, limits and origin ambiguity. Extend the lexical portion of F-07 and V01-N05/S02; do not assume every historical FTS table is active.
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

- Revalidate Slice 16's selected lexical route/capability baseline revalidated by Slice 20, then refine the feature-local adapter using available unchanged libraries and stored FTS structures.
- RED/GREEN route/field/tokenizer attribution, query validation, indexed-text/origin reads and bounded retrieval against real lexical fixtures.
- Implement the indexed-content and ranked-match UI with only verified match evidence and distinguish BM25 from BM25F/fused scores.
- Complete lexical result-to-record navigation, scratch ownership, resource/error cases and public guide examples.

Keep all implementation under `src/inspector/` and its narrowly enumerated
Inspector test/build/docs wiring. Changes to Engine, MCP, SDK, schema and
agent-access code/contracts are outside this slice, even if earlier proposals
suggested shared refactoring. A genuine dependency gap is a blocker, not
permission to expand scope or downgrade acceptance.

## Testing and slice completion

- F-07 known lexical matches and indexed-text/body differences with independently specified expected record identities, supported route and score-family facts.
- Query/field/filter/limit boundaries; verified versus missing match spans; ambiguous/missing origin; zero matches versus error/cancel/budget exhaustion.
- No extra Inspector mutation commands or cache/network writes, qualified Engine effects, scratch cleanup, and browser index → query → result → source → back with retained request observations.

Run focused lint/type/behavior checks first. For an individual finding, rerun
the reproducer and affected/adjacent tests; full regressions require a broad
blast radius, unresolved risk or an explicit required final gate. Broader
Slice 60 qualification does not excuse missing tests or an unusable feature
here. Follow repository TDD, review, environment and retry rules.

**Completion:** A user can inspect indexed content, execute the supported lexical operation, inspect factual match/scoring/route data and reach the exact originating record. The feature is not deferred to integration.

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

Assigned matrix rows: **M06 (text), M09–M17 (lexical), M21–M22, M25, M28–M34, M36**. Extend F-01/02/04/07 with hand-checkable effective lexical routes, corpus/formula/spans and exact origin cardinalities; actual indexed-text/query/ranked-result/source/back keyboard flow. Test owned scratch/request cleanup and no guessed unavailable evidence.

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

Owned components: **Q lexical + S indexed evidence + U lexical**. Use selected public Engine text/property/BM25F operations with route-specific meaning; show actual indexed fields/effective tokenizer and method-specific scores. Any FTS5 highlight is a separate index-match observation unless actual shared snapshot evidence exists. Do not import sibling snippets/degradations or write a second tokenizer/ranker. Preserve policy-filtered rank gaps, exact origins and child deadline behavior.

Assigned architectural verification: **M06, M09–M17, M20–M22, M25, M28–M34, M36**, using existing fixtures/oracles
in the [single matrix](../verification-plan.md), including this feature's native
UI, keyboard, security, partial/error and resource paths. Test names/RED evidence
remain feature work. Feed semantic changes back to canonical requirements/AC/HLD
and affected matrix rows before implementing; preserve Inspector-only scope.

Postcompletion Astra N16-A01: Verify hostile-parent experimental search caps/reader PRAGMAs do not change actual lexical/hybrid candidates or route context on F-07. The [canonical environment contract](../../../design/high-level-design.md#process-environment-is-part-of-the-selected-composition) and shared matrix own the one selected allowlist.
