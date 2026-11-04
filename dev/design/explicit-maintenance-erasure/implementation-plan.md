---
title: Explicit maintenance erasure — proposed design and qualification work
status: ACTIVE
date: 2026-11-04
target_release: 0.8.28 Slice 85
---

# Proposed implementation and qualification sequence

HITL approved the [contract/API](contract.md) on 2026-11-04 and assigned
[Slice 85](../../plans/0.8.28/features/slice-85/plan.md) as a separate clean
implementation. This is not a continuation of failed optimization work.
Current source is the restored Slice 83 behavior plus retained observers/tools
and verified fixes, as summarized in the [write-up](../../notes/erasure-optimization-sequence-findings.md).

## Changes requiring successor design

| Existing authority | Proposed change to design and review before code |
| --- | --- |
| [Resumable cleanup ADR](../../adr/ADR-0.8.28-resumable-physical-cleanup.md) mandates idle/reopen service and scoped concurrent writes | Persisted explicit opt-in policy, no autonomous work for that policy, exclusive maintenance and conservative post-delete mutation fence. Autonomous mode remains current behavior. |
| Existing singleton root journal | Separate bounded pre-deletion queue. Immutable selector/cut and full closure-mutation freeze; explicit activation reuses exact RootDebt capture/proof. No queued intent is structural-zero authority. |
| Existing physical success and offline authority | Online handle and operator recovery share registered-copy/sink/proof/checkpoint primitives, without competing workers or a second completion definition. |
| Engine reader/projection/writer/close ownership | Single maintenance coordinator with a documented acquisition/lock order, newcomer refusal and accounted drain. Reader API gates alone are insufficient; admin/queue/telemetry/publication owners must participate. |
| Existing source/lifecycle contracts and SDKs | Explicit-mode MaintenanceRequired, typed queue/status/window/outcomes and identical Rust/Python/TypeScript semantics. Public Rust/SDK/wire/CLI docs change together with tests. |

The design must settle queue-key/receipt metadata budget, scope-freeze effect
inventory, policy/config wire shape, original-debt detection, close/cancellation
ownership and all API error precedence. These are implementation prerequisites,
not permission to silently broaden the approved product contract. If safe queued
scope requires expensive admission discovery, return the conflict to HITL rather
than introduce an unbounded pause into the normal path.

## Proposed behavior-specific RED/GREEN phases

1. **Policy and admission.** Persist/latch policy under product ownership. Require
   the reviewed fresh-store format break: old binaries refuse new files
   and the feature release refuses prior formats before mutation. RED policy
   mismatch/missing/unknown state, active legacy debt conversion, queued capacity and no cleanup on explicit-mode open.
   GREEN changes only the reviewed policy behavior; keep autonomous regressions.
2. **Queue identity and scope.** RED atomic intent/fence, invalid/deleted-first
   requests, conflicting idempotency keys, lost response, capacity and same-scope
   mutation/rebuild/provenance races. GREEN bounded queue with indexed guards.
   Generate real SQLite writer histories and compare immutable selected scope
   with independently constructed expected roots/dependents. Reject oversized or
   unprovable cheap admission, exercise heavier window admission, unrelated resource
   growth and explicit queued withdrawal racing activation. No low-latency or
   logical-revocation claim from a queued receipt.
3. **Maintenance ownership.** RED newcomer read/write/admin/publication/queue
   admission, old snapshots, same-thread ownership, concurrent begin/close/stale
   handle and expired acquisition. GREEN a single coordinator and lock order,
   not another writer connection or polling loop retaining the primary mutex.
4. **Physical activation and outcomes.** RED queue→RootDebt atomicity, committed
   deletion before failure, per-family rollback/continuation, original sinks,
   redacted global fence and successful TRUNCATE. Include authenticated original-sink
   reattachment that cannot register/substitute or enable capture, and atomic
   offline activated-ticket/root/receipt reconciliation; queued intents do not
   execute through the old offline completion command. GREEN reuse of existing proof
   primitives. No replacement FTS mechanism or proof/accounting optimization.
5. **Crash/reopen and binding parity.** RED real process exits at queue admission,
   activation, sanitation, pre/post-redaction, checkpoint and terminal receipt.
   GREEN truthful retry/state and manual-only recovery after explicit-mode open.
   Python/TypeScript async interruption or caller drop cannot clear work/fences.
6. **Independent physical qualification.** Existing four-family positive-marker,
   mapped-vector, retained-history/survivor ranking, late telemetry, wrong/missing/
   multiple sink, external-reader and offline tests remain oracles. Exercise all
   new queued copies in main/WAL/sink proof; no generated golden snapshots.

Each new behavior receives an observed staged/committed RED before GREEN, and
Astra review of concrete design/ADR/interface successors before implementation.
Astra code review and an independent verification subagent precede timing/landing.
Missing proof authority or a deadlock blocks acceptance; no benchmark can override it.

## Measurement proposal for approval with implementation scope

Reuse committed Slice 81–84b tools and original corpora; write a small adapter
for the new product APIs rather than a new benchmark framework. Preserve all old
outcomes. Freeze source/binaries, observer overhead, SQLite/runtime/provider,
inputs/managed triggers, resource ceilings, offered rates and failure dispositions
before timing. No concurrent builds/tests in timed windows.

Measure separately:

- Normal reads/writes/open with explicit policy and no queued job, then bounded
  queued backlog; queue admission and exact-key retries. Match no-deletion controls
  and report every success/refusal/miss/unfinished outcome, not just latency.
- Acquisition latency/refusal with no owned work, held owned reads/writes and
  projection jobs; observe actual newcomer refusal and gate blackout.
- Exclusive maintenance with one target request on node/edge/property originals;
  actual native/proof/sanitation/sink/checkpoint and whole-window time, including
  drain and release. Compare final retained Autonomous code at both idle and
  sustainable load for context, not an expectation of a 1–3s result.
- Failure cases: external pinned reader, unavailable/wrong sink, process exit and
  retry. Record prolonged durable write-fence states separately from successful
  maintenance windows; do not compute completion latency for incomplete attempts.

Use a small screen before repeated measurements; choose the normal-path latency/
throughput tolerance and window acquisition budget with HITL, based on intended
application load. There is no invented numerical deadline in this draft. A future
qualification protocol must have explicit per-cell/total time, scratch/RSS and
sample-count gates before execution. No new campaign is authorized by writing this
plan. Fewer than 1,000 valid samples or refusal/miss-bearing streams cannot support
a qualified p99. No pooling incompatible protocols to reach sample thresholds.

## Checks, records and scope control

Apply the [quality roadmap](../../notes/fathomdb-quality-testing-roadmap.md):
real-database differential/state-machine, selected process/commit/sink faults,
boundary and ownership/resource-cleanup tests. Defer broad VFS/OOM/MC/DC, physical
storage-remanence promises and new frameworks. No temporal/current-corpus rewrite
or fast logical-revocation subsystem is included in the minimal proposal.

Use scoped lint/typecheck/tests while iterating and one required final full code
phase gate; narrow repairs rerun failed/affected suites. Platform WAL/SDK suites
cover the changed ownership and outcome blast radius. CI interactions use `gh`.
Documentation-only proposal validation uses `agent-lint-md.sh` and state-view
checks when planning references change. No production tests are required merely
for this draft. Status must report exact gate/resumption provenance.

Implementation is placed in Slice 85; user-facing docs/examples and SDK parity
are included. Next action is exact design and Astra review in that separate run.
Preparation performs no implementation, publication or cross-repo message.
