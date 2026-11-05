---
title: DBMS erasure availability and bounded completion
date: 2026-11-03
status: research; FathomDB proposals require approval
---

# DBMS erasure availability and bounded completion

The owner permits deferred physical erasure, provided completion is guaranteed
under an explicit SLA. This authorizes investigating a stronger completion
contract; it does not select an asynchronous API, scheduler or cleanup mechanism.
The recommendation is to separate committed logical exclusion from durable,
resource-reserved physical completion, and qualify the smallest maintenance unit
that releases SQLite's writer between commits. A background thread alone provides
neither writer availability nor a deadline.

None of the reviewed DBMS documentation establishes an unconditional deadline for
physical deletion. The common mechanisms are MVCC/tombstones, background work,
snapshot retirement, throttling and backpressure. Their retention thresholds and
task completion signals must not be relabeled as forensic-erasure SLAs.

## Scope and existing FathomDB constraints

This memo distinguishes three outcomes: logical invisibility through product
interfaces; removal from the declared database files and registered artifacts;
and secure sanitization of the underlying media. Only the second is the proposed
physical-completion boundary. Filesystem snapshots, detached backups, replicas,
SSD remapping, process memory and swap are not implicitly covered. Each managed
copy added to scope needs its own obligation and completion evidence.

The [follow-up evidence](../experiments/current-history-search/followups/RESULTS.md)
reports residual node-FTS terms after logical deletion and WAL truncation, and
approximately 4.6 seconds for a candidate secure full refresh. These are distinct:
the first is a shipping erasure defect; the second rejects a candidate availability
strategy. This research ran no new performance or erasure experiment.

Current [dependency closure](../adr/ADR-0.8.25-dependency-lifecycle-closure.md)
requires exact originating-operation retry for physical completion and fences
other writes while completion is pending. The
[offline completion ADR](../adr/ADR-0.8.27-offline-physical-erasure-completion.md)
adds a narrow operator path; automatic completion on Engine open is outside its
decision. Consequently, a background worker alone would leave write availability
blocked. Accepted-pending results, automatic recovery and narrower write fences
require explicit successor contracts. Existing erasure success must continue to
mean completed erasure until such a successor is approved.

## Documented DBMS mechanisms

| System | Availability mechanism | Actual completion limits |
| --- | --- | --- |
| PostgreSQL | Ordinary VACUUM reclaims dead versions while SELECT/INSERT/UPDATE/DELETE continue. VACUUM FULL rewrites the table, needs extra disk and an exclusive lock. | Versions potentially visible to transactions cannot be reclaimed. Ordinary vacuum generally makes space reusable rather than returning it to the OS; neither operation is documented as media sanitization or a deletion deadline. [Routine vacuuming](https://www.postgresql.org/docs/current/routine-vacuuming.html) |
| MySQL InnoDB | Background purge processes undo history in configurable batches with configurable workers. A desired purge-lag threshold introduces DML delays to let cleanup catch up. | Long consistent reads, including backup transactions, retain undo. The default lag setting imposes no maximum; maximum lag delay caps DML delay, not deletion latency. History-list length exposes backlog. [Purge configuration](https://dev.mysql.com/doc/refman/8.4/en/innodb-purge-configuration.html) |
| RocksDB | Deletes/range tombstones are logged and later processed by flush/compaction. Snapshots preserve visible versions; live iterators can retain obsolete SST files after replacement. | Compaction completion and old-file retirement are separate. Some stale-file deletion awaits later activity. These mechanisms provide no bounded deletion deadline. [DeleteRange architecture](https://rocksdb.org/blog/2018/11/21/delete-range.html), [snapshots](https://github.com/meta/rocksdb/wiki/Snapshot), [stale files](https://github.com/meta/rocksdb/wiki/Delete-Stale-Files) |
| CockroachDB | Background MVCC garbage collection removes obsolete history; protected-timestamp records preserve versions needed by jobs such as backups. | Protected versions cannot be collected until protection is released. GC retention configuration is not proof that bytes disappear at that timestamp, much less from backups or every unavailable replica. [Storage and protected timestamps](https://docs.cockroachlabs.com/docs/stable/architecture/storage-layer) |
| Elasticsearch | Delete-by-query offers asynchronous task status, batched deletes, throttling, slicing and cancellation. Successfully completed batches survive subsequent failure. | Snapshot/version conflicts and exhausted retries can leave work incomplete. Task acceptance is not deletion completion; logical deletion is distinct from later segment reclamation. [Delete-by-query](https://www.elastic.co/docs/api/doc/elasticsearch/operation/operation-delete-by-query) |
| MongoDB TTL | A background monitor rotates among indexes, stopping each turn after 50,000 deletes, one second or exhaustion; metrics expose passes and deletes. | Documentation explicitly denies immediate expiry deletion; workload can extend delay beyond the nominal 60-second sweep. Deletes run as foreground delete operations. Secondary members replicate primary deletes. This is a useful best-effort counterexample, not an SLA model. [TTL indexes](https://www.mongodb.com/docs/manual/core/index-ttl/) |
| SQLite | WAL permits readers alongside a writer; checkpoints can make incremental progress. | There is still only one writer. Old read snapshots can stop checkpoint progress. Background work on another connection does not create a second writer or bound transaction occupancy. [WAL concurrency](https://www.sqlite.org/wal.html#concurrency) |

Several operational details are particularly transferable:

- PostgreSQL cost-based vacuum delays trade maintenance throughput for foreground
  I/O availability; critical-lock sections do not necessarily yield at the cost
  threshold. Its anti-wraparound failsafe can abandon delays. This illustrates
  priority escalation, not an erasure deadline. [Vacuum configuration](https://www.postgresql.org/docs/current/runtime-config-vacuum.html)
- PostgreSQL standby conflicts expose the reader tradeoff directly: cancel
  conflicting queries after configured delays, or use feedback that delays
  primary cleanup. [Hot standby conflicts](https://www.postgresql.org/docs/current/hot-standby.html#HOT-STANDBY-CONFLICT)
- PostgreSQL concurrent REINDEX builds a replacement, performs another pass for
  intervening tuples, switches validity/references, waits for old users and drops
  old indexes in separate transactions. It costs more work and can leave invalid
  artifacts after failure. This is a concrete replacement/retirement pattern,
  not a ready-made SQLite implementation. [REINDEX](https://www.postgresql.org/docs/current/sql-reindex.html#SQL-REINDEX-CONCURRENTLY)
- RocksDB slows or stops writes when compaction debt grows; a nonblocking write
  option returns incomplete instead of waiting. Backpressure protects finite
  capacity rather than promising unlimited foreground work. [Write stalls](https://github.com/meta/rocksdb/wiki/Write-Stalls)
- Elasticsearch force merge reclaims deleted-document space, but its task is
  noncancelable, can need substantial temporary disk, and is recommended for
  indexes no longer receiving writes. Open point-in-time searches retain old
  segments. Async response does not remove those costs. [Force merge](https://www.elastic.co/docs/api/doc/elasticsearch/operation/operation-indices-forcemerge), [PIT retention](https://www.elastic.co/docs/api/doc/elasticsearch/operation/operation-open-point-in-time)

## SQLite primitives and their limits

FTS5 normally leaves obsolete postings until merges. Its per-table secure-delete
option removes postings during updates/deletes; SQLite core secure deletion is
also needed for file-level traces. FTS5 documents incremental optimization:
start with `merge(-N)`, continue with `merge(N)` in separate transactions until
the documented no-work test. N approximates pages written, not elapsed time.
Repeated negative starts can restart work when new trees arrive. Rebuild deletes
and recreates the index; contentless tables do not support it. Secure deletion
also has format compatibility consequences. [FTS5 commands](https://www.sqlite.org/fts5.html#special_insert_commands)

Core `secure_delete=ON` overwrites deleted ordinary-table content; `FAST` may
leave freelist traces. Core secure deletion alone does not guarantee removal of
FTS shadow traces. Enabling either policy now is not evidence that previously
retained postings have been sanitized. [SQLite secure_delete](https://www.sqlite.org/pragma.html#pragma_secure_delete)

PASSIVE checkpoint does not wait and may remain incomplete. FULL waits for
the required writer/reader conditions and blocks new writers; RESTART also waits
until readers use the database file only; TRUNCATE additionally reduces WAL to
zero on success. Unrelated readers/writes matter, so source-local fences alone
cannot bound finalization. [Checkpoint API](https://www.sqlite.org/c3ref/wal_checkpoint_v2.html)

Interruption acts at the earliest opportunity, not a documented hard time bound;
interrupting a write inside an explicit transaction rolls back that transaction.
Progress callbacks count approximate VM instructions and cannot modify their
connection. Neither provides resumable checkpoints inside an arbitrary FTS
command. Sleeping between rows inside one transaction still retains the writer.
[Interrupt](https://www.sqlite.org/c3ref/interrupt.html),
[progress callback](https://www.sqlite.org/c3ref/progress_handler.html)

## FathomDB inference/proposal: mechanisms to compare

1. **Eager secure deletion, bounded transactions.** Baseline for future erasures:
   remove known targets under both secure-delete policies, commit bounded groups,
   then discharge remaining artifacts/WAL. Single large bodies or posting lists
   may still dominate a group; row count alone cannot establish its time bound.
2. **Tombstones plus deadline-driven incremental cleanup.** Compare durable
   logical denial followed by bounded merge work against the baseline. Certify
   every affected registered family and the complete file/artifact boundary.
   A merge returning no work is not itself an erasure proof. Pending work cannot
   depend on future user writes, and newer work must not perpetually restart it.
3. **Staged replacement plus retirement.** Build a replacement in committed
   chunks, capture intervening mutations, catch up, atomically publish validated
   metadata, then retire the old generation and scrub its copies. Budget extra
   disk, delta backlog, publication and retirement separately. Same-file staging
   still consumes the sole writer. External staging introduces additional copies
   and atomicity/recovery obligations; no cheap rename/cutover is assumed.

Do not preselect eager secure deletion for every ordinary projection update as
the only valid policy. Compare its steady-state cost with deferred cleanup under
the approved guarantee. Do not toggle a global flag opportunistically without a
single-writer ownership and crash-recovery proof.

Keep three workloads separate: legacy retained-term sanitation; future physical
erasure of an explicitly selected target generation; and membership expiry or
supersession that intentionally preserves authorized history. A validity-boundary
refresh is not authority to erase that history. Legacy cleanup needs its own
backlog, deadline and readiness disclosure; finishing one new erase must not
silently certify the entire old store as sanitized.

## FathomDB inference/proposal: durable completion protocol

Reuse the existing closure inventory, source/dependency authority, projection
drain/freeze, telemetry obligations and WAL proof. The following are conceptual
states, not proposed public API spellings or an independent erasure subsystem.

| Stage | Durable invariant and next action |
| --- | --- |
| Admit and deny | Atomically record authenticated scope/generation, acceptance time, deadline policy, required targets and resource reservation with logical denial. A pending receipt never claims physical completion. |
| Clean in chunks | Commit each bounded unit with its progress cursor/proof. Retry is idempotent; crash loses at most the uncommitted unit. Revalidation prevents stale projection workers or replacement generations from restoring erased content. |
| Retire readers/artifacts | Track old epochs and required sinks; stop granting leases that would violate the remaining budget. Explicitly resolve same-source-ID reingestion: reject until completion or admit a distinct generation without reviving old work or deleting new data. |
| Finalize | Drain/fence all participants required by actual checkpoint conditions, including unrelated work where necessary. Perform the measured final window, sink completion and WAL truncation; no busy/partial result becomes success. |
| Complete | Persist completion only after the target inventory and physical obligations are proved discharged. Restart after completion must not reintroduce erased generations. |
| Blocked or overdue | Persist cause, remaining work and original deadline. Retry/escalate automatically only under an approved successor; never reset the deadline, drop the obligation or reinterpret a miss as completion. |

Logical denial linearizes at the committed intent/closure fence for newly
admitted reads. Specify already admitted and pinned reads separately: current
snapshot semantics may let them finish; a stronger denial acknowledgement needs
a reviewed drain/cancellation policy. Previously returned data cannot be revoked.
Neither an atomic intent nor a source fence proves constant-time large dependency
closure or publication.

During deferred cleanup, retained rows/postings must not influence the authorized
selected corpus statistics, candidates/caps, graph seeds/traversal or any relaxed,
historical or evidence path. Publish membership/certification consistently with
denial. If that is not possible atomically, affected scopes must refuse, await
publication or use independently certified safe routing. Filtering unauthorized
results after ranking is insufficient even when no erased row is returned.

Narrowing today's global write fence requires proving safe scoped admission for
canonical, dependency, projection, registry and repair paths. Retain a short
global reader/writer admission window if finalization requires it, with explicit
maximum blocking and latency qualification. If that cannot be bounded, availability
claims must disclose the maintenance pause rather than hide it behind a task ID.

## SLA and admission choices requiring owner agreement

An enforceable bound requires finite admitted work, reserved cleanup throughput,
bounded reader lifetimes and a defined availability clock. Unlimited readers,
unrestricted competing ingest, indefinite engine closure and no interruption
cannot coexist with a finite physical-completion deadline.

- **Clock:** choose wall-clock from durable acceptance, with an always-running
  executor/host obligation, or cumulative engine-available service time. The
  latter is not a calendar erasure promise. Define what counts as available;
  scheduler starvation or deliberate admission of competing work cannot pause it.
  Record wall age as well. Define clock rollback/restart handling explicitly.
- **Capacity:** reserve cleanup service and temporary disk before accepting an
  SLA; bound pending bytes/jobs and worst-case target expansion. Admit only when
  queued work plus the new job fits the deadline with reader/finalization margin.
  Retain denial or an explicit unaccepted outcome when capacity is unavailable;
  do not issue an accepted guarantee and later discard it.
- **Readers and priority:** establish maximum snapshot/operation lifetime before
  granting work. As deadlines approach, increase cleanup share, throttle/refuse
  competing work and stop admitting pinning reads. Cancellation requires a
  supported successor or a proven existing work bound; run-to-completion alone
  does not bound a reader. Escalation begins before the deadline, not after it.
- **Close/reopen:** choose drain-on-close or durable remaining work with explicit
  paused/unavailable semantics. An embedded engine cannot erase files while no
  process runs. Reopen must reconcile owed work before normal admission under
  the selected successor; the present ADR does not authorize automatic completion.
- **Failure:** permanent storage/host failure makes unconditional physical
  completion impossible. State the supported fault envelope and repair duty.
  Misses remain observable breaches with persistent obligations. A monitor or
  retry loop is not a substitute for a reserved service/admission argument.

No numeric SLA is justified by the present experiment. Record per-job accepted
time, deadline, available-time accounting, phase, remaining bytes/targets, last
progress, oldest blocking reader, sink/WAL status, retries, reserved capacity and
misses using content-free identifiers. Completion evidence must cover live rows,
FTS shadow data, applicable freelist/main-file traces, WAL and registered telemetry;
absence from MATCH or product search alone is insufficient.

## Required POCs and decision list

Before selecting a mechanism, compare the three options on the same real table
modes and pinned SQLite version. Include legacy ghosts and fresh deletes,
large/high-frequency terms, simultaneous expiry, concurrent ingest and zero
subsequent traffic. Measure end-to-end foreground read/write p50/p99, maximum
writer occupancy, acceptance latency, physical completion, CPU/I/O, disk/WAL
growth and failures; count incomplete requests, not only successful samples.

Use positive unique-term witnesses in each populated registered FTS family;
inspect raw shadows/files as applicable, verify survivors and source isolation,
then reopen/rebuild and prove no resurrection. Crash or interrupt at every durable
transition, including after cleanup before progress commit, publication before
retirement, sink completion and WAL truncation. Test retry idempotence, same-source
new generations, projection races, pinned unrelated readers, continual new
readers, disk exhaustion, unavailable sinks, close/reopen and deadline exhaustion.
Bounded merge must prove committed progress and completion despite concurrent
updates; replacement must prove coherent cutover and cleanup of abandoned copies.

Owner decisions needed: SLA clock and supported envelope; API distinction between
accepted and completed; maximum reader lifetime and permitted cancellation;
scoped versus temporary global fencing; backlog/admission/escalation policy;
close/reopen responsibility; legacy-sanitation deadline; included artifact scope;
and same-source reingestion semantics. Then approve successor ADR/interfaces and
the measured mechanism. CH1 remains a proposal; this memo implements nothing.

## Review and provenance

Prepared by the research subagent from the linked primary documentation on
2026-11-03. Astra-medium independently checked the proposed implications and
existing contract boundaries. Its required denial-linearization and pending-corpus
isolation clarifications are incorporated above. This is research/proposed design,
not a completed experiment or production implementation.
