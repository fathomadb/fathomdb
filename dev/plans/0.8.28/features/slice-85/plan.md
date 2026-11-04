---
title: FathomDB 0.8.28 Slice 85 — explicit physical-erasure maintenance
status: ACTIVE
target_release: 0.8.28
---

# Slice 85 — explicit physical-erasure maintenance

HITL approved the [Astra-reviewed contract](../../../../design/explicit-maintenance-erasure/contract.md)
at `0db97db08c912cc7ca634b5dc9e9295414d4afe2` on 2026-11-04 and assigned its
implementation to a separate clean slice. Dependency is completed Slice 84,
including 84b; Slice 90 follows 85. Implementation is NOT_STARTED.
The [approved outline](../../../../design/explicit-maintenance-erasure/implementation-plan.md)
is the detailed work-order reference. This slice adds testable acceptance and a
clean execution handoff without duplicating or weakening that contract.

## Approved scope and boundaries

- Persist an explicit opt-in maintenance policy; Autonomous remains the default.
  Explicit-mode reopen does no autonomous physical cleanup and preserves owed fences.
- Queue bounded immutable intent with logical state UnchangedByThisRequest. This
  is not fast logical revocation; reads retain their existing eligibility until
  committed removal. Same-scope mutation restrictions install with acceptance.
- Explicit exclusive maintenance gates newcomers, drains owned work and performs
  atomic native deletion plus existing exact sanitation/sink/WAL finalization.
- Preserve physical-success meaning and truthful blocked/incomplete results,
  including possible durable write fencing after handle release or process reopen.
- Include queued-only withdrawal, heavy admission inside maintenance, original-sink
  reattachment and offline activated-ticket receipt reconciliation.
- Implement the approved fresh-store format break. Older binaries must refuse
  new files; the feature release refuses prior formats. No migration, metadata-only
  opt-in of old files, dual-format shim or existing-debt conversion.
- No deadline/SLA, active-operation timeout, reader cancellation, immediate source
  revocation subsystem, new MCP surface or optimization campaign. Rejected 84/84b
  implementations stay rejected. No tag/publication authorization.

## Reconciliation before design

Preparation baseline is `0db97db08`; it contains retained production after 84b
rejection, the durable findings and approved proposal. At execution verify actual
branch ancestry, clean ownership and final dependency receipts rather than assume
this hash is current. Read memory/ADR/interface authority and the approved scope.
Run preflight against the active release state before creating any worktree.
Use a clean isolated implementation checkout if another writer is active; do not
share a checkout, borrow another checkout's venv or stage unrelated files.

Enumerate changes since this draft and inventory current owners of queue/policy,
canonical/source/dependency writes, root debt, projection/publication, telemetry,
readers, close and offline completion. The current singleton journal is executing
physical authority, not a pre-deletion request queue. No queue acceptance may
pretend structural zero or affordable whole-store certification already exists.

Confirm the next unused format version at that point; do not assume a numeric
version here. Update schema/admission docs, release-state schema metadata, rebuild
fixtures and downstream release-format expectations when the format actually lands.
Current metadata stays at actual version 35 during this preparation. Preserve the
full old-binary refusal oracle; editing only a version fixture is insufficient.

## Slice requirements and acceptance

S85 labels are local slice acceptance, not new canonical product AC identifiers.

| ID | Required outcome and decisive acceptance |
| --- | --- |
| S85-1 format/policy | Fresh-store initialization records valid policy; absent/malformed/mismatching policy fails as contracted. Actual older binary refuses new files, new binary refuses prior formats before mutation. Autonomous regressions retain behavior; no transparent debt conversion or explicit-mode worker on open. |
| S85-2 queue/scope | Atomic bounded intent/fence, exact selector/cut and full closure freeze; complete mutation-effect inventory. Unsupported/unprovable cheap admission returns refusal/NeedsMaintenanceAdmission before acceptance. Exact retry/conflict/retired-key/store identity, capacity and heavy-window admission match contract. Unrelated growth stays queued on envelope change; withdrawal races activation atomically. Queued intent changes no read/statistics/visibility semantics. |
| S85-3 maintenance owner | One Engine/store-bound handle gates all read/write/admin/queue/publication owners, drains admitted operations and promptly refuses newcomers. Acquisition wait applies only before active cleanup, no active SQL deadline. Same-context snapshot, nested/concurrent begin/close, stale handle and dropped caller do not deadlock or leak ownership. |
| S85-4 physical activation | Queue→RootDebt and atomic deletion/proof commit together. Positive node/edge/property/v2, vector/receipt/sink/main/WAL witnesses and exact survivors pass. Temporary queue/source/retry data is removed before identity-free checkpoint; only actual finalization produces Complete. No hidden physical work from normal queue/read/write/open. |
| S85-5 incomplete/recovery | Real exits/rollback/commit failure at queue, activation, unit, redaction, checkpoint and receipt produce truthful status with immutable scope. Handle drop/reopen cannot clear durable post-delete mutation fences. Pinned readers, wrong/unavailable/unbound/multiple original sinks and corrupt authority never report completion. |
| S85-6 operator/sink | Only authenticated already-recorded original reattachment is admitted through recovery; no registry substitution/capture or mutation bypass. Offline completion handles activated debt and publishes the same ticket receipt atomically; it never activates queued intent. Original operator acknowledgements/lock/proof remain mandatory. |
| S85-7 SDK/contract | Rust/Python/TypeScript config, queue/status/withdraw/handle/result behavior and error precedence agree. Explicit-mode direct erase/purge return MaintenanceRequired before destructive work. Public docs, successor ADR and interface deltas land together. No accepted-only success or accidental public recovery listing. |
| S85-8 qualification | Measure normal operations with no/queued work, acquisition/drain/newcomer refusal, exclusive blackout and actual physical completion independently. Freeze approved thresholds and protocol before timing; preserve all failures/incomplete attempts. New API latency is observed, never inferred from historical 1–3s idle or 3–15s loaded results. Report exact final gate provenance and supported failure/platform envelope. |

## Concrete design gate before code

Write the exact production design, successor ADR draft and test mapping, and have
Astra review them. Approval of the product contract is complete; this gate checks
implementation correctness and does not reopen that approval by default.

Settle these implementation choices explicitly:

1. Policy/config wire shape, new format value and initialization/refusal paths;
   metadata/terminal-receipt budgets and exact key/status error precedence.
2. Queue selectors/cuts, bounded indexed admissibility, all supported writer/admin/
   rebuild/provenance effects, overlap and closure freeze. If the promised cheap
   path cannot prove safety, use the approved maintenance-admission fallback.
3. Maintenance/close/primary/projection/reader/telemetry lock order, ownership and
   newcomer admission points. No extra live writer connection or worker race.
4. Queue activation, original journal reuse, receipt finalizer, original-sink
   recovery exception and offline reconciliation, including identity scrubbing.
5. Public handle lifetimes, async/caller-drop behavior, errors and Rust/Python/TS
   parity, documentation/examples that never label queued data erased.
6. Measurement sample size, offered workload, normal-service tolerance, acquisition
   wait, per-cell/total time and RSS/scratch ceilings. The approved contract retains
   HITL agreement on numerical qualification thresholds/budget before timing;
   preparing this slice does not invent or authorize another campaign.

Any broader public behavior, instant-revocation requirement, migration, weaker
physical proof or changed guarantee returns to HITL as a concrete scope change.
Ordinary design choices within the approved contract proceed through Astra review.

## RED/GREEN and verification sequence

Follow the [six implementation groups](../../../../design/explicit-maintenance-erasure/implementation-plan.md#proposed-behavior-specific-redgreen-phases):
policy/format; queued identity/fences; maintenance ownership; physical outcomes;
crash/reopen and bindings; independent physical qualification. Each new behavior
has its own observed staged/committed RED before GREEN. Existing passing safety
oracles remain guards, not fabricated failures. Tests stay read-only during fixes.

Use real SQLite files, generated state-machine histories with human-authored
invariants, positive physical controls, exact independent survivors and targeted
rendezvous. Exercise old/new endpoints, same-batch writes, retained history,
configured property FTS, mapped non-head vectors, original sinks, pinned external
readers and actual process exit. No mocked database or regenerated golden oracle.

Astra reviews code; an independent verification subagent reruns decisive RED/GREEN,
physical/rollback/reopen, ownership/close and SDK/operator witnesses and checks
measurement accounting. Reuse existing fixtures/tools; no new general framework.
Apply the [quality roadmap](../../../../notes/fathomdb-quality-testing-roadmap.md):
composition faults, boundaries/differential checks and resource cleanup. Defer
exhaustive VFS/OOM/MC/DC and storage-remanence/backup claims.

Run scoped lint, typecheck and tests in latency order while working. Run one final
full `agent-verify.sh` for the completed code phase on its clean checkout-owned
venv. Narrow fixes rerun failed/affected suites; record assembled/resumed checks
honestly. Test platform WAL and native SDK/packaging paths affected by the new
ownership/format. GitHub CI inspection/dispatch uses `gh` only.

## Separate-run completion and handoff

Update [status](status.md) with S85-1–8 results, exact implemented contract/format,
Astra verdicts, independent verification, normal/maintenance measurements,
blockers/platform/resource limits and full/scoped gate provenance. Update state
and regenerate views from JSON, not generated blocks. Land into the approved
release line, remove any task worktree/branch after clean landing, and leave
unrelated files/caches untouched. Bring outcomes back to HITL. Slice 90 follows;
publication and any new guarantee outside this approved contract remain separate.

This session prepares the slice only. The implementation session starts with
baseline reconciliation and concrete design, not an assumed ready-made worker
or a new optimization experiment.

## Execution reconciliation — 2026-11-04

Actual clean baseline is `db1fdb6dc` on existing `dev/0.8.28`; no new worktree or
branch is needed. [Production design](../../../../design/explicit-maintenance-erasure/production-design.md)
enumerates seven changes/facts since the draft and the complete owner/effect
inventory. Draft scope is retained; the expensive existing envelope means new
normal-path admission uses NeedsMaintenanceAdmission and usable handle admission.
No purported constant-time envelope certificate is added. Owner accepts actual
observed testing timings; the design freezes a bounded empirical screen without
an invented SLA or p99 claim. Successor ADR is proposed pending Astra review.

| Acceptance | New decisive tests | Retained guards |
| --- | --- | --- |
| S85-1 | fresh36 policy validation/mismatch; real old executable/new file and reverse admission | current-only refusal, Autonomous idle/reopen |
| S85-2 | exact retry/conflict/retired/capacity/store; full-closure mutation histories; changed envelope; withdrawal race | plain write prior-row/endpoint/trigger guards |
| S85-3 | admitted reader/admin/write drain; fast newcomer refusal; self-pin/close/stale/concurrent handle | reader/worker connection ownership suites |
| S85-4 | atomic ticket/root/delete; queue identity absent from main/WAL; receipt after actual TRUNCATE | four-family FTS/vector/survivor controls |
| S85-5 | process exit and rollback at queue/delete/redaction/checkpoint/receipt; durable reopen fence | existing root/closure corruption and reader blockers |
| S85-6 | authenticated original reattachment; multiple originals; offline same receipt; queued offline untouched | offline lock/ack/proof and sink witnesses |
| S85-7 | config/API/outcome/error parity, async drop, exact public exports | recovery denylist and governed API allowlist |
| S85-8 | empirical control/queued/service/acquisition/blackout/completion samples, all outcomes | final code gate and platform limitations recorded |

Design/Astra review precedes production changes. Behavior-specific RED commits
precede GREEN. Independent verification follows code review. Final full gate runs
once on the completed clean code tree; narrow repairs use affected suites.
