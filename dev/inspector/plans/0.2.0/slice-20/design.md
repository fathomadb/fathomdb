---
title: Inspector 0.2.0 Slice 20 — record navigation design draft
date: 2026-11-19
product: FathomDB Inspector
target_release: 0.2.0
status: COMPLETE
desc: Source-grounded extension of existing record handles, factual status and bounded observations
---

# Slice 20 — record navigation design

Execution design2026-11-19, baseline `daa2842c7`; unchanged Engine
`6edc1155f`, format37. [Plan](plan.md), [scope](../scope.md),
[master allocation](../plan.md), [workflow](../slice-workflow.md) and
[HLD](../../../design/high-level-design.md) govern.

## Concrete increment

- Detail adds individually scoped `facts`: storage, inspection eligibility,
  scalar readiness, retrieval eligibility/readiness and runtime. Layout facts
  share the detail transaction/context. Stored bytes prove storage; successful
  permitted detail proves inspection eligibility, not retrieval eligibility.
  Exact attribute read is ready for scalar inspection, not vector/text serving.
  No unbounded Engine status call; no fabricated aggregate readiness.
- Attribute detail mints canonical origin handle only after exact materialization
  and current classification validation. Origin action preserves class/cursor/
  revision; attribute remains an attribute, source/derived canonical identities
  remain distinct. All follow-up reads recheck D01.
- Detail retains its rendered workspace and separate tree/association observations
  in the existing cache. Back validates every restored content reference (anchor, materialization and
  association target), then restores original DOM, expanded
  tree/chunk state and initiating focus without rereading. Evicted hydration
  clears its nodes and shows explicit refresh state; detached nodes are cleared
  too. Explicit refresh discards presentation and starts new detail/hydration.
  No second cache or budget: existing ten/20MiB observations own retained bytes.
  Chunk responses also own and clear their displayed/detached payload nodes.
  `check.handles` validates at most201 exact handles in one bounded transaction
  (anchor plus shared200 association items), with unchanged authoritative policy.
  `metadata_handles` validates permitted structural row labels without demanding
  withheld payload permission. The201 cap combines all reference categories;
  one25s deadline and100k locator/dependency-closure counter apply to the batch.
  Queue JSON decode retains its separate existing16×16,384-cursor/8MiB bound,
  once per transaction; the SQL progress hook has its shared100M-step/30s bound.
  A failed/incomplete check invalidates the restored view; no cached target label
  survives because only its unrelated anchor was allowed.
- Capture initiating view/focus before awaiting navigation; Back focuses only a
  connected permitted control.
  Native buttons retain Enter/Space behavior. Navigation intent increments a
  token before each navigation request; guard acceptance before attachment,
  context, error invalidation and observation side effects. Superseded replies
  cannot select old detail or invalidate newer observations. Cancel/detach invalidates outstanding navigation. Late hydration updates
  only its retained owner, never an unrelated active view.
- Facts expose fixed labels/scopes and already-permitted lengths only. Existing
  reader/worker deadlines, caps and transaction-local queue checks remain binding.

## Existing foundation and delivered increment

`storage.rs` already mints bounded opaque canonical/operational/attribute handles,
checks class/cursor/revision, returns per-read context and separate associations,
and supports exact chunks/tree/export. `session.rs` owns ordinary Engine None
admission and current-format read transactions, including transaction-local bounded
queued-erasure membership. `app.js` retains ten observations/20 MiB, Back and
explicit refresh; changed generation clears cached values. These are reuse seams.
New work is a coherent canonical anchor and representation navigation with typed
factual/capability variants, source-qualified status and equivalent keyboard
interaction. Do not replace these mechanisms with a second session or framework.

```text
exact record selection → document/tree + metadata + factual status
                      → verified materialization/source choice → exact detail
                      → Back to permitted original anchor/context
later 30/40/50 result → typed origin resolution → same record anchor
```

Canonical derived records retain their own identities; a scalar attribute is a
representation, not another canonical record. Future result references preserve
namespace/ordinal/observation and zero/one/many origins. Distinct payload variants
share context/reference primitives, never one lossy universal row. Later slices
supply actual lexical/vector/graph interactions; this slice must not simulate them.

## Contracts and source feasibility

| Concern | Fit and limitation |
| --- | --- |
| Exact anchor | Reuse opaque handles; retain database/attachment/class/exact cursor/available revision separately from logical ID and originating request. Exact resolution refuses removal/mismatch rather than selecting latest. Operational current/log identities retain collection/key/cursor semantics. |
| Read context | Preserve operation/build/protocol/attachment, read times and actual visibility/projection/dependency generations from `Reader::context`. Current layout attests only its transaction; Engine API status/hydration is separately observed unless its API supplies matching authority. Unknown write boundary stays unknown. |
| Readiness/runtime | `Engine::read_projections`, `read_projection_status`, `read_projection_generation_status` and `read_embedding_readiness` are public pure reads. Read source and Rust interface for scope: shared-pipeline/corpus status does not attest a selected record's vector row or query eligibility. Stored scalar materialization is an independently observed fact; declaration is not completion. |
| Source/private boundary | Inspector-owned current-layout reads are acceptable only qualified against unchanged schema/Engine semantics, bounded and with D-01. Private derivation helpers are not callable cross-crate APIs. `read_mutation_projection_status` requires a real receipt; do not manufacture one for arbitrary browsing. Test-only readiness/time/provider controls remain fixture-only. |
| Capability | Distinguish operation implemented, attachment admitted, representation declared/materialized, runtime available and current request allowed. Present source/context with each fact. None-session dense unavailability does not prove all CPU cache-only paths impossible; 40 investigates supported production admission. Production Default/None cannot be replaced with TestEmbedderChoice::Caller. |
| Policy | Every newly resolved reference and content-bearing path checks current authoritative classification. Queue membership stays bounded per read transaction; neither unchanged generation nor earlier allowed observation authorizes later disclosure. Unknown classification refuses affected content. |

## Resolved design challenges

- Public registry/status reads can allocate or lock without the Inspector reader's
  SQL progress hook. Before choosing them, establish response/work/deadline bounds
  and worker ownership; use bounded qualified layout facts where appropriate,
  label separately observed API status, or surface a precise prerequisite. Never
  call an unbounded read then truncate its output and claim bounded work.
- Progress is a new observation, not a new record identity. Preserve permitted
  original context through Back; evict payloads on policy/generation invalidation
  and state explicit refresh/refused/expired. Do not silently reread the original
  result or combine a later readiness read with an earlier record snapshot.
- Aggregate ready and actual per-record materialization/eligibility are distinct.
  Show established facts and explicit unknown scope; prove required positive
  stored materialization/navigation now. Missing mandatory positive route is a
  blocker for its owning feature, never an unavailable-mode completion waiver.
- Shared limits include status, associations, labels, handles, history and late
  replies; no fresh category budgets. Retain canonical response/chunk/queue/time/
  memory caps. Stop/cancel only owned work and suppress late data after selection,
  cancellation or detach. Active anchor/focus must survive an honest partial state.
- History/inactive is visible by default; erasure-pending content is withheld by
  default; telemetry payloads need their own flag. All eight combinations compose
  on new views/copy/links/export. Real ExplicitMaintenance queues remain unchanged
  and physically absent content stays absent. Do not create or delay erasure state.

## Review, verification and extension boundary

Source/contracts and actual browser checks settled the implementation choices;
no external research or new dependency was required. Direct Sol-high design
review resolved3 gaps: all restored content owners are validated, rendered chunks
own eviction cleanup, and acceptance precedes shared response effects. The fresh
code review found the failed-old-check error path; its independent-intent
RED/GREEN regression and focused review resolved it. Direct Terra independently
verified actual keyboard/workspace/origin/policy behavior; [status](status.md)
records exact passes, invalid-composition attempts and remaining release scope.

Backend and browser checks use real databases with authored exact revision/value
facts, retained independent contexts, bounds and refusal/cancel/late-response
oracles. All4 carried native/allocation/F06 positives, committed startup/scale
probes and the single clean full source gate passed. None of these establishes
model serving, instantaneous memory peaks or installed22.04 window-close proof.

30/40/50 extend real lexical/vector/graph payloads using exact handles and scoped
observations; no lossy universal DTO was introduced. 40 must qualify cached CPU
serving through production Default/None and update session runtime facts when
admission changes. 60 owns pins/compare and the complete cross-view person task;
70 adds installed/integrated/human qualification. Record basics and mandatory
native/resource positives are complete here, not transferred to those owners.
