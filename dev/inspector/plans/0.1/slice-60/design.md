---
title: Inspector 0.2.0 Slice 60 — high-level draft design
date: 2026-10-28
product: FathomDB Inspector
target_release: 0.2.0
status: PROPOSED
desc: Integrated investigations and v0.1 qualification
---

# Slice 60 — high-level draft design

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

## Status and intent

DRAFT at `037a8522e`. This is design guidance for the
[slice plan](plan.md), not implemented behavior or an approved dependency
change. The executing writer reconciles it against current in-repo work,
completes the needed contracts, and obtains direct **gpt-6.1-sol-high** design
review before affected implementation.

A cohesive, accessible and packaged Inspector with cross-view investigations, available outcome evidence, bounded-resource behavior and complete v0.1 qualification.

This feature refines the completed [Slice 15](../slice-15/plan.md)
requirements/verification and [Slice 16](../slice-16/plan.md) architecture
baselines. Reconcile this draft against both; shared contracts belong in the
canonical design and verification matrix, with feature-local detail here.

## Components and data flow

Inspector investigation/observation state, result comparison, shared accessibility/resource controls, local package/launch boundary and end-to-end qualification harness.

```text
Launch installed app → attach → navigate coordinated feature views and compare observed results → inspect available outcome facts → cancel/detach → close and clean exit.
```

All new components are Inspector-owned. Use the existing shell/session/typed
reference mechanisms from prior completed slices where available rather than
create parallel frameworks. UI interaction and read/backend behavior form one
slice-complete feature; table-only placeholders do not replace required views.

## Read and presentation contracts

Every cross-view reference retains database/class/identity/revision/read context; comparison distinguishes separate observations. Unknown missing-result causes remain unknown. Package/platform claims are bounded by tested artifacts, not inherited SDK matrices.

Qualified Engine startup and its ordinary workers are allowed. Inspector
exposes no migration/repair/editing action and retains cache-only/no-download
constraints; successful startup has no byte-preservation guarantee. A real record/result is
never synthesized from layout, equal text or guessed provenance. Product
metadata and factual unknown/unavailable states remain explicit.

## Dependency feasibility and ownership

Integrate Inspector code only. Existing Engine/MCP libraries remain unchanged dependency inputs. If integrated qualification exposes an out-of-scope dependency bug, preserve the reproducer and report it; do not patch that subsystem in this release slice.

Source reads are permitted for conformance and reuse assessment. No design
sentence authorizes edits to Engine, schema, MCP, SDK or agent-access. Validate
existing callable surfaces versus private/test-only helpers. Detail any
necessary Inspector-local storage adapter and its semantics/conformance before
implementation; an unavailable required capability is recorded, not hidden.

## Errors, resources and consistency

Earlier feature regressions are fixed with targeted reproductions, not bypassed. Missing models/tools/participants, unreviewed material changes and unsupported dependent capabilities remain explicit outstanding work. No pass-by-skip or automatic publish.

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

Owned components: **K installed qualification + R aggregate + U/O integrated**. Qualify the exact Linux offline tar/browser/build pair, all views and eight policy combinations with conditional retained-erasure export contracts and actual Engine absence/flag integration; measure aggregate memory/jobs/bytes/time and real close/kill/reap/lock release. Run unseen Critical tasks and separate High investigation with honest participant records. Produce actual supported producer/reader/query/browser evidence; architecture review is not an installed/runtime pass.

Assigned architectural verification: **M01–M39 integrated, especially M14, M28, M31, M35, M37–M39**, using existing fixtures/oracles
in the [single matrix](../verification-plan.md), including this feature's native
UI, keyboard, security, partial/error and resource paths. Test names/RED evidence
remain feature work. Feed semantic changes back to canonical requirements/AC/HLD
and affected matrix rows before implementing; preserve Inspector-only scope.

Postcompletion Astra N16-A01: Repeat hostile-parent header/candidate/cache/network/log cases against the installed artifact and declared environment-policy build revision. The [canonical environment contract](../../../design/high-level-design.md#process-environment-is-part-of-the-selected-composition) and shared matrix own the one selected allowlist.
