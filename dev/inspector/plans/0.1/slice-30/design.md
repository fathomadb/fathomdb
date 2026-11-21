---
title: Inspector 0.2.0 Slice 30 — high-level draft design
date: 2026-10-28
product: FathomDB Inspector
target_release: 0.2.0
status: PROPOSED
desc: FTS/BM25 inspection and lexical retrieval
---

# Slice 30 — high-level draft design

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

## Status and intent

DRAFT at `037a8522e`. This is design guidance for the
[slice plan](plan.md), not implemented behavior or an approved dependency
change. The executing writer reconciles it against current in-repo work,
completes the needed contracts, and obtains direct **gpt-6.1-sol-high** design
review before affected implementation.

A complete indexed-text and lexical-search workflow linked to the exact records and request observations from Slice 20.

This feature refines the completed [Slice 15](../slice-15/plan.md)
requirements/verification and [Slice 16](../slice-16/plan.md) architecture
baselines. Reconcile this draft against both; shared contracts belong in the
canonical design and verification matrix, with feature-local detail here.

## Components and data flow

Inspector lexical capability descriptor, validated request adapter, indexed-content/origin reader, bounded lexical executor, result observation and ranked-match renderer.

```text
Select record or lexical structure → inspect indexed content → submit supported lexical request → inspect ranked evidence → resolve verified origin.
```

All new components are Inspector-owned. Use the existing shell/session/typed
reference mechanisms from prior completed slices where available rather than
create parallel frameworks. UI interaction and read/backend behavior form one
slice-complete feature; table-only placeholders do not replace required views.

## Read and presentation contracts

Request identifies actual route, fields, tokenizer, scoring family, effective filters/limits and read context. Result includes observed payload, typed identity, evidence availability and verified origin references; it does not promise unobserved per-token statistics.

Qualified Engine startup and its ordinary workers are allowed. Inspector
exposes no migration/repair/editing action and retains cache-only/no-download
constraints; successful startup has no byte-preservation guarantee. A real record/result is
never synthesized from layout, equal text or guessed provenance. Product
metadata and factual unknown/unavailable states remain explicit.

## Dependency feasibility and ownership

Treat Engine/MCP/agent-access as read-only references and unchanged dependencies. Existing Engine lexical execution, including supported BM25F scratch writes, may be reused unchanged; no parallel query-only runtime is required.

Source reads are permitted for conformance and reuse assessment. No design
sentence authorizes edits to Engine, schema, MCP, SDK or agent-access. Validate
existing callable surfaces versus private/test-only helpers. Detail any
necessary Inspector-local storage adapter and its semantics/conformance before
implementation; an unavailable required capability is recorded, not hidden.

## Errors, resources and consistency

Unsupported route is identified before query; absent metadata is unknown, not fabricated. Query cancellation/budget exhaustion differs from an empty set. Any private scratch state is Inspector-owned and cannot add Inspector mutation commands or evade cache/evidence constraints; normal Engine effects are separately qualified.

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

Owned components: **Q lexical + S indexed evidence + U lexical**. Use selected public Engine text/property/BM25F operations with route-specific meaning; show actual indexed fields/effective tokenizer and method-specific scores. Any FTS5 highlight is a separate index-match observation unless actual shared snapshot evidence exists. Do not import sibling snippets/degradations or write a second tokenizer/ranker. Preserve policy-filtered rank gaps, exact origins and child deadline behavior.

Assigned architectural verification: **M06, M09–M17, M20–M22, M25, M28–M34, M36**, using existing fixtures/oracles
in the [single matrix](../verification-plan.md), including this feature's native
UI, keyboard, security, partial/error and resource paths. Test names/RED evidence
remain feature work. Feed semantic changes back to canonical requirements/AC/HLD
and affected matrix rows before implementing; preserve Inspector-only scope.

Postcompletion Astra N16-A01: Verify hostile-parent experimental search caps/reader PRAGMAs do not change actual lexical/hybrid candidates or route context on F-07. The [canonical environment contract](../../../design/high-level-design.md#process-environment-is-part-of-the-selected-composition) and shared matrix own the one selected allowlist.
