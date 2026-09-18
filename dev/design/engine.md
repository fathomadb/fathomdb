---
title: Engine Subsystem Design
date: 2026-09-17
target_release: 0.8.26
desc: Current open, connection, write, projection, cursor, identity, and close design
blast_radius: fathomdb-engine runtime/writer/reader; interfaces/*.md; requirements and acceptance engine intersections
status: ACTIVE
---

# Engine design

This file owns the current engine runtime and storage coordination contract.
Public language spelling remains in `dev/interfaces/`; error variants remain in
[`errors.md`](errors.md); embedder identity and mean centering remain in
[`embedder.md`](embedder.md); and projection generations remain in their
feature designs.

## Open path

Public 0.8.26 open is fresh-only. A missing or zero-length path is bootstrapped
directly to schema 34. A nonempty database is accepted only when its
`PRAGMA user_version` is exactly 34; every other version returns
`IncompatibleSchemaVersion` before product mutation. Public open does not run
an in-place migration ladder.

Admission canonicalizes the database namespace, acquires the persistent
sidecar lock, configures SQLite for WAL operation without
`locking_mode=EXCLUSIVE`, classifies/bootstrap-validates the file, runs the
always-on corruption and stored embedder-profile checks, prepares the runtime
embedder, and starts readers and projection workers. The sidecar lock prevents
two first openers from racing bootstrap. Detailed corruption stages and codes
are owned by [`errors.md`](errors.md); schema construction remains owned by the
schema crate.

### Untrusted-file posture

Open treats the path as untrusted structured input. It fails closed on a
noncurrent schema, malformed header/schema state, WAL-replay failure, or stored
embedder-identity inconsistency. It neither treats a legacy database as fresh
nor rewrites it into the current format. Operator inspection and recovery use
the boundaries in [`recovery.md`](recovery.md).

## `Engine.open` success result

Open returns a live engine plus an `OpenReport`. The report includes schema
before/after facts, migration-step reporting (the schema construction steps on
a fresh bootstrap and an empty list when reopening a current database),
embedder warmup timing, and current startup events.
Interface documents own binding access and casing; migration and embedder
designs own the meaning of their report fields.

## Writer / reader split

The live engine has three connection roles:

- `Engine.connection` is the mutex-serialized primary caller writer used by
  accepted writes, admin configuration, and synchronous operator maintenance;
- projection workers each use their own connection, do expensive embedding
  outside the critical section, and serialize their write commits through the
  shared `commit_gate`; and
- `ReaderWorkerPool` owns multiple read connections and executes read work in
  DEFERRED transactions rather than behind the primary writer mutex.

SQLite still permits one write transaction at a time. The explicit primary
writer mutex and projection `commit_gate` make the engine's ordering and
maintenance invariants visible without pretending that all work uses one
dedicated writer thread. Admin configuration uses the primary writer path; it
is not arbitrary caller-supplied SQL and does not bypass admission or commit
coordination.

## Canonical identity and supersession

Canonical node and edge identity has two forms:

- a nullable caller-supplied `logical_id` identifies a governed entity across
  revisions; and
- the engine-assigned `write_cursor` identifies the physical stored revision
  position and is also the current row-identity carrier.

Callers do not supply a separate canonical `row_id`. Anonymous inserts may
leave `logical_id` null. For governed rows, the active uniqueness constraint is
on `logical_id` alone, separately for nodes and edges; `kind` is payload
classification, not part of identity. A new revision may therefore change
kind while superseding the prior active revision with the same logical ID.

Supersession appends the new physical revision and marks the previous active
revision inactive in the same transaction. Edges reference logical endpoint
IDs; a missing or superseded active endpoint is counted, not silently repaired.
There is no public restore operation. `purge` and source erasure are deletion
operations and cannot reactivate history.

Search results expose a typed public `IdSpace`, which is distinct from the
internal positional `write_cursor`. The two must not be conflated in binding or
pagination code.

## Batch submission semantics

`Engine.write(&[PreparedWrite])` validates a batch before commit-sensitive
work. An accepted batch executes in caller order inside one SQLite transaction;
mixed canonical and operational-store writes either all commit or none do.

Each written row receives its own monotonically increasing cursor. The
`WriteReceipt` returns:

- `row_cursors`, one-to-one with input order;
- `cursor`, the batch high-water cursor (the final row cursor); and
- `dangling_edge_endpoints`, the informational count of edge endpoints that
  did not resolve to an active node at commit time.

Projection jobs are published only after the canonical transaction commits.
No public partial-success batch contract exists.

## Cursor contract

`write_cursor` is a monotonically increasing committed-row position. It backs
physical revision identity, stable ordering, pagination/high-water boundaries,
and projection source positions. A batch of N rows receives N consecutive
positions; the batch receipt's `cursor` is the last one.

Projection progress has separate generation/readiness authority. A canonical
write cursor therefore does not claim that asynchronous vector work is already
visible. Frozen reads authenticate both the canonical boundary and the
relevant dependency/projection generations.

## Projection maintenance

Lexical projections are maintained transactionally with canonical writes.
Dense projection is asynchronous: committed source state becomes vector-ready
only when the owning projection generation records a successful current
outcome. Failed or stale work is explicit state, not silent eligibility.

Dense rows share the schema-owned `vector_default` virtual table. Authority and
recovery metadata live in `_fathomdb_vector_rows`, the projection registry, and
generation tables. The current design does not create one vector table per
kind; `source_type` and `kind` distinguish governed rows inside the shared
projection.

Projection workers may compute concurrently, but their commits are ordered by
`commit_gate`. Mean pinning and manual mean recomputation use that same gate so
no centered/uncentered vector commit can interleave with the unique
requantization transaction.

## Operational store

Operational collections, append-only mutations, latest state, canonical rows,
and projections live in the same SQLite product database. Typed
`PreparedWrite` variants, rather than raw SQL, cross the engine write boundary.
Declared payload validation occurs before commit; operational rows may commit
atomically with canonical rows in the same batch.

The operational API is a logical surface, not a second database, and it does
not weaken the single product namespace, durability, or close rules.

## EngineConfig ownership

Engine configuration controls runtime behavior; interface documents own the
exact Rust/Python/TypeScript spellings and conversion precedence. Adding a
public knob requires all supported bindings in the same slice. Configuration
does not select a legacy schema, enable automatic recovery, supply vector
identity strings, or expose raw SQL.

## Close path

`Engine.close` is explicit, idempotent, and bounded. Its order is:

1. mark the engine closed so new operations observe the typed closing state;
2. stop and join the projection runtime;
3. shut down and join reader workers after they uninstall their profile
   callbacks and release their connections;
4. uninstall the primary profile callback and release the primary connection;
5. release test-only connection registration and clear profile callback
   contexts; and
6. release the sidecar admission lock last.

Step 6 is load-bearing: readers drain before the primary writer connection so
SQLite's last-handle checkpointer runs on that connection, and the admission
lock remains held until every owned SQLite resource is gone.

Drop/finalizer paths are best-effort safety nets and must not panic. Bindings
preserve bounded process exit even when application code omits an explicit
close; explicit close remains the only path that can report shutdown failure.

Historical dedicated-writer-thread, caller `row_id`, per-kind vector-table,
single-cursor-per-batch, restore, and public migration-on-open descriptions are
retained in Git history but are not current 0.8.26 authority.
