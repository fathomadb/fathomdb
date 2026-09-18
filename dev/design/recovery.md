---
title: Recovery Subsystem Design
date: 2026-09-17
target_release: 0.8.26
desc: Current operator diagnostics, maintenance, export, and loss-authorized recovery boundary
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

The binary has exactly two roots:

- `fathomdb doctor <command>` for diagnostics, inspection, export/cache work,
  and one explicitly authorized non-lossy derived-vector maintenance command;
  and
- `fathomdb recover --accept-data-loss <action> <db_path>` for CLI
  data-loss-authorized operator recovery.

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

## Recovery inventory

The recovery parser exposes five actions:

| Action | Engine operation | Scope |
| --- | --- | --- |
| `--truncate-wal` | WAL truncate/checkpoint recovery | Physical recovery state. |
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

## Connections, locks, and mutation posture

Most database-bearing commands use operator-enabled Engine methods and inherit
the engine's admission lock, primary writer, reader, projection, and close
rules. Readout does not imply that `Engine::open` itself is immutable: public
0.8.26 admission accepts only a current schema-34 database or a fresh path.

`data-plane-integrity` is the deliberate exception. It does not call
`Engine::open`, does not start workers, and does not migrate, repair, rebuild,
or create product files. It resolves symlink aliases to the target product
namespace and treats non-quiescent recovery state as a typed refusal.

Recovery mutations run through the operator feature and the engine's serialized
write/projection coordination. Erasure may durably remove governed rows and
then return a typed WAL-checkpoint-incomplete outcome; the deletion result and
at-rest checkpoint guarantee are distinct and remain fail-closed.

## JSON shapes for other doctor verbs

`--json` selects the normative machine-readable representation. Every current
doctor command emits exactly one JSON object when `--json` is selected. Every
current recovery action also emits exactly one JSON object in its current
machine-readable path; success exits with the accepted-loss class `64`.
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
| `E_CORRUPT_HEADER` | Attempt `doctor safe-export`, then rebuild/re-import externally. |
| `E_CORRUPT_SCHEMA` | Diagnose; use projection rebuild only when the finding and current schema permit it. |
| `E_CORRUPT_EMBEDDER_IDENTITY` | Treat stored profile drift as corruption; do not auto-accept identity change on open. |
| `E_CORRUPT_INTEGRITY_CHECK` | Doctor-only full-integrity finding. |

### Wal replay failures

`E_CORRUPT_WAL_REPLAY` refuses normal open. Inspection precedes the explicitly
acknowledged `--truncate-wal` recovery action.

### Header malformed

`E_CORRUPT_HEADER` refuses normal open. `safe-export` is the non-destructive
first operator attempt; reconstruction and import remain external.

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

Historical 0.6.0 verb subsets, deferred purge/restore discussion, and progress-
stream output are retained in Git history but are not current authority.
