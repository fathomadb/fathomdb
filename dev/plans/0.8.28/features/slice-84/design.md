---
title: Slice 84 — cleanup contention design and negative qualification
status: APPROVED
target_release: 0.8.28
---

# Slice 84 cleanup contention design and disposition

The [plan](plan.md) owns requirements S84-1–7, acceptance, TDD and measurement.
The reviewed scheduling candidate was implemented and tested, then rejected
after the fixed paired campaign failed qualification. [Slice 83 design](../slice-83/design.md) and the
[accepted ADR](../../../../adr/ADR-0.8.28-resumable-physical-cleanup.md) govern
current safety. Preserve the end-of-Slice-83 guarantee/API decision.

## Observed path and unproved attribution

`write_inner` takes the maintenance writer turn and primary connection, calls
`guard_plain_write`, then the canonical commit path calls certification inside
its transaction. `certify_plain_write` performs `envelope`, advances the safe
boundary, stores root debt and runs `validate_zero`. Envelope scans canonical
body sizes/counts, vector mappings and provenance/receipt metadata. Zero proof
checks committed boundaries/generation, all registered erased-cursor copies and
captured mapped vector IDs. Unit/finalization paths also validate authority.

Those operations occur under scarce writer ownership. Root/denial decoding,
per-row provenance checks, SQL preparation, full scans, root serialization and
priority/service-turn waits are separate possible costs. Slice 83 measured long
worker wait, not individual certification component time. Reusing prepared vector
queries did not establish latency improvement. First instrument; do not attribute
all waiting to proof scans without a measured decomposition.

## Preferred design order

1. Remove only measured redundant computation within the same transaction and
   locked connection, preserving identical SQL checks, error precedence and phase
   authority. Examples to evaluate: decode root once per certified batch, reuse
   prepared metadata/probe statements and carry validated transaction-local facts.
   This is the lowest-risk first candidate, but must pass integrated timing.
2. If full scans dominate, propose narrowly scoped transactional accounting or
   proof reuse for the already-certified plain mutation path. For each fact name
   its owner, initial exact check, delta, invalidating events, rollback semantics,
   commit publication and reopen reconstruction. No general proof cache or durable
   counter system is presumed necessary. A changed persisted format needs its own
   reviewed admission/compatibility disposition before code.
3. Only if measured scheduling delay remains dominant, adjust the existing
   worker/foreground handoff, batching certification within existing limits or
   avoiding idle priority acquisitions. Preserve opportunities for both services,
   interruptible backoff, wake-after-commit and worker shutdown. Do not hold the
   primary mutex while sleeping or convert an atomic sanitation transaction into
   a series of unlocked SQL calls. Do not split native canonical deletion.

Astra reviews the selected concrete amendment before implementation. Keep at most
one certification and one scheduling candidate; report a rejected approach rather
than add replacement indexes/generations or a third cleanup mechanism.

## Required proof ledger in the concrete design

A small table in the final design must map each reused fact to authority and
fallback, not merely call it cached:

| Fact | What the concrete design must establish |
| --- | --- |
| Resource envelope | Exact row/body/vector/metadata/page semantics. Count actual effects of inserts and old-revision retention, same-batch supersession, unnamed edges and projection/FTS growth. Check independent page allocation after work; do not assume submitted bytes equal file growth. Unknown/unhandled mutation recomputes fully or refuses. |
| Denied scope/write eligibility | Same source and old/new logical/generated identities/endpoints remain checked, including earlier batch members. Protected provenance and persistent/TEMP trigger fallback retain refusal. No wider mutation class. |
| Structural zero | Explain why every admitted effect cannot recreate any captured cursor/vector/dependent row. Generation/watermark are bound to this exact journal and scope. Stale, corrupt or unproven state runs full exact proof or refuses; finalization keeps authoritative proof. |
| Progress/safe watermark | Update in the same transaction as actual mutations; failed/deferred commit rolls back both. Never publish speculative authority to the worker or a later transaction. |
| Reopen/legacy/redacted phase | Process-local optimization state is disposable and reconstructed only from authoritative DB state. Legacy journals keep conservative validation; redacted finalization retains the persisted global fence and successful checkpoint boundary. |
| Service turn | Lock order and ownership among public caller, foreground writer, maintenance and reader finalization; no lost wake, starvation, spin, leaked guard or join deadlock. |

No optimization may silently skip a safety obligation; an equivalent mechanism
must be explicit and independently tested. Compare optimized versus original full
validation after every generated state transition in test fixtures. Independent
physical markers/survivors remain necessary because both validators can share a
bug. Fail closed on uncertainty rather than let timing success authorize a write.

## Tests and instrumentation

Reuse existing test hooks, SQL-authorizer work counters and physical metrics.
Add no public timing API or production telemetry payload. Keep fault models real:
rollback, deferred commit failure, process exit/reopen, corrupt/missing authority,
new/old endpoint denial, same-batch writes, unbound/missing sinks, vector residue,
reader pin and close. A clock-based race test cannot substitute for a rendezvous.

Separate reduced work from improved service. Profile baseline/candidate with the
same metrics, preserve raw accounting and run the plan's core benefit gates.
Unchanged full-proof fixtures are regression oracles, not tests to rewrite for an
optimization. Work-count RED is required for the selected redundant-work change;
existing safety guards need no invented RED failure.

## Contract and final deliverable

No new successful outcome or weaker refusal/completion meaning. Update relevant
interface/ADR text only if implementation changes a documented mechanism; seek
approval for any contract/envelope/scope change. End-of-83 HITL decisions remain
inputs to reconcile, not choices this slice can invent. Final status must separate
safety pass, work-count reduction, service qualification and residual Phase 82
cost, with measured failures retained.

## Rejected candidate — exact committed-boundary query

The rejected candidate would replace only the structural-zero validator's
`MAX` over the union of all cursor rows with `MAX` over each table's `MAX`.
SQLite can use the existing cursor indexes on canonical nodes, canonical edges
and operational state. Operational mutations still scan; no index is added.
The outer `COALESCE(...,0)` stays outside both maxima levels, preserving empty,
NULL and negative-input behavior. Conversion to `u64` retains its original
failure semantics. Query preparation and errors remain at the same validation
point, before generation and registered-copy checks.

| Fact | Authority, lifetime and fallback |
| --- | --- |
| Committed maximum cursor | Exact SQL over the same four tables and the same locked connection/transaction, recomputed every invocation; algebraically equivalent associative maximum, no stale state or fallback needed |
| Resource envelope and denial | Existing full scans and root/denial validation remain unchanged, including triggers, prior provenance and old/new endpoints |
| Structural zero and generation | Existing exact registered-copy/vector and generation checks remain after the boundary query; missing/corrupt/legacy debt follows existing refusal/validator behavior |
| Safe watermark and rollback | Existing transactionally stored root/watermark is unchanged; failed/deferred commit rollback and reopen reconstruction retain existing authority |
| Finalization and service | Existing full validator, global tombstone fence and actual TRUNCATE remain; no scheduling, reader or lock-order change |

Deterministic TDD measures executed SQLite VM steps on a populated real fixture,
with the same query used by the validator. Differential properties compare the
original union query against the candidate on empty tables, mixed extrema,
NULLs, negative cursors, deletion, transactional mutation and rollback. Existing
mapped-vector, generation mismatch, crash/reopen, denial and completion suites
remain unchanged. No cross-transaction proof reuse is introduced; no new
invalidation state, durable version, public API or ADR successor is needed.

The observer is test-hook-only: a scoped thread-local metrics handle binds
nested root/envelope/certification/zero/journal stages to the current Engine
writer; guards restore prior scope. Outside hooks it is a no-op. Both binaries
carry identical observers. Elapsed counters are overlapping wall time, not CPU
measurements; certification CPU is explicitly unavailable in this instrumentation.
The diagnostic profile must establish whether this candidate is worth selecting;
work reduction alone cannot satisfy integrated S84-6.

## Rejected scheduling candidate — synchronous sanitation priority

Baseline diagnostic cells at final Slice 83 plus identical observers measured
public calls 8.612/3.189/12.321 seconds (node/edge/property). Boundary maxima cost
only 0.077/0.054/0.043 seconds; reject the exact-MAX candidate above as immaterial.
Exact cursor proofs and eligibility remain required. No certification algorithm
is changed and no proof reuse is introduced.

The synchronous caller holds the cleanup service gate throughout its operation,
so the worker cannot assert priority on its behalf. Its sanitation units formerly
waited on the primary mutex without the existing maintenance turn, allowing
foreground reacquisition to delay cleanup. The selected scheduling change acquires
the existing maintenance priority and turn before each synchronous sanitation
unit's primary-lock wait. One shared helper serves worker and public caller.
The service gate guarantees one priority owner. Poisoned turn locks recover their
guard as before; RAII clears priority and notifies waiting writers on every exit.

The existing sanitation wait timer starts before turn acquisition. Release primary
ownership and maintenance priority/turn before the existing after-unit rendezvous
and five-millisecond sleep. Initial validation, finalization, reader admission,
atomic deletion and all authority queries remain unchanged. Thus a foreground
operation already holding the turn completes, newcomers yield for one committed
unit, then foreground service can proceed between units. No mutex is held over
backoff, and close retains its original join protocol.

Astra approved this concrete mechanism before code. Deterministic RED/GREEN uses
the actual synchronous path: a new foreground writer must observe the maintenance
priority wait at the sanitation rendezvous, then acquire after the unit; the
existing after-unit Engine write witness verifies actual useful progress. Fault
injection at the acquisition boundary verifies priority is released on errors.
Existing worker lost-wake, contention, idle/reopen and close tests remain guards.
S84-3 authority-cache invalidation and certification work-count reduction are
not applicable: no check or reusable fact changes. S84-1's measured decomposition
justifies the scheduling change; S84-6 still requires integrated paired benefit.

## Final disposition

All 54 fixed cells completed, but every family failed S84-6. Node median duration
changed 7.783→7.705 seconds, edge 3.291→3.404, property 15.412→13.957; controls
and per-pair read/write constraints also failed as recorded in the status.
Priority transferred wait from the primary mutex to the writer turn without
removing the full certification cost of an already-admitted write.

Astra recommends rejecting this production candidate. The accepted ADR already
requires worker priority and between-unit progress, both delivered by Slice 83;
the new synchronous priority test specifies a new policy rather than exposing an
existing contract violation. Restore the original production scheduling, ADR
wording and original test suite. Preserve the candidate in Git, its frozen binary,
reviews and all measurement outcomes. Retain useful test-hook profiling and the
bounded driver/controller. The service regression remains unresolved; neither
correctness nor work relocation can substitute for the failed performance gate.

## Web research before disposition

Owner requested external research before closure (2026-11-03). Primary-source
research below informs next design selection; none establishes a measured
FathomDB speedup or authorizes a replacement campaign. The completed candidate
must retain its original verdict. These are application inferences, not claims
that the sources prove FathomDB erasure safety.

| Discovered approach | Evidence and fit to observed costs | Disposition |
| --- | --- | --- |
| Optimize exact probes using measured query plans and indexes | SQLite documents indexable equality/IN predicates, covering indexes and OR-arm optimizations. `guard_plain_write_inner` checks old revisions and several provenance relations; these are more promising than the measured-small boundary MAX. | First follow-up: attribute VM steps and full scans per eligibility/proof query, inspect plans on the pinned SQLite build and original families, then select only a demonstrably redundant scan. Existing all-revision, old/new endpoint and same-batch semantics remain exact. Do not blindly add indexes or use head-only predicates. |
| Maintain aggregate checks from transaction deltas | DBToaster demonstrates incremental maintenance of SQL views; related constraint-monitoring research applies it to continuously checked compliance queries. `envelope` recomputes body counts/bytes/maxima and metadata totals on admitted writes. | Promising larger design: exact initial aggregate plus actual committed insert/update/delete deltas in the same transaction. Exactly reproduce SQLite `length(CAST(value AS BLOB))`, NULL/COALESCE and saturation/threshold semantics for every counted metadata column. Count retained historical rows, projection effects and metadata; check actual page allocation independently. Maxima need a multiset/index or exact fallback when deleting the maximum. Missing/corrupt authority, unsupported mutation, rollback, generation change and reopen must reconstruct or refuse. No general cache or new library is justified. |
| Incrementally preserve structural-zero invariants | Constraint monitoring via incremental views suggests tracking the effects capable of violating a previously established invariant, rather than rescanning all unrelated data after every write. | Research direction only: prove the certified mutation class cannot recreate any captured cursor/vector/dependent row. Keep authoritative full proof at admission/reopen/finalization and exact fallback for uncertainty. A process-local boolean cannot authorize durable physical completion. |
| FTS identity lookup through an explicit row mapping | FTS5 UNINDEXED columns lack a full-text index. An official SQLite forum case documents severe scan behavior when treating an ordinary FTS column as a relational key. Here `write_cursor` is UNINDEXED in multiple schema definitions. | Inspect each registered family separately. Never replace cursor equality by MATCH (token semantics differ) or assume cursor=rowid: migrated/body/property families differ and property has multiplicity. A new authoritative cursor-to-row mapping would need its own integrity/recovery design; an auxiliary mapping alone cannot prove an orphan is absent. |
| Bound FTS merge work and schedule checkpoints separately | SQLite describes incremental merge, potentially long crisis merges and the extra work of secure deletion. WAL readers can inhibit checkpoint completion. | Already substantially represented by Slice83 committed sanitation units and finite checkpoint attempts. Keep as diagnostic checks if per-statement profile identifies them; current measurements do not justify generic merge/PRAGMA tuning. Disabling secure-delete, relaxing TRUNCATE or cancelling readers would change safety/contract, not solve this slice. |
| More writers or a busy-timeout change | Ordinary WAL has one writer. SQLite's separate BEGIN CONCURRENT enhancement still serializes commit and aborts conflicting page-level read/write sets. | Reject for this slice. FathomDB serializes on its own shared connection, and broad proof scans read many pages. More connections do not remove this critical section; alternate SQLite branches and retries are a much larger architecture change. |

Recommended next sequence: query-level work attribution first, exact eligibility
query optimization second, then a separately reviewed transactional-accounting
or invariant-maintenance design only if scans still dominate. Preserve real-store
differential tests against the full validator after each generated transition,
rollback/crash/reopen and corruption cases, and independent physical residue
witnesses. Performance qualification must retain matched controls, accounting and
physical-success gates. No automatic retune or threshold relaxation follows this
research.

Measurement distinction: `SQLITE_STMTSTATUS_VM_STEP` is a work proxy, not CPU
nanoseconds; `FULLSCAN_STEP` locates scans. EXPLAIN QUERY PLAN is for investigation
and its textual output is not a stable cross-version testing API. Preupdate hooks
are not a universal delta oracle: SQLite explicitly excludes virtual tables and
system tables, so FTS/vector effects require independently proved coverage.

Sources reviewed:

- [SQLite query optimizer: indexable predicates, OR and covering indexes](https://www.sqlite.org/optoverview.html).
- [SQLite EXPLAIN QUERY PLAN and unstable display format](https://www.sqlite.org/eqp.html).
- [SQLite prepared-statement work counters](https://www.sqlite.org/c3ref/c_stmtstatus_counter.html).
- [DBToaster: Higher-order Delta Processing for Dynamic, Frequently Fresh Views](https://arxiv.org/abs/1207.0137) (Ahmad et al., 2012).
- [What Can Database Query Processing Do for Instance-Spanning Constraints?](https://arxiv.org/abs/2206.00140) (Aamer et al., 2022; research analogy, not physical-erasure certification).
- [FTS5 configuration, merge and secure-delete semantics](https://www.sqlite.org/fts5.html).
- [SQLite forum: FTS column equality versus MATCH](https://sqlite.org/forum/info/4574555168ff4c9d) (2020 case report; not a transferable speedup estimate).
- [SQLite preupdate-hook coverage restrictions](https://www.sqlite.org/c3ref/preupdate_blobwrite.html).
- [SQLite WAL concurrency and checkpoint behavior](https://www.sqlite.org/wal.html).
- [SQLite BEGIN CONCURRENT branch design](https://www.sqlite.org/src/doc/begin-concurrent/doc/begin_concurrent.md).

Astra reviewed this research and its applicability before disposition. Exact-query
optimization is conditional on query-level attribution; there may be no scan-free
equivalent for an UNINDEXED FTS owner cursor. No follow-up implementation or
replacement measurement is authorized by these research directions.
