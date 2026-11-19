---
title: Slice 50 runtime execution design
status: APPROVED
slice: 50
target_release: 0.8.30
---

# Slice 50 runtime execution design

## Reconciliation and scope

Baseline `cd64360a63bf06c7d1bc5f27865cca6d7400eff8`, isolated branch
`impl/0.8.30-slice50`. The full [plan](plan.md) remains selected by seq-408.
No mandatory row is removed by the unavailable prerequisites below.

| Change since draft `f7fb1da9a` | Disposition |
| --- | --- |
| Approved10 updates Tokio/TLS/Markdown and selects Rust1.95, Python>=3.11 with pinned tools, development Node26.13.1/npm11.20.0.20 supplies checkout-owned noneditable build and diagnostics. | Keep; provision owned tools/artifacts, reuse exact tool executables read-only. Development Node does not change public support. |
|30 fixes MCP revision feedback and dependency trace and adds committed coverage/rank controls. | Consume unchanged; do not reopen correctness or occurrence-label design. |
|40 completes source at `d64e870bd`, including signed ownerless ranking, interrupted output and observer recovery. Its original full-gate exit1 and inherited200us hints criterion remain visible. | Consume; no replay of40 benchmarks/gate.70 owns remaining qualification. |
| seq-406 audit and seq-407 divide runtime50, data/recovery55, source70, artifacts75. | Revise old prework3/4/5 runtime proposals to this split. Keep queue, close, Node, logging and conditional release here.55 owns fuzz, durable crash/WAL and doctor gaps. |
| Actual CLS cells return static borrows. Engine reranker uses `Mutex<Option<SingletonState<T>>>`, leaking a successful boxed model into a static borrow; it memoizes unavailable/policy refusal separately from retryable CUDA failures. | Revise the draft's generic singleton description; a release API requires owned inference leases throughout all consumers. |
| Consumed cudarc `343b04fa73f85ee78754d6ee2baa07b959cc9ccc`, Candle trio `2ee39305a78ed1463ff913f71dcd3d45ecbfd84f`, kernels `182ceab0547348993b6dbec4b1aed610dcc8b23f`. .29 source/seam110 explicitly holds real reset-close/last-clone/consumer proof.60 remains proposed. | Required exact-pin Orin safe-drop proof is unavailable. Keep conditional B30-01 exercise/shipment held; perform independent safe work. No reset/session authority is inferred. |
| .29 default Node FAIL/OFF_NOT_RUN; separate semispace4 pair passed. | Retain both separately. Actual default private-pool comparison requires a currently authorized Orin venue, matched addon/model and ordinary defaults. Local CPU work cannot establish that result. |

Goals are truthful admission/cancellation, positive isolated work, ownership
release and exact restart state, and disposition of retained resource/logging
findings. Non-goals are format/API expansion without its prerequisite, general
scheduler/framework rewrites, automatic idle release, same-process CUDA
recovery, new GPU sessions, paid campaigns, live reset and publication.

Apply the [quality roadmap](../../../../notes/fathomdb-quality-testing-roadmap.md):
real stores, first/persistent queue pressure, positive recovery and owned
cleanup. Defer exhaustive schedules, VFS faults, broad fuzz and duplicate SDK
matrices to their selected owners. Existing runtime seams make those additions
unnecessary here. No canonical acceptance number is created.

## Local acceptance and bounded schedules

| Case | Schedule and independent acceptance |
| --- | --- |
| MCP queued cancellation | Hold one real `fetch` after admission; arm the existing one-attempt pre-enqueue observation before sending B with an explicit `_meta.progressToken` on its dedicated raw JSON-RPC connection. Observe/release the pre-enqueue hold, then use a bounded test-only read-until-notification helper to witness B's own first `queued:` progress. Cancel B by request ID; do not expect a cancellation response. Wait for its lease to retire (`!is_active()`), assert empty `bridge_epochs`, then queue a later request. Release one at a time; only surviving requests reach the call rendezvous and return exact `l:note:a`/body. No fixed sleep establishes arrival. |
| MCP started run limit and stores | Reuse the existing abandonment case with held-call rendezvous and exact timeout/deadline, in_flight1/abandoned1 until actual return. Then positive exact fetch and zero abandoned. In a single registry with max_open_stores1, assert busy while held, then switch successfully; two independent registries on distinct DBs must complete simultaneously. |
| Direct dispatch and isolation | Run existing foreground suite including direct `Embedder` started timeout, `Overloaded` saturation/queued expiry, error/panic reuse and documented sparse fallback. Add one bounded two-engine case: A's first call is held past its deadline with pool1, B commits and reads exact authored state and embeds successfully; A queued calls cannot invoke provider before release, then healthy work succeeds. No MCP permit assertion substitutes for a worker assertion. |
| Close, alias and restart | Reuse real SQLite-owner close/deadline observations: no deadline before quiescence, one shared deadline through concurrent/repeated close/Drop, provider-only retention then accounting0 on return. Extend alias case so targeted A startup hook cannot affect positive B, and committed A/B data survives close. Exercise canonical/alias busy ownership and concurrent/repeated close. A bounded fresh child reads exact authored pre-close IDs/bodies, writes a distinct sentinel and reads its exact contents, then closes and exits0 naturally. A subsequent reopen checks the original rows plus sentinel, proving both read and write ownership recovered. |
| Logging synchronization | First reproduce unchanged `6 != 3` with owned installed native code. `configure` intentionally emits started and finished; each callback attempts three forbidden operations. Replace first-callback completion with an explicit condition over completed per-phase batches. After both serialized configure calls return, issue a positive model-free search whose distinct Search/finished event is a FIFO delivery fence. Wait for that callback to finish; assert the exact Admin sequence started/finished/started/finished and three named InvalidArgumentError results per event. The search fence does not attempt reentry. Close/reopen and independently read the two persisted named configuration records, not just receipt cursors. Preserve the original failure. Distinguish two intended events from duplicate product emission; do not change count without an event oracle. |
| Default Node resources | Freeze venue, Node/addon/model/input identity, defaults, positive workload, passive counters, sample bounds and natural-close/exit rules before any selected pair. Use existing soak tools. JS heap used/capacity, external/arrayBuffers and native pool USED/RESERVED are different measures; Linux RSS/anonymous/swap supply process context. Do not force GC or claim opaque native attribution. Missing actual venue authority leaves this required row open. |
| Conditional B30-01 | Inventory is complete below; exact-pin safe-drop60 precedes exercising or shipping the CUDA path. A held prerequisite keeps the row open; no CPU-only implementation is silently called complete. Any later API change needs same-change contracts/ADR and thin Rust/Python/TS installed behavior. |

Each case has its selected schedule plus at most one discriminating follow-up
unless a concrete new finding justifies expansion. Passing characterization is
reported as such. Each behavioral repair has an observed failing assertion
committed before its separate GREEN; test oracles remain fixed during repair.

## Ownership and design alternatives

The engine dispatcher owns bounded provider workers and queue deadlines. MCP
`StoreRegistry` owns permits and abandoned-call guards across a blocking bridge.
Closing a registry and closing an Engine are different owners. Keep existing
SQLite ownership, lock release and quiescence-before-provider-drain ordering
under the [runtime ADR](../../../../adr/ADR-0.8.27-engine-owned-runtime-topology.md).
Test watchdogs are not production close SLAs.

Python logger transport has one bounded delivery worker per attachment. Engine
callbacks enqueue events; they never call Python on SQLite threads. Without
overload, live functioning loggers receive intended phase pairs. The
[subscriber ADR](../../../../adr/ADR-0.8.27-python-subscriber-delivery.md)
already rejects database reentry on that delivery thread. The first callback's
Event is an insufficient completion witness for two callbacks. Prefer a
condition guarding phase-plus-error batches followed by a distinct Search/finished
FIFO fence after both serialized configure returns. This bounds exact Admin
totals without inventing an operation ID or sleeping. Query the reopened
operational_collections table independently for exact named schema records. Production changes are warranted only if the
event-specific oracle finds duplication or a permitted reentry.

B30-01 consumers are Python `embed_batch_cls_impl` after singleton lookup and
GIL detach, Node `embed_batch_cls_impl` inside `spawn_blocking`, and engine
`CandleCrossEncoder::try_get_loaded` through search and standalone reranking.
The latter uses `get_or_load`; successful values currently become static.
Engine-owned Mean providers stay under Engine close, outside module release.
An eventual selected release should replace static borrows with owned leases:
one shared cell state Empty/Loaded/Releasing; lookup and lease acquisition are
atomic under the cell mutex, while inference runs outside it. Release must
refuse active leases with a typed busy outcome, remove the sole cell owner,
keep Releasing visible during destruction, and only then return to Empty.
Repeated Empty release succeeds; refusal/unavailable memoization clears only
through explicit release. Never hold cell locks over native teardown, GIL or
engine locks; never allow a new load during teardown. Arc strong-count sampling
alone cannot serialize lookup against release. Waiting indefinitely for active
calls and forcing last-drop during use are rejected alternatives. Exact API
spelling and error schema remain conditional on prerequisite proof and review;
no new public symbol is introduced by this design-only analysis.

Primary references checked2026-11-19: [Rust Arc ownership](https://doc.rust-lang.org/std/sync/struct.Arc.html),
[pinned Node process memory](https://nodejs.org/download/release/v26.13.1/docs/api/process.html#processmemoryusage)
and [V8 heap spaces](https://nodejs.org/download/release/v26.13.1/docs/api/v8.html#v8getheapspacestatistics).
Arc keeps values alive until the last strong owner; this is necessary but not
sufficient for serialized model release. Node's passive process/V8 counters do
not expose every native allocation; memory sampling itself has a cost.

## Review, implementation and verification

Direct Astra-medium returned plan PASS and design PASS after one consolidated
FIX-1 on2026-11-19. The fixes specify token-specific raw queued progress,
retired-lease bridge exclusion, a fresh-process read/write sentinel and a
Search/finished FIFO logging fence. Set the fixture slow-event threshold above
its watchdog before attachment so legitimate Slow diagnostics cannot alter
the selected four Admin events. Commit after PASS. Then
execute scoped MCP/engine/Python checks, repair demonstrated defects with
RED/GREEN and run direct Sol6.2-high code review and direct Terra runtime/native
verification. All reviewers are read-only in this checkout.

Provision this checkout's .venv with>=3.11, pip and pinned tools. Use
`scripts/setup-python-owned.sh` for noneditable owned wheels and existing
test-hooks/native build routes, owned npm modules and exact read-only tool
executables. Never borrow another venv or overwrite its native module. Keep
raw logs under `/tmp/fathomdb-0830-slice50-evidence`. After stable source,
prepare required native receipts/generated views, commit, then run the full
`agent-verify.sh` once on unchanged clean source in a ptrace-capable executor.
Preserve actual original failures/counts; recover only affected/failed/unrun
scopes. Metadata-only final edits use scoped Markdown validators.

Status must give all six rows a measured result or explicit open consequence,
including original logging/default Node failures and the safe-drop prerequisite.
Parent integrates the clean committed branch and owns release-state updates,
ledger writes and cleanup. End all owned processes before handoff.
