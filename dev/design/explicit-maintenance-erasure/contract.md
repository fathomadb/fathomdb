---
title: Explicit maintenance erasure — approved contract and API
status: ACTIVE
date: 2026-11-04
target_release: 0.8.28 Slice 85
---

# Approved contract and explicit cleanup API

This proposal answers the original end-of-Slice-83 guarantee/API question using
[completed findings](../../notes/erasure-optimization-sequence-findings.md).
HITL approved this contract on 2026-11-04 and assigned implementation to
[Slice 85](../../plans/0.8.28/features/slice-85/plan.md). Concrete design, accepted
successor ADR, implementation and independent qualification are complete at
`a2fb4a647`; [status](../../plans/0.8.28/features/slice-85/status.md) records
measured costs, actual verification provenance and platform/CI limits.
The [implementation outline](implementation-plan.md) lists design/test gates.

## Approved decision

Provide an opt-in, database-persisted **explicit maintenance policy**. During
normal service callers can queue a bounded physical-erasure intent. Expensive
deletion/sanitation runs only when the application deliberately grants a
maintenance window. The window excludes ordinary product reads/writes and
projection/admin work, allowing the existing exact proof and cleanup mechanisms
to run without repeated concurrent-survivor certification.

This first version deliberately promises **queued intent, not immediate logical
revocation**. The current engine achieves logical exclusion by atomic canonical
and projection deletion, which itself takes measurable time. A cheap source flag
plus output filtering would not preserve BM25 statistics, candidate caps, graph,
vector, property, historical/frozen and evidence authority. Those semantics cannot
be inferred from these experiments. Fast logical revocation is a separate possible
successor, not a hidden dependency or implemented feature of this proposal.

Users needing immediate non-disclosure must choose maintenance now, or continue
using an existing applicable revocation contract with its actual guarantees;
a queued receipt alone cannot meet that need. If low-latency immediate source
revocation is mandatory, reject this minimal v1 and commission its exact routing/
publication design before coding. Do not advertise queue acceptance as deletion.

## User-visible guarantees

| Event/outcome | What it means | What it does not mean |
| --- | --- | --- |
| Request accepted | A durable, bounded intent with immutable root selector/cut and a keyed receipt exists. Same-scope mutation restrictions are installed atomically. | Target data may still be readable. No logical or physical completion, deadline, reserved maintenance time or background execution is implied. |
| Logical removal committed | The selected roots and required dependency closure were atomically removed from canonical and registered projections. New admitted snapshots exclude that scope under existing routing semantics. | Old snapshots/results cannot be revoked; FTS retained bytes, WAL or original sinks may still need cleanup. |
| Physical complete | Exact structural absence, applicable registered-copy sanitation, original-sink obligations and successful WAL TRUNCATE completed before the durable receipt. | No universal storage-remanence, backup, snapshot, swap, returned-result or perpetual store-cleanliness claim. |
| Incomplete/blocked | The request and remaining obligation are retained; result describes committed effects, blocker and allowed service state. | No rollback of already committed logical deletion, no success, no new deadline, and no silent forgiveness of missing sinks. |

Physical scope preserves the current registered-copy inventory: canonical root/
dependent rows, registered FTS/content/shadows and applicable freed database
traces, vector mappings/storage, pending publication and receipt references,
original telemetry redactions and live WAL. Existing contract-authorized audit
and closure metadata carve-outs remain explicit; this is not deletion of every
byte referring to a subject. Newly introduced queue selectors, retry fingerprints
and temporary identity data are themselves registered erasure-owned copies and
must be removed before the final identity-free checkpoint boundary.

A terminal receipt is scoped to an immutable request/cut and declared artifact
boundary. It contains opaque identities, outcome and safe counts, not source
labels, body text, denial hashes or low-entropy target fingerprints. Later writes
can dirty store-wide sanitation again. Prior completion remains an observation
about its cut, not a certificate for future writes or unknown historical copies.

## Opt-in and existing behavior

Proposed `PhysicalErasurePolicy`: `Autonomous` or `ExplicitMaintenance`, stored
in validated database metadata under the canonical product lock. Autonomous
remains the default profile; explicit mode requires deliberate selection. All
opens must honor the persisted mode; a mismatching explicit open request refuses
before mutation. Reopen cannot silently re-enable background cleanup.

**Old-binary safety is a release prerequisite.** Pre-Slice-85 format-35 binaries do
not know these queue/fence rules. A policy key alone is unsafe. The concrete
proposal is a fresh-store format revision, assigned when implementation is placed,
so earlier binaries refuse the new file before mutation. The feature release
refuses prior-format files; no data migration, metadata-only opt-in of existing
files or dual-format compatibility shim is included. Autonomous and explicit
profiles both use that new admitted format. New-file initialization always writes
validated policy metadata; missing policy in that format is corruption, not an
Autonomous fallback. HITL explicitly approved this deployment cost on 2026-11-04. An
alternative minimum-reader mechanism is acceptable only if existing older binaries
already enforce it, proved with an actual older-binary refusal oracle.

Existing stores/debt remain owned by the old product until completed/exported
through its existing supported procedures. This proposal does not convert them.
Within the new format, a policy change requires no active root debt, physical
closure, redaction debt, maintenance owner or queued request; recorded terminal
receipts remain read-only. Malformed/unknown policy refuses open. Missing FTS clean
metadata still means dirty retained-term history, never a clean certificate.

| Surface | Autonomous policy | ExplicitMaintenance policy |
| --- | --- | --- |
| Existing erase_source/purge/operator erasure | Current success/incomplete and idle/reopen worker behavior retained. | Direct online calls return typed MaintenanceRequired before starting destructive work; the maintenance handle exposes physical operations. This is an explicit configured-profile behavior change requiring approval and binding parity. |
| Ordinary writes with no active physical job | Current behavior. | Current behavior plus narrow guards for queued selectors; no full physical zero proof or cleanup certification is run merely because intent is queued. |
| New queue/status APIs | Queue creation refuses PolicyMismatch. Read retained keyed status if applicable. | Durable queue/status available. No automatic physical worker, query-triggered cleanup or expensive cleanup at open. |
| Existing active journal/tombstone | Current autonomous recovery. | Resume/validate owed state at open without executing physical work. Keep its required mutation fence until an explicit maintenance attempt. No transparent mode conversion. |
| Offline recovery | Existing lock/proof/sink obligations. | Same proof primitives; explicit operator acknowledgement grants a maintenance attempt. No new online SDK recovery-named operation. |

The new queue must not be implemented by dropping requests into the current
singleton RootDebt and merely stopping its worker. Queue intent precedes the
physical zero proof; RootDebt remains the one executing operation. This separates
normal queued service from the expensive post-delete scoped-certification path.

## Proposed API surface

These are the approved contract-level names/types, not existing methods; exact
wire/SDK forms are settled in Slice 85 design within these semantics. Rust is normative;
Python uses snake_case and TypeScript camelCase with identical outcome meanings.
CLI equivalents are a later operator mapping of the same authority, not a second
implementation. No new MCP surface is included in v1.

```text
Engine::queue_physical_erasure(SourceAllAtCut | DeletedLogicalAllAtCut,
                               idempotency_key) -> QueueResult
Engine::physical_erasure_status(ticket) -> Option<ErasureStatus>
Engine::withdraw_queued_erasure(ticket) -> WithdrawResult
Engine::begin_erasure_maintenance(acquisition_wait_ms) -> AcquireResult<Handle>
Handle::queue_physical_erasure(selector, idempotency_key) -> QueueResult
Handle::complete_physical_erasure(ticket) -> CleanupResult
Handle::reattach_original_erasure_sink(ticket, sink) -> ReattachResult
Handle::close() -> ServiceState
```

A handle is bound to one Engine/store/open incarnation, is non-reentrant and
cannot be used through another Engine, after close, or concurrently. Only one
maintenance owner and one executing root job exist. An application can complete
several explicit tickets sequentially in one acquired window; no live all-pending
wildcard silently expands the accepted scope. V1 does not add a background queue
runner, autonomous batch schedule or user-configurable unsafe work-unit knobs.

### Queue admission and immutable scope

The normal operation is visibly named `queue_physical_erasure`, with response
`Accepted { ticket, logical_state: UnchangedByThisRequest }`. It performs no full target materialization, projection drain, physical merge
or checkpoint. Bounded indexed admissibility probes are required. If exact
supported root/closure limits and current resource admissibility cannot be
established cheaply and authoritatively, return NeedsMaintenanceAdmission before
acceptance. The caller may acquire a window and use Handle.queue_physical_erasure
for the heavier preflight, still without destructive effects. Unsupported scope
returns UnsupportedScope, not an accepted permanently unexecutable job. No fast
query plan is assumed: prove actual indexes/effect bounds and measure admission
before advertising a latency envelope.
A busy maintenance owner refuses queue admission rather than enqueueing a
request behind seconds of maintenance work.

The writer transaction records source selector or deleted logical ID, current
committed cursor cut, request version, safe idempotency binding and the queued
mutation fence together. Logical-ID requests retain deleted-first validation;
source validation retains the governed SourceId contract. Admission fails without
a request on invalid input, queue/metadata capacity, unsupported scope or failure
to install complete mutation guards. Proposed maximum is 16 queued requests plus
one executing root. Limits are admission limits, not a completion-time bound.

Admissibility includes the existing exact target-cursor and work/resource limits,
including the 16,384-target cap; a cursor cut alone proves none of them. Do not
admit oversized dependency closure by counting only direct roots. Unrelated growth
while queued can invalidate resource readiness: revalidate the exact envelope under
exclusive maintenance before activation. If it no longer fits, remain Queued with
ResourceEnvelopeChanged and perform no destructive work. The application may
reduce unrelated resources through safe supported operations or withdraw the
queued intent. No resource reservation or guaranteed future executability is
implied by acceptance. This avoids reintroducing full envelope certification on
every unrelated write simply to promise capacity that v1 does not guarantee.

The concrete design must prove capture/ownership for every supported mutation:
roots at the cut remain addressable until maintenance; later source writes,
retargeting, hard deletion, dependency/provenance changes or destructive repair
that could change the selected closure are refused while queued. Unrelated
mutations remain permitted under cheap indexed/small-queue guards. Overlapping
selectors refuse as ScopeConflict unless they are the same retained idempotent
request. No overlap coalescing is included. Queued requests can be explicitly
withdrawn before activation, as specified below. At maintenance, discover
and bind the exact root/dependent cursor/vector scope under exclusivity before
mutation, consistent with the accepted selector/cut. If a complete root/closure
freeze cannot be proved without expensive admission discovery, stop and revise
the contract; do not claim an immutable scope from a cursor number alone.

Queued intent does not change existing read eligibility. Targets may remain
readable under existing authorization; a deleted-first purge target can already
be excluded for independent lifecycle reasons. This is
not a revocation fence and must not alter ranking/statistics. Authorization changes
still obey their current contracts. Reads cannot bypass a later committed
physical/logical removal by using the queue's older cut or an old evidence token.

Idempotency keys are caller-supplied random opaque values; never derive them from
subject identifiers. While a request record is retained, the same key and exact
selector returns the same request; conflicting pending payload returns
IdempotencyConflict. At the identity-free finalization boundary scrub the selector/fingerprint and
retire the key for new admission, while the ticket remains retryable for actual
completion. Retain only opaque key/ticket and safe outcome fields. Reusing that retired key
for queue admission returns KeyRetired, never another argument's completion;
status(ticket) retrieves the receipt. Terminal receipt storage is bounded by an
explicit metadata budget: exhaustion refuses new queue admissions, never drops
owed work. Retention/archival relief requires a reviewed policy, not silent
pruning that breaks idempotency. No store-wide listing is introduced.

`withdraw_queued_erasure` is an explicit escape from pending intent, not erasure
success. Under the same coordinator/transaction as activation it succeeds only
while Queued with no logical removal and no executing RootDebt. It removes queued
scope guards and target-bearing intent, and returns terminal WithdrawnBeforeRemoval
with logical state unchanged. A racing activation wins exclusively: withdrawal
then returns AlreadyActivated without clearing any debt/fence. No cancellation of
an executing or logically removed request is offered. Withdrawal makes no physical
claim about the discarded queue metadata/WAL. Its content-free receipt is distinct
from Complete; the idempotency key is retired, and a later request needs a new key.
An unsupported queued intent therefore need not freeze its scope forever.

Tickets are store-bound opaque references, not independent authorization tokens.
Every call requires the Engine's existing access authority; wrong-store and
unknown well-formed tickets return no status without disclosing a selector.
Logs/debug/errors/status omit target identities. On uncertain admission COMMIT or
lost response, retry the same key; do not imply refusal or rollback from transport
failure. Invalid/corrupt durable state fails closed and remains an operator issue.

### Maintenance acquisition

Applications explicitly call begin at a chosen quiet time. Close admission for
new product snapshots, writes, ordinary queue additions, admin/lifecycle and publication;
serialize any running erasure/service turn; drain previously admitted owned
reads/writes/projection work. Existing reads run to completion; no cancellation
is introduced. New ordinary calls fail fast with MaintenanceBusy, rather than
waiting silently for the whole cleanup. A request already admitted before gate
closure is accounted for in the drain and may finish.

`acquisition_wait_ms` bounds only the cooperative wait to acquire/drain owned
work. Zero means try once. It is not an active SQLite-command deadline or a
hard OS scheduling guarantee. If acquisition expires, release temporary gates
and return Busy with safe counts/owner categories; no destructive erasure began.
External WAL readers cannot be excluded by this handle and may block finalization.

Only after all owned mutable activity is drained is Handle returned. The handle
may admit an explicitly selected request through its heavier preflight; this is
the sole queue-creation exception while the maintenance gate is held. The
implementation must define one lock order covering Engine close, maintenance,
primary writer, projection workers, reader admission and original telemetry sinks.
An application-held owned snapshot in the same calling context returns a clear
SelfPinnedReader/Busy result rather than deadlocking. Nested maintenance and
concurrent handle use refuse. Close serializes against an active handle operation;
it does not promise cancellation or a short wait for a running command.

### Physical execution and completion

1. Validate ticket/store/policy, current queued fence and source/cut authority.
   Inspect known blockers (original sink attachment/authentication, drain state,
   resource envelope and schema) before destructive work. This reduces avoidable
   partial progress but cannot rule out later I/O, checkpoint or process failure.
2. Capture exact registered closure/vector scope and native report. Move queue
   intent to the existing executing debt authority atomically with native root/
   canonical/projection deletion and structural proof. Delete atomically; never
   split this measured fast phase merely to satisfy a budget parameter.
3. Run existing durable sanitation continuation and original-sink redaction under
   exclusive admission. No unrelated survivor write can interleave, so no
   repeated foreground safe-write certification is needed in this policy.
   Preserve every exact proof and existing conservative recovery invariant.
4. Before final checkpoint, remove new target-bearing queue data as well as
   temporary denial/retry identities and persist the validated identity-free
   finalization authority. Completion still requires actual WAL TRUNCATE.
5. Record a content-free terminal receipt only after all required physical
   obligations discharge. A crash between checkpoint and bookkeeping leaves
   idempotently retryable debt, not speculative success. Receipt bookkeeping
   cannot reintroduce target identity. Release the same-source queued restriction
   only at terminal completion; subsequent reingestion is a new authorized event.

Physical state may include earlier store-wide retained ghosts. Finish required
sanitation over all registered families before certifying this pass, preserving
authorized survivor history. Report target completion and that sanitation cut
separately. Unknown old telemetry sinks cannot be reconstructed or certified.

### Blocker resolution and offline reconciliation

The mutation fence has a narrow recovery exception: authenticated reattachment of
an **already-recorded original sink**, owned by the maintenance/recovery coordinator.
Handle.reattach_original_erasure_sink validates the canonical identity/fingerprint
and original registration before attachment. It does not register a new sink,
substitute current configuration, enable capture or mutate target/projection state.
The existing enable_telemetry path performs registry writes and is not a permitted
bypass. Sink attach/rewrite/checkpoint lock order must be explicit. With multiple
original obligations each must be authenticated and discharged through shared
primitives; no guessing, clearing or skipping an unavailable original. A read-only
status can tell the operator that reattachment is required without revealing its
path. If authority is absent/unbound, return OriginalSinkUnverifiable and keep debt;
repairing that authority is a separate operator decision, not caller override.

Offline `--complete-erasures` completes **activated debt only** under the canonical
product lock. It must never implicitly activate or delete a merely Queued intent.
An activated API ticket is bound to the root/journal authority, so offline completion
uses the same state/receipt finalizer as the handle: after actual proof/redaction/
TRUNCATE, root/closure cleanup and terminal ticket publication commit atomically.
A crash after checkpoint but before that commit leaves replayable identity-free
fenced debt. A crash after it leaves one Complete receipt and no active root; no
orphan Executing ticket or root-only success. Offline dirty-only sanitation remains
a distinct operation and cannot mark a queued target Complete. Offline completion
is an explicit operator-granted attempt, not a way to re-enable an idle worker.

There is no active `timeout_ms` promise in v1. An attempt is finite in retry count
and may return at an existing safe unit boundary or known blocker. It cannot
promise to interrupt any active SQL/OS call within N milliseconds. Caller drop
or transport cancellation does not revoke an accepted request or cancel an
executing command; the owned operation finishes to a safe boundary and retains
durable status. Physical success remains conditional on completing all obligations.

## Result and blocker contract

`CleanupResult` is `Complete { receipt }` or `Incomplete { status }`.
Domain blockers are inspectable outcomes, not success. Validation, corruption,
unsupported/malformed state and storage failures are typed errors; callers use
status afterward because an error after commit can still leave owed work.

`ErasureStatus` has `phase`, `logical_state`, `blocker`, `service_state` and safe
progress counts. It contains no raw selector, source/body IDs or approximate ETA.
Phases: Queued, Executing, Sanitizing, Finalizing, Complete, WithdrawnBeforeRemoval. Logical state is
UnchangedByThisRequest or Removed; it never advances before deletion commits.
Blocker is orthogonal to phase, and retries cannot reset the accepted scope.

| Blocker/outcome | Durable effect | Service on handle release/reopen |
| --- | --- | --- |
| Acquisition busy / owned reader drain expired | Request remains Queued; logical state unchanged; no erasure executed. | Normal reads/writes resume subject to queued scope guards. |
| Preflight sink unavailable/wrong/unbound/multiple unresolved originals | Request remains Queued unless an earlier attempt already committed removal. Never guess a replacement sink. | Queued case: normal service. Existing executed debt: writes remain fenced. |
| Unsupported envelope/scope or scope conflict before acceptance | Refusal, no request or destructive effects. | Prior service state retained. |
| Resource envelope changed since queuing | Intent stays Queued; no deletion, no executing debt; report ResourceEnvelopeChanged. | Normal queued service; explicit withdrawal remains possible. |
| Failure before logical-delete commit | Transaction rolls back; queued intent remains if already accepted. | Normal queued service, unless earlier durable debt exists. |
| Sanitation/telemetry/proof/storage failure after removal | Logical state Removed and exact unfinished debt remain. No partial success or implicit restoration. | Coherent reads may resume under existing post-delete contracts; all mutations remain conservatively fenced until explicit completion. |
| External WAL reader / checkpoint busy after redacted tombstone | Identity-free debt and persisted global mutation fence remain; logical state Removed. | Releasing temporary reader gate allows reads; writes stay fenced until explicit retry completes. |
| Process crash or Engine close | Durable Queued/executing/finalization state survives; speculative state does not. | Explicit-mode open validates and reports state, performs no autonomous physical work; active debt retains fences. |
| Missing/corrupt authority or unknown phase | No invented completion; fail closed, preserve evidence. | Refuse unsafe open/mutation; operator diagnosis/recovery required. |

Service states are NormalWithQueuedRestrictions, MaintenanceOwned,
ReadsAllowedWritesFenced and ClosedOrUnavailable. Handle.close/drop releases
only ephemeral ownership/gates; it cannot clear a durable fence. Drop is not a
physical-completion result. After a blocked post-delete attempt, the application
may therefore face a prolonged write outage until the blocker is resolved and
maintenance is explicitly retried. That is a disclosed failure-mode tradeoff,
not a claim that maintenance can always restore normal service on demand.

## Acceptance before shipping this approach

- Older binaries refuse the new format before modifying policy/queue/data. Prior
  formats are refused by the feature release; no metadata-only or backwards-readable
  deployment assumption. Profile changes refuse while debt/queue exists.
- Queue-only operation performs no native deletion, full physical proof or
  sanitation; exact source/cut/closure guards cover all writer/admin/rebuild paths.
  Show ordinary unaffected operations and score/order/history semantics remain
  correct while Queued. Same-source/closure-changing paths refuse atomically. Bounded admissibility,
  oversized closure, later envelope growth, heavy preflight and the withdrawal/
  activation race follow their stated refusal/queued outcomes.
- Queue receipts, conflicting retries, unknown/wrong-store references, metadata
  capacity and lost/uncertain admission responses follow the contract on real DBs.
  Target-bearing queue/retry data is absent from main/WAL/registered sinks after
  complete; terminal receipt contains only approved content-free fields.
- Maintenance ownership gates every route, drains admitted work without deadlock,
  refuses newcomers promptly and excludes projection/telemetry races. Test same-
  context pinned readers, concurrent begin/close and stale handles.
- Independent survivor-only/physical witnesses, randomized retry sequences and
  process exits at queue/activation/unit/redaction/checkpoint/receipt boundaries
  prove exact scope, rollback, no resurrection and no false physical success.
- Busy original sinks and external readers retain the stated fences on handle
  drop/reopen; retry converges after actual resolution. No worker restarts in
  explicit mode. Autonomous mode retains its behavior. Old-format debt is never
  silently converted. Offline activated completion publishes the same receipt, and
  original-sink reattachment remains possible without registry/capture bypass.
- Measure request admission, ordinary-operation outcomes/latencies with a bounded
  queued backlog, acquisition refusal/drain, actual maintenance blackout, native
  deletion, full proof/sanitation, finalization and idle/no-erase controls.
  HITL accepts actual observed timings; the reviewed bounded screen reports
  samples/medians/ranges and every incomplete/refusal outcome, without inventing
  a regression threshold or claiming p99 from insufficient samples.
- Idle 1–3s and loaded 3–15s historical observations motivate this design only.
  Neither is a new API estimate, maximum or guarantee. Exclusive-window benefit
  and normal admission were qualified only within the small-store screen
  recorded in status; no broader performance guarantee is inferred.

## HITL approval — 2026-11-04

HITL approved: (1) explicit persisted opt-in policy, (2) queued intent with
visibility unchanged until maintenance, (3) exclusive maintenance/fast newcomer
refusal, (4) typed Complete/Incomplete with durable post-delete write fences,
(5) compatibility matrix and no deadline claim, (6) fresh-store format break
and queue-admissibility/withdrawal semantics. Immediate low-latency logical
revocation is excluded from this approved v1. The original end-of-83 guarantee/API
decision is resolved by this approval and assigned to Slice 85; no numerical
latency/SLA or physical deadline is selected.

## Astra review record — 2026-11-04

APPROVE for HITL proposal review, no remaining blocking findings. The first pass
required exact admission/escape for unsupported queued scope, offline ticket
reconciliation, an authenticated original-sink recovery exception and protection
from older binaries. All four are included above. Queued logical state is
UnchangedByThisRequest, including already-excluded deleted-first purge targets;
no new visibility or revocation guarantee is inferred. Astra approval is not HITL
contract acceptance or implementation authorization.

HITL subsequently approved this reviewed contract on 2026-11-04 and directed a
separate clean Slice 85. Concrete design/ADR/interface mapping, implementation,
metadata limits and independent qualification were completed with Astra approval.
The status record preserves the small measured envelope and hosted CI limits.
