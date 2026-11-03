---
title: FathomDB 0.8.28 Slice 81 — draft implementation design
status: DRAFT
target_release: 0.8.28
---

# Slice 81 draft implementation design

This is the POC implementation design, not the production erasure successor.
[Requirements](requirements.md), [experiment](experiment-design.md) and
[test oracles](test-plan.md) govern it. Sources S81-3/5/6/9 are indexed in
[README](README.md#references). Exact seam feasibility is a code-phase gate.

## Layers and authority

1. **Unchanged Engine control:** reproduce physical ghost and pending-closure
   global write rejection using public methods. Never silently alter the existing
   ignored failure or mark a native closure complete to unlock unrelated writes.
2. **Mechanism layer:** actual Engine files/table modes and writer connection;
   measure A/B commands, durable POC progress and raw physical proofs. This layer
   can establish SQLite feasibility, not actual product availability.
3. **Composition layer:** exercise candidate denial, publication, real read/write
   admission and physical finalization. Coverage must identify native mechanisms
   retained, test-only seam behavior, exclusive-adapter restrictions and unsupported
   public paths. A standalone reader pool/SQL wrapper cannot qualify native Engine
   availability or product AC-076/081.

Use existing engine-owned `execute_for_test`, query hooks and reader rendezvous
where sufficient. Each work unit executes its entire BEGIN/work/progress/COMMIT
under one writer acquisition; never leave a transaction open between separate
helper calls where another Engine write could join it. Measure lock wait separately
from occupancy. Never mutate the live database through an unowned second writer.

A required scoped-fence/reader-policy seam must be specified and reviewed before
coding: engine-owned, opt-in, per-instance, `test-hooks` gated, default unchanged,
covering actual admission/publication paths. Preserve native validation and prove
source/dependency/projection/registry/repair safety before narrowing a global gate.
If a safe seam requires broad production call-chain redesign, stop that layer,
record Engine availability UNQUALIFIED and propose the exact successor work.
Do not implement the full production contract merely to make a POC claim pass.

## Candidate mechanisms

**A — eager reference.** On fresh fixtures enable per-table FTS secure-delete
before population plus existing core secure deletion. Erase using actual owner
inventory and existing completion obligations. For legacy ghosts separately
qualify full rebuild/replacement from surviving authoritative rows. A prospective
flag alone is not sanitation. Preserve each table's true membership and tokenizer.
Report single-transaction stalls and current global-fence limitations honestly.

**B — deferred committed cleanup.** Commit intent and the defined denial fence,
then delete live target membership in bounded transactions and perform necessary
FTS merge work in separate committed units. Follow SQLite's documented negative
initial merge/positive continuation protocol; persist campaign ownership/progress
rather than restarting a negative merge every turn. Revalidate concurrent mutations
and job generations. A no-work result is not physical proof. All live/posting
removal, core freed-page treatment, source/receipt/telemetry cleanup and WAL closure
remain required. Determine applicability per real FTS mode; do not assume every
target is contentless or every command supports every table.

**C — conditional staged replacement.** Only after the experiment's B-failure
trigger and reviewed delta design: register the new copy before population, build
in committed units, capture/catch up concurrent mutations, validate, atomically
publish the replacement, then retire old readers/generation and erase old/abandoned
copies. Include all generation copies in the erasure inventory. Same-file staging
still takes the sole writer; external staging adds copy/atomicity obligations and
is excluded unless a narrowly reviewed reason requires it. No cheap rename/swap
or immediate retirement is assumed. A publication success is not erase completion.

Ordinary expiry removes current membership while retaining authorized history.
It has its own certificate/publication oracle; it is not a physical-erasure job.

## Durable state and scheduling

Prototype metadata belongs to explicitly prefixed experiment tables, created only
in disposable fixtures. Store an opaque operation identity, immutable scope digest,
source generation/cut, original accepted time/deadline policy, phase, family/copy
inventory, last committed work cursor, remaining estimate and blocker/proof state.
Do not retain erased body text or low-entropy subject identities in completion
metadata. Reuse native durable obligations where compatible; no second public
erasure service or public schema is introduced.

Conceptual states: prepared (not accepted), accepted/denied, cleaning, retiring,
finalizing, complete; blocked/overdue preserve the owed operation and original
deadline. Commit progress with the corresponding mutations. Completion is persisted
only after all required obligations discharge; replay after a crash between external
side effects and bookkeeping must revalidate idempotently, not assert success.
An overdue flag is orthogonal to phase so a job can continue and finish late.

One candidate scheduler uses the real writer, finite pending-work admission and
reserved service; simulate policy time for deterministic tests and measure actual
service for benchmarks. Max work-unit pages/rows are tunables frozen before timing,
not proven time bounds. Initially reject same-source reingestion until completion.
Bound/refuse closure expansion when the reservation cannot support it. Do not accept
an unconditional SLA from estimated row count or a full scan hidden in admission.

Declare clock as an experiment parameter: wall acceptance/deadline survives restart;
engine-available service is recorded separately. Test clock rollback, closed-engine
intervals and overdue reopen. No worker can run while the embedding process is absent.
No shipping auto-resume or success meaning changes without successor approval.

## Denial, readers and physical finalization

Record acceptance and denial linearization independently even when they share a
transaction. New admitted candidate reads after the fence must use correct scope
and membership. Old pinned reads may finish under current semantics; a stronger
policy needs explicit tested drain/cancellation. Already returned results cannot
be revoked. Refuse/await uncertified scopes instead of post-ranking exclusion.

Apply this to body/property/dense/graph and relaxed/frozen/evidence surfaces in the
coverage matrix. Supporting a node-native collector does not qualify all of them.
Per-job denial must not contaminate surviving corpus statistics or candidate caps.
Large closure/publication may take time; measure that rather than assert O(1).

Finalization uses required drain/fencing, telemetry and checkpoint behavior.
Source-local fences alone do not satisfy TRUNCATE's unrelated reader/writer
conditions. Test a bounded global admission window where necessary, reader leases
and policy escalation; measure its entire stall. A busy checkpoint or unavailable
original sink leaves a durable blocker. Run-to-completion is not an active-operation
deadline: unsupported interruption remains an explicit constraint, not a hidden fix.
Physical completion covers all registered candidate copies and traces; it does
not assert SSD, backups, filesystem snapshots, swap or returned-result erasure.

## Planned artifacts and TDD sequence

Create an unpublished standalone crate and small controller under the planned
`dev/experiments/erasure-availability/` directory; use path dependencies on the real
engine/query/schema and the pinned SQLite. Commit its tools/lockfile/manifests.
Runtime configuration must precede every SQLite open in fresh and cached child
processes. Reuse existing fixture/provider/probe patterns, preserving old results.

Build behavior groups RED then GREEN: baseline inventory/physical witnesses;
durable state and rollback; A physical reference; B bounded/progress cleanup;
denial/publication and admission; crash/reopen and finalization; measurement
controller. C is a separate conditional group. Stage/commit each failing test
before fixing it; never invent a failure where a baseline guard already passes.
Astra code review and independent verification precede measurement. A final
coverage/limitations table is mandatory before recommending any production seam.
