---
title: Inspector 0.2.0 Slice 40 — high-level draft design
date: 2026-10-28
product: FathomDB Inspector
target_release: 0.2.0
status: PROPOSED
desc: Vector representations and similarity neighborhoods
---

# Slice 40 — high-level draft design

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

## Status and intent

DRAFT at `037a8522e`. This is design guidance for the
[slice plan](plan.md), not implemented behavior or an approved dependency
change. The executing writer reconciles it against current in-repo work,
completes the needed contracts, and obtains direct **gpt-6.1-sol-high** design
review before affected implementation.

A usable vector-representation and metric-faithful neighborhood workflow, with compatible cache-only query runtime and exact origin navigation.

This feature refines the completed [Slice 15](../slice-15/plan.md)
requirements/verification and [Slice 16](../slice-16/plan.md) architecture
baselines. Reconcile this draft against both; shared contracts belong in the
canonical design and verification matrix, with feature-local detail here.

## Components and data flow

Inspector vector/profile/origin reader, cache-only model adapter, named neighborhood executor, request ownership controller and metric comparison renderer.

```text
Select a record/representation → inspect profile and readiness → choose supported record/query input → compute declared neighborhood → inspect measured values → open verified origin.
```

All new components are Inspector-owned. Use the existing shell/session/typed
reference mechanisms from prior completed slices where available rather than
create parallel frameworks. UI interaction and read/backend behavior form one
slice-complete feature; table-only placeholders do not replace required views.

## Read and presentation contracts

A response states representation/profile, metric, selected population, selection procedure/stages, relevant centering, limits and observed values. Distances within a shortlist are not proof of exhaustive nearest neighbors.

Qualified Engine startup and its ordinary workers are allowed. Inspector
exposes no migration/repair/editing action and retains cache-only/no-download
constraints; successful startup has no byte-preservation guarantee. A real record/result is
never synthesized from layout, equal text or guessed provenance. Product
metadata and factual unknown/unavailable states remain explicit.

## Dependency feasibility and ownership

Existing vector/embedding source is read-only evidence. Reuse only public cache-only mechanisms available unchanged; private or test-gated engine helpers are not automatically usable product APIs. No loader/kernel/engine edits belong here.

Source reads are permitted for conformance and reuse assessment. No design
sentence authorizes edits to Engine, schema, MCP, SDK or agent-access. Validate
existing callable surfaces versus private/test-only helpers. Detail any
necessary Inspector-local storage adapter and its semantics/conformance before
implementation; an unavailable required capability is recorded, not hidden.

## Errors, resources and consistency

Unavailable/corrupt assets disable the dependent operation without repair/download and leave independent browsing/text available. Wrong profile/dimension or stale origin fails explicitly. Native compute is bounded or isolated; UI cancellation is not a false interruption guarantee.

Use explicit operation/read context, bounded work and session-local resource
ownership. Shared connection/scratch state is serialized or isolated. The
Engine-backed allowance requires lifecycle-effect and observation tests, not
byte preservation after successful startup; GUI cancellation and actual
execution interruption remain distinct. Run the relevant slice fixtures and
UI flows from [plan](plan.md), including negative outcomes.

## Design decisions to complete during execution

- Enumerate draft deltas and confirm feature allocations against current code.
- Select the minimal Inspector-local payloads, adapters and view contracts
  needed for this slice, not a speculative universal data-browser framework.
- Record actual existing-library reuse, missing capabilities and effects on
  Engine effects, read-context fidelity and cache constraints; apply adopted D-01 to affected reads/export without reruling it.
- Map each contract to the slice's acceptance and independent oracle, then
  obtain Sol-high review and proceed to TDD implementation.

No Engine/MCP redesign, roadmap promotion, global contract rewrite or separate
worktree is required by this draft. Follow the
[common workflow](../slice-workflow.md) for review, verification and closure.

## Slice 15 baseline input

The [shared matrix](../verification-plan.md) baseline **S15-2026-10-30** and
[canonical operating envelope](../../../requirements.md#verification-and-operating-envelope-slice-15-2026-10-30)
now supply fixture/oracle/owner/stage obligations. Use the assigned rows in
[the plan](plan.md); architecture/toolchain feasibility is Slice 16 work.
Preserve zero/one/many origins, exact observation/revision, class-specific
eligibility, effect allowlists and honest unavailable states. INS-X07–X11 add
local access, inert rendering, keyboard equivalence, installed lifecycle and
aggregate bounds. Verify predecessor policy/completion in Slice 15 status.

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
