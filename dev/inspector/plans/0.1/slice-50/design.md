---
title: Inspector 0.2.0 Slice 50 — high-level draft design
date: 2026-10-28
product: FathomDB Inspector
target_release: 0.2.0
status: PROPOSED
desc: Graph, provenance and retained revisions
---

# Slice 50 — high-level draft design

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

## Status and intent

DRAFT at `037a8522e`. This is design guidance for the
[slice plan](plan.md), not implemented behavior or an approved dependency
change. The executing writer reconciles it against current in-repo work,
completes the needed contracts, and obtains direct **gpt-6.1-sol-high** design
review before affected implementation.

Interactive FathomDB graph navigation with edge detail, class-correct lifecycle, bounded provenance/dependency tracing and retained canonical revision navigation.

This feature refines the completed [Slice 15](../slice-15/plan.md)
requirements/verification and [Slice 16](../slice-16/plan.md) architecture
baselines. Reconcile this draft against both; shared contracts belong in the
canonical design and verification matrix, with feature-local detail here.

## Components and data flow

Inspector graph/trace/revision adapters, typed relationship payloads, bounded expansion state, canvas layout/selection, accessible synchronized detail and preserved observation state.

```text
Select exact record → expand directed neighborhood → inspect edge → follow recorded source/dependency or retained revision → return to the original graph/result observation.
```

All new components are Inspector-owned. Use the existing shell/session/typed
reference mechanisms from prior completed slices where available rather than
create parallel frameworks. UI interaction and read/backend behavior form one
slice-complete feature; table-only placeholders do not replace required views.

## Read and presentation contracts

Graph edges carry their own identity, direction, endpoints, kind and observed payload. Trace/provenance/supersession have separate link types and completeness. Node state and edge temporal eligibility have class-specific fields, not one fabricated lifecycle enum.

Qualified Engine startup and its ordinary workers are allowed. Inspector
exposes no migration/repair/editing action and retains cache-only/no-download
constraints; successful startup has no byte-preservation guarantee. A real record/result is
never synthesized from layout, equal text or guessed provenance. Product
metadata and factual unknown/unavailable states remain explicit.

## Dependency feasibility and ownership

Consume unchanged interfaces and Inspector-local stored-data reads only. Missing full edge/revision APIs are identified, not remedied with Engine/MCP code changes or private module imports.

Source reads are permitted for conformance and reuse assessment. No design
sentence authorizes edits to Engine, schema, MCP, SDK or agent-access. Validate
existing callable surfaces versus private/test-only helpers. Detail any
necessary Inspector-local storage adapter and its semantics/conformance before
implementation; an unavailable required capability is recorded, not hidden.

## Errors, resources and consistency

No endpoint/origin is guessed by name or text. Missing/erased history stays unavailable and policy limits are explicit. Bounded expansion announces its visible subgraph; graph geometry is presentation, not stored relation strength.

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

Owned components: **S graph/history/lifecycle + Q trace + U directed SVG**. Read full exact edge payloads/endpoints and retained revisions via S, never infer edges from graph_neighbors node output. Provide actual keyboard/pointer directed SVG expansion/collapse and synchronized list; preserve parallel/self/cycle identities and scope. Public trace/evidence retains its own context. Extend independent inactive/erasure/telemetry gates across closure links, history, graph labels and narrow export.

Assigned architectural verification: **M08–M13, M15, M17–M21, M27–M29, M31–M34, M38–M39**, using existing fixtures/oracles
in the [single matrix](../verification-plan.md), including this feature's native
UI, keyboard, security, partial/error and resource paths. Test names/RED evidence
remain feature work. Feed semantic changes back to canonical requirements/AC/HLD
and affected matrix rows before implementing; preserve Inspector-only scope.

Postcompletion Astra N16-A01: Use the same selected process composition; do not add graph/trace-specific environment overrides or bypass inherited-config tests. The [canonical environment contract](../../../design/high-level-design.md#process-environment-is-part-of-the-selected-composition) and shared matrix own the one selected allowlist.
