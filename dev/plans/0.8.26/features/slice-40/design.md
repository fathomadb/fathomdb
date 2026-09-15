---
title: FathomDB 0.8.26 Slice 40 — atomic derived-edge design
status: APPROVED
---

# Slice 40 design — schema-34 derived-edge activation

## Fixed contract and exists-versus-net-new boundary

Accepted
[`ADR-0.8.26-breaking-v1-contract-and-fresh-database-boundary.md`](../../../../adr/ADR-0.8.26-breaking-v1-contract-and-fresh-database-boundary.md)
requires one changed-in-place V1 actuation surface and fresh 0.8.26 databases
only. D26-04 and D26-05 fix complete prospective endpoint validation and the
compact receipt boundary.

Slice 35 already supplies the production candidate for the five-operation V1
grammar: `PutDerivedEdge(ProvenancedEdgeV1)`, prospective-state simulation,
canonical edge application, one writer transaction, digest/replay, bounded
affected revisions, exact request-relative source references, projection,
erasure, traversal, and Python/TypeScript bindings. Those mechanisms are
retained, not redesigned here.

The net-new Slice 40 product work is the on-disk identity and open boundary:
schema 33 must become 34, the prototype freshness classifier must become a
production precondition, current committed WAL state must remain readable, and
the current interface contract must stop promising automatic migration.

## Schema 34

Append migration step 34 as a content-free statement and set
`SCHEMA_VERSION = 34`. It introduces no table, column, index, trigger,
backfill, projection rebuild, receipt rewrite, or canonical-row change. Its
only product meaning is that a database was bootstrapped for the breaking
0.8.26 contract.

The full ordered migration table remains the implementation used to bootstrap
a missing/empty database. It is not an upgrade promise. Production refuses any
non-empty database whose effective version is not 34 before the migration
runner is called. The `open_with_migrations_for_test` seam retains direct
access to the runner only under the engine's dedicated non-forwarded
`migration-test-hooks` feature. Its `#[doc(hidden)]` public spelling is the
minimum Rust integration-test seam, not a supported product API. Default
engine, facade, Python, TypeScript, and CLI builds do not compile or forward
that helper. Internal migration-mechanism targets opt into the feature
explicitly.

## Shared public-open policy

Every public Rust open variant and both native bindings converge on
`open_with_embedder_and_subscriber`. That shared route applies this locked
admission sequence:

1. Canonicalize the requested database path without creating the database.
2. Acquire the product lock without truncating or writing its metadata. A new
   lock path is persistent: unlinking an advisory-lock inode would let a later
   opener create and lock a second inode while the first remains active.
3. Classify the path while holding the unmodified lock: missing/zero-length is
   `Bootstrap`; a non-empty SQLite database is `Current` only when its
   effective committed `user_version` is exactly 34. This result is
   authoritative. A non-current result drops the lock without rewriting its
   metadata; all database and SQLite sidecars remain byte-identical.
4. Only after the locked check admits the path, write the lock PID metadata
   and continue through the ordinary write-mode open, integrity/WAL probes,
   migration/bootstrap, recovery, projection reconciliation, and worker
   startup.

Holding the product lock across classification and writable open closes the
cooperating-writer TOCTOU window. A deterministic internal race test proves an
older FathomDB opener can either win the lock and leave schema 33 for a later
typed refusal, or lose with `DatabaseLocked`; it can never be migrated by the
0.8.26 caller. A raw SQLite writer that
ignores the FathomDB product lock remains outside the engine's exclusion
protocol, as it does on the existing write path.

The lock inode is a persistent namespace object, not database content. A
refusal preserves every database, WAL, SHM, and journal byte and never rewrites
an existing lock. If a caller presents a database with no lock sidecar, the
attempt may leave one empty lock file; removing it cannot be made race-safe
because another opener may already hold the same inode. This is the sole
allowed refusal-side namespace change.

`Engine::open`, `open_with_choice`, and `open_with_migration_event_sink` all
use this policy. PyO3, N-API, and the CLI already call those Rust entries and
retain their existing typed error translations. Hidden helpers that use the
ordinary shared route inherit the product policy. Only the explicitly named
custom-migration test helper bypasses it.

## Read-only classification and WAL

For a non-empty database, classification uses SQLite `mode=ro` plus
`query_only=ON`, not `immutable=1`: immutable mode ignores WAL and can read a
stale page-1 `user_version`. The connection is closed before classification
returns. Byte snapshots prove that both a clean schema-33 fixture and a
representative schema-33 fixture with WAL/SHM retain identical database, WAL,
SHM, journal, lock, and recovery-sidecar bytes after refusal.

The read-only connection may take transient SQLite locks in SHM and may rebuild
a missing SHM file, but it leaves no durable product change. If classification
refuses or errors and SHM did not exist on entry, the connection is closed and
that attempt-owned SHM file is removed before the product lock is released; a
before/after inventory proves restoration. On admission, the ordinary current-
schema open may retain the rebuilt SHM for recovery. Classification performs
no write-mode database open, persistent PRAGMA update, migration, recovery,
projection work, or lock metadata write. This bounded sidecar restoration is
necessary to observe the effective version in a valid current crash artifact;
silently ignoring its WAL would be incorrect.

The classifier returns the observed version, not a boolean, so the existing
typed incompatible-schema error remains truthful. It validates the WAL header
before asking SQLite to read the sidecar, then performs the ordinary header and
schema-tree probes and maps failures through the existing sanitized
`EngineOpenError::Corruption` stages. A classifier failure is final; detailed
corruption is not deferred to an unreachable later open. After admission, the
ordinary open remains authoritative for deeper current-schema validation. The
precheck must not weaken `DatabaseLocked` behavior for an active current
engine or convert an admitted current database into a migration candidate.

The acceptance suite covers five WAL-sensitive cases: schema-34 committed
state in WAL is admitted with and without pre-existing SHM; a second open of a
live current engine retains product-lock refusal; and schema-33 WAL fixtures
with and without SHM are refused with exact durable byte identity. The latter
asserts that the attempt-created SHM is removed. No raw WAL parser or new
compatibility file format is introduced.

## Retained actuation transaction

After a fresh/current schema-34 open, Slice 35's accepted algorithm remains:

1. Validate the current V1 grammar, identities, provenance, and ordered digest.
2. Simulate the bounded batch under a rollback-only savepoint and derive the
   final active endpoint state.
3. Refuse a derived edge when `from`, then `to`, is absent from that complete
   state; later operations count and edge position does not.
4. Apply nodes, dependencies, edges, lifecycle effects, projection work,
   receipt, replay, affected revisions, and exact source references in one
   writer transaction.
5. Commit once; fault or validation failure rolls back the complete unit.

No edge-specific receipt field, dangling count, consequence manifest,
historical reader, or operation-ID compatibility namespace is added.

## Failure and test model

- Non-current non-empty databases fail with the existing typed incompatible-
  schema error before product mutation or migration events.
- Missing/empty paths bootstrap all 34 ordered steps; current paths run none.
- Future and foreign zero-version SQLite files follow the same refusal path.
- Current corruption, WAL validation, runtime configuration, embedder identity,
  and lock errors retain their existing typed boundaries after admission.
- Locked admission prevents a cooperating older opener from turning a fresh
  path into a silently migrated schema-33 database.
- Focused integration proves schema/open behavior, clean refusal bytes,
  current-WAL admission, active-lock precedence, and fresh mixed-unit
  replay/traversal.
- Existing Slice 35 engine/property, PyO3, N-API, Python, and TypeScript tests
  remain the detailed actuation oracle. Slice 40 adds only cutover coverage and
  adjusts tests whose automatic-upgrade premise the accepted ADR supersedes.

## Deliberate exclusions

No V2 API, compatibility router, database converter, version-by-version
migration matrix, historical receipt/replay translation, performance rerun,
package matrix, publication, architecture inventory, or design-document
convergence belongs to this slice.
