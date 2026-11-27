---
title: Recovery Subsystem Design
date: 2026-11-26
target_release: 0.8.32
desc: Current operator diagnostics, maintenance, export, backup/restore/rebuild, migration, and loss-authorized recovery boundary
blast_radius: fathomdb-cli; operator feature; requirements REQ-035..REQ-040 and REQ-054; interfaces/cli.md
status: ACTIVE
---

# Recovery design

This file owns the current operator action inventory and effect boundaries.
Concrete command spelling, JSON fields, and exit classes remain in
[`interfaces/cli.md`](../interfaces/cli.md). Typed engine and binding errors
remain in [`errors.md`](errors.md). Projection-generation repair detail remains
in [`recovery-0.8.25.md`](recovery-0.8.25.md).

## Operator boundary

The binary exposes three roots:

- `fathomdb doctor <command>` for diagnostics, inspection, export/cache work,
  and one explicitly authorized non-lossy derived-vector maintenance command;
- `fathomdb recover --accept-data-loss <action> <db_path>` for CLI
  data-loss-authorized operator recovery; and
- `fathomdb migrate --backup-dir <dir> [--json] <db_path>` for offline
  Schema `38 -> 39` (and chained `37 -> 38 -> 39`) upgrades with pre-upgrade
  SHA-256 backup verification.

This split is not equivalent to “all doctor commands are read-only.” The narrow
successor
[`ADR-0.8.26-cli-derived-maintenance-boundary.md`](../adr/ADR-0.8.26-cli-derived-maintenance-boundary.md)
grandfathers only `doctor recompute-mean` as mutable derived maintenance. It
does not authorize another mutable doctor command or weaken the required
recovery acknowledgement.

Neither root is an SDK command family. Governed SDK `purge` and `erase_source`
are application lifecycle/erasure operations, so the CLI is not the sole
erasure route. Reserved-namespace source excision and op-store record excision
remain operator-only behind `recover`.

## Doctor inventory and effects

The current doctor inventory has 15 commands.

| Effect class | Commands | Contract |
| --- | --- | --- |
| Database-free diagnostics | `gpu`, `platform`, `reranker-gpu` | Inspect platform/device policy without opening a product database or loading a model. |
| Engine-backed diagnosis/readout | `check-integrity`, `verify-embedder`, `trace`, `dump-schema`, `dump-row-counts`, `dump-profile`, `dump-mutations`, `orphan-provenance` | Open the current database through the operator-enabled engine and emit bounded diagnostic/readout data. |
| Immutable quiescent inspection | `data-plane-integrity` | Resolve the product namespace, require exact schema and quiescence, then inspect with read-only/query-only `immutable=1`; never create an Engine or repair. |
| Artifact/cache work | `safe-export`, `warm-cache` | Write an explicit export/manifest or populate the verified model cache; neither is a product-database recovery mutation. |
| Non-lossy derived maintenance | `recompute-mean` | Atomically recompute the stored mean from retained uncentered vectors and recreate/requantize derived vector rows. |

### Doctor-only flags

`doctor check-integrity` accepts `quick`, `full`, and `round-trip` request
flags. The first and third preserve the default bounded check selection;
`full` additionally activates SQLite's full integrity check. These flags do not
enable open-time integrity work or recovery mutation.

`data-plane-integrity` is the only command described as an immutable out-of-
process database inspection. It refuses live-lock, WAL/rollback-recovery, schema
mismatch, invalid bound, and integrity states with its typed envelope and emits
no partial report.

`recompute-mean` runs normal engine admission and shared pre-writer maintenance
first. Its command-owned transaction reads `vector_default.embedding`, updates
`_fathomdb_embedder_profiles.mean_vec`, and recreates/requantizes
`vector_default` while preserving governed row metadata. Projection-worker
commits are excluded by the shared `commit_gate`; injected failure rolls the
transaction back. Shared dependency-closure maintenance remains owned by its
own design and is not new doctor mutation authority.

Current explicit-policy ticket/fence admission and offline receipt reconciliation
are owned by the [maintenance contract](explicit-maintenance-erasure/contract.md)
and its [production design](explicit-maintenance-erasure/production-design.md).
Queued intent is not activated by offline completion. Activated debt still needs
its exact original sinks and WAL closure; unknown or unavailable sinks remain
owed. Rebuild without a usable provider can invalidate existing dense state and
is not certified numerical compatibility repair.

## Recovery inventory

The recovery parser exposes six actions:

| Action | Engine operation | Scope |
| --- | --- | --- |
| `--truncate-wal` | WAL truncate/checkpoint recovery | Physical recovery state. |
| `--complete-erasures` | Validate and complete owed physical dependency closures offline | Already-admitted `purged` and `source_erased` closures. |
| `--rebuild-vec0` | Rebuild vector storage | Derived vector projection. |
| `--rebuild-projections` | Rebuild projection materializations | Derived serving state. |
| `--excise-source <id>` | Excise every reachable row for one source | Canonical/provenance erasure, including reserved operator namespaces. |
| `--excise-collection <name> --excise-record-key <key>` | Excise all versions and current state for one op-store record | Operational record erasure; both flags are required together. |

`--accept-data-loss` is root-level and mandatory. Doctor rejects it. A recovery
request with no selected action is an unrecoverable refusal rather than an
implicit default. The command selects one action by the parser/dispatcher's
defined precedence; it does not promise a multi-action transaction.

Projection rebuild uses active eligible canonical state and the current
projection-generation authority. It is a recovery workflow, not an automatic
open-time heal and not a separate `regenerate` root.

## Backup, restore, rebuild, and internal `fathomdb.logical-export/v1` (`R32-04`)

- **Installed-artifact CLI backup / restore / rebuild flow (`slice50_backup_restore_rebuild.rs`):**
  1. Quiesce and close the application `Engine`, then snapshot via
     `fathomdb doctor safe-export <export.sqlite> --manifest <manifest.json> <source.sqlite>`
     (SQLite `VACUUM INTO` plus SHA-256 manifest verification).
  2. Copy the verified snapshot to the restored target path.
  3. Rebuild derived state via **separate** `recover` invocations (each command
     selects one action):
     `fathomdb recover --accept-data-loss --rebuild-projections --json <restored.sqlite>`
     and
     `fathomdb recover --accept-data-loss --rebuild-vec0 --json <restored.sqlite>`
     (each exiting `64`).
  4. Reopen a dense store with its matching embedder and call `drain()` to
     re-materialize vector rows, discharging any remaining FTS sanitation
     certificate via `fathomdb recover --accept-data-loss --complete-erasures --json <restored.sqlite>`.
  5. Verify post-restore health with both
     `fathomdb doctor check-integrity --full --json <restored.sqlite>` and
     `fathomdb doctor data-plane-integrity --json <restored.sqlite>` (`0`
     findings).
- **Internal `fathomdb.logical-export/v1` seam (`logical_export.rs`):**
  Strictly feature-gated behind
  `#[cfg(any(test, feature = "test-hooks", feature = "operator"))]` and absent
  from default public SDK APIs (`fathomdb-sdk`, Python, TypeScript). Covers all
  `36` `fathomdb_schema::TABLE_MANIFEST` tables, preserves exact `write_cursor`
  values and orphaned `non_canonical` `_fathomdb_projection_terminal` cursors,
  refuses export atomically inside the snapshot transaction when any pending
  erasure exists, never emits derived vector/probe payloads (`embedding_bin`,
  `mean_vec`, `_fathomdb_embed_probe`), and derives `mean_vec` on restore iff
  the vector count is `>= 256` (`NULL` below `256`).

## Connections, locks, and mutation posture

Most database-bearing commands use operator-enabled Engine methods and inherit
the engine's admission lock, primary writer, reader, projection, and close
rules. Readout does not imply that `Engine::open` itself is immutable: public
0.8.32 admission accepts only a current schema-39 database or a fresh path.

`data-plane-integrity` is the deliberate exception. It does not call
`Engine::open`, does not start workers, and does not migrate, repair, rebuild,
or create product files. It resolves symlink aliases to the target product
namespace and treats non-quiescent recovery state as a typed refusal.

Recovery mutations run through the operator feature and the engine's serialized
write/projection coordination. Erasure may durably remove governed rows and
then return a typed WAL-checkpoint-incomplete outcome; the deletion result and
at-rest checkpoint guarantee are distinct and remain fail-closed.

`recover --truncate-wal` is the second deliberate path-scoped exception to
generic Engine admission. After the CLI validates `--accept-data-loss`, the
operator-only `recover_truncate_wal` function resolves the canonical namespace,
requires an existing nonempty regular database, acquires the canonical product
lock, rechecks those facts, and refuses a nonempty rollback journal. It probes
the main file through immutable/read-only/query-only SQLite before classifying
the fixed 32-byte WAL header under the lock. A malformed WAL may be discarded
only when the standalone main file is schema 39 and satisfies current Fathom
schema invariants; a valid WAL is checked through SQLite's effective
main-plus-WAL view because it may carry pending schema state. The effective-view
probe snapshots transient SHM state and restores it when validation refuses.
One recovery-only `mode=rw` SQLite connection, opened without create permission,
then owns `PRAGMA wal_checkpoint(TRUNCATE)` and the destructive WAL recovery.
The path never bootstraps, migrates, starts workers, or edits the WAL directly,
and normal `Engine::open` remains fail-closed.

`recover --accept-data-loss --complete-erasures <db_path>` shares that
path-scoped admission sequence and lock. It opens one recovery-only read-write
connection after preflight, without Engine reader pool or projection runtime.
It refuses malformed WAL, validates each owed physical closure against both
its recorded proof and current zero state, and checks the durable telemetry
redaction queue before any checkpoint. The queue does not persist an
authenticated original sink path, so a telemetry obligation or
`telemetry_redaction` blocker remains owed and the action exits `70`; the
operator must attach the original sink and retry the originating engine
erasure. A clean `wal_checkpoint(TRUNCATE)` precedes marking the validated
closures complete. SQLite BUSY leaves them owed and exits `71`; successful
completion exits `64`. If no physical closure is owed, the action leaves an
unrelated WAL untouched and exits `0` with status `clean`. The required
`--accept-data-loss` acknowledges
irreversible removal of recoverable erased content from the WAL.

## JSON shapes for other doctor verbs

`--json` selects the normative machine-readable representation. Every current
doctor command emits exactly one JSON object when `--json` is selected. Every
current recovery action also emits exactly one JSON object in its current
machine-readable path; completed recovery exits with the accepted-loss class
`64`. A busy WAL checkpoint is not completion and exits `71`.
Recovery output is not an NDJSON progress stream in the current implementation.

Stable exit classes are:

- `0` — clean doctor completion;
- `64` — recovery completed under explicit data-loss acknowledgement;
- `65` — actionable doctor finding;
- `66` — export/materialization failure;
- `70` — unrecoverable command failure; and
- `71` — lock-held or equivalent precondition refusal.

The CLI serializer owns top-level discriminators and field casing. Engine
report structs are not renamed to satisfy CLI JSON. `check-integrity` retains
its physical/logical/semantic report; `data-plane-integrity` retains its
versioned report/error envelope. Exact fields remain interface-owned.

## Integrity and open-path relationship

`Engine::open` owns its always-on corruption checks and typed open failures.
Doctor findings are a separate report surface and need not have a one-to-one
`CorruptionKind` or `OpenStage` variant. In particular,
`E_CORRUPT_INTEGRITY_CHECK` is a doctor-full finding, not an open-stage enum.

## Code-to-operator-action cross-reference

Open-path recovery hints continue to map stable corruption codes to operator
actions:

| Code | Operator direction |
| --- | --- |
| `E_CORRUPT_WAL_REPLAY` | Inspect and, when explicitly authorized, use `recover --truncate-wal`. |
| `E_CORRUPT_HEADER` | Preserve the original and use external forensic/SQLite recovery tooling; logical `safe-export` is fail-closed. |
| `E_CORRUPT_SCHEMA` | Diagnose; use projection rebuild only when the finding and current schema permit it. |
| `E_CORRUPT_EMBEDDER_IDENTITY` | Treat stored profile drift as corruption; do not auto-accept identity change on open. |
| `E_CORRUPT_INTEGRITY_CHECK` | Doctor-only full-integrity finding. |
| `E_ERASURE_INCOMPLETE` | Stop the application, then run the finding's `recover --accept-data-loss --complete-erasures <db_path>` argv; if telemetry is owed, reattach the original sink and retry the originating erasure instead. |
| `E_DANGLING_EDGE_ENDPOINT` | Active edge endpoint does not resolve to an active same-scope node; inspect and ingest missing nodes in the same scope before relying on graph completeness. |
| `E_CROSS_SCOPE_EDGE_ENDPOINT` | Active edge endpoint resolves only in a distinct scope; quarantine and inspect the scope isolation anomaly before further reads or writes. |

### Erasure incomplete

`doctor check-integrity` adds one logical finding per owed physical closure.
The finding reports an opaque closure id, cause, phase, blocker, sequence,
observed WAL frame count, and a parser-ready remediation argv array; it
redacts the source identity. The argv is provided for checkpoint obligations;
for `telemetry_redaction`, the argv is null and the detail calls for the
original sink and exact originating erasure retry. A durable pending-redaction
queue takes precedence over a not-yet-recorded blocker in the crash window
after the erasure transaction. The frame count is observed from WAL sidecar
bytes, without issuing any checkpoint; it is `null` if the header or sidecar
cannot be read reliably. Doctor does not discharge closures. An owed closure
exits `65`.

### Wal replay failures

`E_CORRUPT_WAL_REPLAY` refuses normal open. The explicitly acknowledged
`--truncate-wal` path independently validates the main database, holds the
product lock, rejects a counterfeit current-schema database, preserves database,
WAL, and transient SHM bytes on preflight refusal, and lets SQLite perform the
checkpoint/discard. Its report adds
`discarded_corrupt_wal`, true only when the locked header classification was
malformed and SQLite returned `Done`; `Busy` remains retryable exit `71` and
never claims discard.

### Header malformed

`E_CORRUPT_HEADER` refuses normal open and `doctor safe-export` remains
fail-closed because SQLite cannot produce a logical `VACUUM INTO` artifact from
an unreadable header. Preserve the source bytes and use external forensic or
SQLite recovery tooling; a raw byte copy is not a successful safe export.

### Schema inconsistent

`E_CORRUPT_SCHEMA` refuses normal open. Projection rebuild is appropriate only
when diagnosis establishes that canonical state and the current schema remain
admissible.

### Embedder identity drift

`E_CORRUPT_EMBEDDER_IDENTITY` is not automatically accepted or rewritten.
Stored and runtime identity must agree under the embedder owner contract.

### Integrity-check full findings

`E_CORRUPT_INTEGRITY_CHECK` is emitted by the doctor full-integrity surface and
is not an `Engine::open` stage.

### E dangling edge endpoint

`E_DANGLING_EDGE_ENDPOINT` is emitted by `doctor check-integrity` (semantic
section) when an active edge (`superseded_at IS NULL`) references a `from_id`
or `to_id` that resolves neither to a current active node
(`superseded_at IS NULL AND state = 'active'`) nor to a genuine historical
superseded-only active node (`superseded_at IS NOT NULL AND state = 'active'`
with no `superseded_at IS NULL` row, matching `fathomdb migrate` preflight) in
the edge's own scope (and does not resolve to an active node in another scope).
In particular, missing targets and endpoints whose current row is `pending`,
`deleted`, or `purged` (even when an older superseded `'active'` revision
exists) are flagged. By contrast, `doctor data-plane-integrity --check
edge_endpoint_integrity` strictly requires a current non-superseded active node
(`superseded_at IS NULL AND state = 'active'`). This state can arise
legitimately after an opt-in `BulkNonStrict` write whose target nodes have not
yet been ingested. Doctor performs read-only verification only and exits `65`
while dangling endpoints remain; ingesting the missing nodes in the same scope
resolves the finding without rewriting the edge.

### E cross-scope edge endpoint

`E_CROSS_SCOPE_EDGE_ENDPOINT` is emitted by `doctor check-integrity` (semantic
section) when an active edge (`superseded_at IS NULL`) references a `from_id`
or `to_id` that is absent from the edge's own scope and resolves to an active
canonical node in a distinct scope. Normal engine write paths refuse
cross-scope endpoints atomically, so this finding indicates out-of-band storage
corruption or bypass. Doctor performs read-only verification only and exits
`65`.

Historical 0.6.0 verb subsets, deferred purge/restore discussion, and progress-
stream output are retained in Git history but are not current authority.
