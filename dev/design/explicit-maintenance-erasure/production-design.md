---
title: Explicit maintenance erasure — production design
status: ACTIVE
date: 2026-11-04
target_release: 0.8.28 Slice 85
---

# Production design

Implements the approved [contract](contract.md). No immediate revocation or
physical deadline is introduced. Requirements and test mapping are in the
[Slice 85 plan](../../plans/0.8.28/features/slice-85/plan.md).

## Reconciliation

1. Draft baseline `0db97db08` gained only Slice 85 preparation at `db1fdb6dc`;
   retained product is still Slice 83 plus accepted fixes after rejected 84/84b.
2. `physical_erasure::RootDebt` is an executing singleton; it cannot hold queued
   intent. `PhysicalMaintenance` currently starts its idle worker on every open.
3. `PhysicalReaderAdmission` excludes only reader transactions at finalization.
   It does not gate admin, write, telemetry or projection ownership. Existing
   `ensure_open` is only a closed flag. A second reader flag is insufficient.
4. Existing `envelope` scans all canonical bodies, vectors and nine metadata
   families. There is no transactional indexed whole-store envelope certificate.
   Therefore first-time ordinary queue admission must return
   NeedsMaintenanceAdmission. Exact pending-key retry remains cheap. Heavy
   admission through the handle is the usable v1 path, not a hidden scan on open.
5. Existing plain-write denial covers source, stable identity, old/new edge
   endpoints and prior superseded rows; reuse its predicates for queued scope,
   but do not invoke its physical zero/envelope certification for queued writes.
6. Current format is 35. Production admits current-only; format 36 is unused in
   this release. The format-35 executable frozen independently at
   `/tmp/slice85-old-binary/fathomdb-format35` supplies the older-binary oracle.
7. Owner accepts empirical timings observed in testing (2026-11-04). No new SLA,
   numerical regression threshold or percentile assertion is inferred.

## Format and policy

Fresh stores use format 36 with additive fresh-schema queue/receipt tables.
Current-only admission refuses 35 before opening a mutating connection; no
production upgrade route is added. Migration-test hooks remain test-only.
Policy metadata is required even for Autonomous, has exact supported version and
value, and is validated before boot reconciliation or worker startup. Missing,
unknown or malformed metadata is corruption. EngineConfig has optional explicit
policy request: absent honors persisted policy, fresh absent selects Autonomous;
explicit mismatch refuses without mutation. The effective policy is immutable
for an open Engine. No policy-change API is added in v1.

Store identity is a freshly generated opaque random 128-bit nonce persisted on
initialization. Tickets and keys are 128-bit opaque random hex strings. Ticket
wire format includes store nonce; unknown/wrong-store well-formed tickets return
None. Invalid syntax is a typed validation error without echoing the argument.
All public statuses and Debug implementations omit target identities.

## Durable queue and identity

Queue entries contain request version, opaque key/ticket, exact selector, committed
cursor cut, scope-denial identities and exact expected root/dependency cursors.
They are distinct from RootDebt. One writer transaction inserts intent and guards.
Maximum 16 queued plus one executing; terminal keys/receipts are capped at 4096
records and total queue metadata at 8 MiB. Exhaustion refuses new admission;
existing retries/status/cleanup remain possible. No pruning or archival API.
Keys must be random application values; syntax validation cannot prove randomness.

Existing key lookup precedes new resource admission: exact retained selector
returns the same Accepted ticket; other selector returns IdempotencyConflict;
terminal or identity-free Finalizing key returns KeyRetired. Never return another
request's success; finalization retries use the original ticket. Unknown
well-formed status gives None. Status contains phase, logical_state, blocker,
service_state and safe counts only. Pending rows validate strictly on open.

Heavy admission holds the exclusive window and primary transaction. It applies
existing resource envelope, selector validation/deleted-first and exact native
closure preview. Extract a read-only scope planner from the current purge/source
paths so admission and deletion cannot disagree. Count all root, touching-edge,
dependency and retained-version cursors against 16,384; root-only count is invalid.
Capture source and stable identities before deletion. Reject unsupported closure
or overlap; do not coalesce. Scope overlap uses exact cursors plus source/identity
and endpoint dependencies, including absent selectors that future writes could
otherwise populate. Record cut under the same primary transaction.

At activation re-plan and compare exact immutable closure; recheck the envelope.
Changed unrelated resource envelope returns Queued/ResourceEnvelopeChanged before
any deletion. Mutation or corrupt durable authority is a typed error, not a new
scope. Withdrawal and activation serialize under primary/coordinator transaction;
withdrawal clears selector/guards and retires key as WithdrawnBeforeRemoval.
Activated withdrawal returns AlreadyActivated and cannot touch debt.

## Mutation effect inventory

| Owner/path | Queued rule | Executing/finalizing explicit rule |
| --- | --- | --- |
| Plain node/edge write, supersession, batch | Check submitted source/stable id, both endpoints, and actual prior rows after earlier batch effects; custom-trigger fallback refuses when queue exists | Global mutation fence |
| Provenanced writes/source versions/links/dependency registration | Check affected source and full prospective/prior dependency closure; refuse when disjointness cannot be established | Global fence |
| Lifecycle transition, consolidation, actuation, operational state | Validate every embedded operation against selected/prior closure in the same transaction; unprovable effects refuse atomically | Global fence |
| Purge/source/operator erasure | Explicit direct API returns MaintenanceRequired; handle uses selected ticket only | Only owning ticket continuation |
| Projection config/rebuild/mean/vector administration | Changes affecting queued closure or global registry refuse; unrelated safe metadata may proceed when exact noninterference is proven | Global fence |
| Projection jobs/publication | May complete existing queued rows without changing canonical closure; no new identity or dependency authority | Frozen/drained before handle; no publication |
| Telemetry enable/feedback/capture | Normal gate and existing registry semantics; no visibility change | Capture excluded; original authenticated reattachment only |
| Offline recovery | Product lock excludes live Engine; queued intent is never activated | Shared finalizer may complete activated ticket |

Queue checks are separate from physical-journal checks. Existing guards cannot
mistake queued work for structural zero. Every supported mutator receives a
transactional guard; a broad public-entry guard alone is not a scope freeze.
Rebuild and provenance paths that cannot prove cheap noninterference refuse;
this limitation is explicit in SDK docs and queue outcomes.

## Coordinator and lock order

A per-Engine admission coordinator holds `{closing, acquisition, owner, active}`
under a short mutex/condition variable. Ordinary public operations hold RAII
admission through their last SQLite/telemetry effect, including reader replies.
Nested internal calls reuse thread-local admission for the same Engine rather
than being refused mid-operation when acquisition closes the gate. Callback
entry is marked separately from internal nesting; a subscriber/profile/provider
callback never inherits the handle permit or permission to start a new operation.
Every subscriber dispatch, reader-worker callback, projection callback and
caller-provided Embedder call installs callback-context RAII. Public begin/close
from callback context returns Busy before any wait or worker join. Public
ordinary callback reentry is refused while acquisition/maintenance is gated. No admission
mutex is held during SQLite, callbacks, provider waits or file I/O.

Acquisition atomically closes newcomers, drains admitted operations using only
acquisition_wait_ms (zero try-once), then freezes/drains projection owners under
the remaining acquisition budget. A same-thread active operation/callback returns
SelfPinnedReader/Busy before waiting. Concurrent/nested begin returns Busy.
Failure releases temporary gates and restores projection service. No deletion
occurs before a handle is returned. All newcomers return MaintenanceBusy promptly.

Order: coordinator admission → close serialization/physical service turn →
projection freeze/drain → reader admission/drain → primary writer → telemetry.
Do not wait for an operation while holding its required primary/telemetry lock.
Physical worker participates in ordinary operation ownership in Autonomous mode;
ExplicitMaintenance never starts that worker. Projection threads participate in
freeze/drain before handle grant; no extra writer connection is opened.

The maintenance handle is owned, non-cloneable and holds Arc<Engine> plus an
open-incarnation generation/token. Engine begin is callable on Arc<Engine>.
Python/TypeScript bindings already retain Arc Engines. Rust SDK core storage
changes internally to Arc while preserving its existing borrowed core() API. Handle methods validate token and serialize
with a try-lock: concurrent calls refuse. A scoped internal permit admits only
that handle operation through normal checks, never all calls on the creating
thread. Drop releases ephemeral ownership only. Engine close marks closing,
serializes with active handle operations, revokes idle ownership, joins workers,
then closes connections; later handle use returns ClosedOrUnavailable.
Caller/async drop never drops an in-flight native operation's owned Arc/permit.
Close and acquire linearize under the coordinator: closing wins by marking closing
and invalidating pending acquisition before taking close_lock; acquisition checks
closing again before handle publication. Handle operations take the operation
mutex before primary; close waits for that mutex after marking closing, and never
waits for the idle handle object to drop. Worker-origin close/begin returns Busy
before joining any owner. Maintenance SQL uses internal_debt_probe to suppress
profile callbacks and never emits public lifecycle callbacks under its permit.
Admission remains alive through public operation completion/telemetry and final
lifecycle events, not merely through write_inner.

### Entry-point and worker coverage

Admission is taken at the outermost write/search/graph/read/evidence/frozen/page,
actuate, lifecycle, dependency/provenance, projection configure/rebuild/mean,
operator and drain entry, including events after inner helpers return. Audit
methods lacking ensure_open explicitly: enable_telemetry takes admission and a
transactional mutation guard BEFORE file create/touch; record_feedback takes
admission and debt guard BEFORE append; profiling/threshold setters take admission
before atomics. Status and safe in-memory config/counters/path observation are
explicit nonmutating exceptions; queue/status/withdraw and handle methods have
their own token-aware admission. No public SQL test hook is a production bypass.

Projection worker commit and scanner own counted worker turns; freeze waits for
accepted work and primary publication, then verifies idle before granting owner.
Autonomous physical worker owns a counted turn; it is absent in explicit mode.
All actuation suboperations (plain/provenanced write, dependency registration,
lifecycle transitions, receipt insertion and closure effects) run queue guards
inside the shared transaction before commit. Per-row plain guards run after prior
batch effects. Queue state may be cached only as a validated presence hint; exact
scope checks use the transaction's durable authority.

After explicit activation commits, set an in-memory write-fenced flag from the
same durable outcome before any postcommit faultpoint; reopen derives it from
validated root/ticket authority. All mutators check durable fence transactionally.
Read admission may resume after handle release, but read-triggered telemetry
capture is suppressed while that durable fence exists; no returned result is
appended to a sink during pending physical cleanup. Feedback and telemetry enable
refuse instead of appending/registering; original handle reattachment is the sole
sink exception. This keeps allowed reads from silently reintroducing target data.

## Activation, sanitation and terminal receipt

Refactor native deletion minimally: the existing deletion transaction also marks
the selected ticket Executing and installs RootDebt bound to opaque ticket.
No commit gap or executing ticket without its root. Existing proof, redaction,
FTS sanitation and checkpoint primitives remain the sole physical authority.
No certified unrelated writes are admitted once an explicit ticket activates.

Before identity-free tombstone commit clear the selected ticket's selector,
cut/cursors/denial/retry material inside the same transaction as redacted root
publication. Keep opaque key/ticket, safe counts and Removed. Other queued
selectors remain separate owed copies and cannot refer to the erased scope by
the overlap rules. Actual TRUNCATE follows. `physical_erasure::finalize` atomically
finishes closure/root and publishes Complete for that ticket in one commit.
Offline completion calls the same hooks, never activates Queued. A crash before
receipt commit retains replayable identity-free debt; after commit one terminal
receipt exists. No separate bookkeeping success window.

Heavy queue admission records the existing original-sink fingerprint inventory
with the accepted scope. This is not a cap on later original obligations: normal
queued service may legitimately register/capture another sink. On each queued
reattachment and immediately before activation, monotonically union the current
validated registry into that inventory under primary ownership. Never remove an
original or replace it using current capture configuration. No ordinary operation
can register/capture concurrently after handle grant. Before activation every
member of that final union must be available/authenticated. Handle reattachment is legal for Queued as well as
activated tickets: canonicalize/fingerprint an existing file, require membership
in the monotonically extended original inventory and persistent registry (activated debt
additionally authenticates its existing obligations), then retain the attachment
in the handle's fingerprint→path map. Multiple attachments coexist; replacing one
must not discard another. Queued attachment performs no redaction. It never
creates a path, enables capture or inserts registry membership. After activation,
iterate retained originals through the existing discharge primitive; each original
is authenticated and retained independently until its actual obligation commits.
A retry after reopen reconstructs attachments explicitly from already-recorded
originals. Wrong/unbound/missing sink is a blocker, never an activation escape.
Test a queued request with two original registered sinks and no preexisting debt:
attach both, activate once, discharge both, then checkpoint and publish receipt.
Also queue first, register a second sink during normal service, capture target
results there, attach both originals, and prove both are clean at completion.

Cleanup errors after commit do not imply rollback. Status derives Removed from
durable activation. Expected sink/checkpoint/resource blockers return Incomplete;
validation/corruption/storage faults are typed errors and status remains readable.
Handle release and reopen preserve global post-delete fence, including redacted
root with no source hash. Explicit reopen only validates authority, without graft,
sanitation, redaction, checkpoint or implicit queue activation.

## Tests and measurements

Behavior-specific RED/GREEN commits: format/policy; coordinator ownership; queue
identity/scope; activation/finalization; recovery/sinks; SDK parity. Keep existing
physical positive controls and independent survivor oracles. Add generated queue/
withdraw/retry/write histories with hand-authored immutable-scope invariants and
real process exits at admission/delete/redaction/checkpoint/receipt boundaries.
Tests cover every mutation-owner row above, old/new error precedence and stale
handle/close/callback drain races: close/begin from reader/profile/provider
callbacks, concurrent close/acquire at both linearization points, two recorded
originals with no prior activated debt, maintenance SQL subscriber reentry,
post-delete reads with capture enabled, and all actuation suboperation effects. Actual frozen format-35 binary opens an actual
new format-36 file and must refuse without main/WAL mutation; reverse admission
uses a real format-35 file. Do not replace this with an edited user_version alone.

After correctness and code review, a bounded empirical screen uses three fresh
small real stores per policy/control cell, 100 normal operations per cell,
0/1/16 queued intents, exact retries, idle acquire/release, a held-reader drain,
newcomer refusal and one full physical completion per applicable store. Include
external-reader and original-sink incomplete/retry controls. Capture every result,
wall time, current/peak RSS and scratch size; report samples/min/median/max only,
no p99 or inferred SLA. Stop any cell at 120 seconds and total screen at 15 minutes,
4 GiB RSS or 2 GiB scratch. These are harness ceilings, not product deadlines.
No timed window overlaps a build/test. Retain failed/incomplete outcomes.

Astra design review APPROVE (2026-11-04): callback ownership, complete entry
coverage and monotone multi-original sink obligations resolved before code.
