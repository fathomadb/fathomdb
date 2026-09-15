---
title: FathomDB 0.8.26 Slice 30 — operator integrity distribution design
status: APPROVED
---

# Slice 30 design — operator integrity distribution

## Decision and boundary

Retain `fathomdb doctor data-plane-integrity` and its V1 request/report as the
single operator route. Add no prebuilt artifact, SDK method, report V2, check,
repair behavior, or raw-SQL escape hatch. This applies D26-02 (`seq-280`): the
exact-version crates.io CLI is the selected distribution, and failure of that
route is a visible HITL stop rather than permission to expand packaging.

The existing semantic checker is unchanged. Slice 30 corrects only its process
boundary: the CLI must not use ordinary `Engine::open`, because that path owns
migration, recovery/reconciliation, runtime construction, and writable lock
state.

## Inspection protocol

The operator-gated Rust facade exports one free function,
`inspect_data_plane_integrity(path, DataPlaneIntegrityRequestV1) ->
Result<DataPlaneIntegrityResultV1, EngineError>`. It is not an `Engine` method:
the type name must not imply that a serving runtime was opened. It performs
these steps in order:

1. Validate the request before touching the path.
2. Canonicalize the existing database parent and require both the database and
   its existing `<db>.lock` sidecar. Open the lock read-only and take
   the same non-blocking exclusive process lock used by `Engine::open`, without
   truncating, rewriting, or creating the sidecar. Failure is a typed refusal.
3. While holding the lock, reject an existing non-empty `<db>-wal` or non-empty
   `<db>-journal`: either is outside this strict quiescence envelope and is
   refused without alteration. Permit `<db>-shm`: the accepted reader-pool
   lock ADR says it may persist after clean shutdown. Never checkpoint, delete,
   or rewrite any sidecar.
4. Preserve runtime initialization order: call the existing
   `configure_runtime_for_open()` before extension registration or any SQLite
   call, then register the process-local SQLite extension, open the database
   through a percent-encoded `file:` URI with `immutable=1`,
   `SQLITE_OPEN_READ_ONLY`, and `SQLITE_OPEN_URI`, then enable
   `PRAGMA query_only=ON`. SQLite's immutable mode is required because plain
   read-only WAL access can still create or alter `-shm`. Do not run the
   migration/open-recovery pipeline and do not construct an `Engine`, reader
   pool, embedder, or projection runtime.
5. Read `PRAGMA user_version` and require exact equality with the binary's
   compiled `SCHEMA_VERSION` (33 for this slice). Both older and newer versions
   refuse; neither is migrated.
6. Run the unchanged integrity executor in its existing deferred read
   transaction. It supplies one coherent SQLite snapshot, canonical check
   order, bounded work/findings, privacy-safe locators, and no partial report.
7. Close the read-only connection and release the product lock. The database,
   lock bytes, and complete sibling sidecar set must match the pre-invocation
   witness.

FathomDB writers cooperate through the product lock, so an active Engine
refuses before SQLite inspection. Raw external SQLite writers are outside the
supported contract; operators must stop them before invoking the command.

## Failure and output model

Success remains
`{schemaVersion:"fathomdb.doctor.data-plane-integrity.v1",status,report}`.
No fields or check meanings change.

Every post-parse semantic or runtime failure from this verb uses
`{schemaVersion,status:"error",verb:"data-plane-integrity",code:
"FDB_DATA_PLANE_INTEGRITY",reason,fieldPath}`. Request reasons remain unchanged.
Clap syntax/type failures occur before verb dispatch and retain the CLI-wide
exit-2/stderr behavior. Post-parse precedence is request validation, database
path, existing lock/open/acquire, recovery sidecars, runtime configuration,
SQLite read-only open/query-only, schema equality, then integrity execution.
The inspection boundary adds:

| Reason | Field path | Exit | Meaning |
| --- | --- | ---: | --- |
| `inspection_unavailable` | `/dbPath` | 70 | The database parent/file or existing lock cannot be opened safely for a non-corruption reason. |
| `inspection_lock_missing` | `/dbPath` | 70 | No existing product lock can prove the supported quiescence contract. |
| `inspection_not_quiescent` | `/dbPath` | 71 | The product lock is held, a non-empty WAL exists, or a non-empty rollback journal exists. |
| `runtime_configuration` | `/runtimeConfiguration` | 70 | Process-lifetime SQLite runtime policy is incompatible or unavailable. |
| `database_schema_mismatch` | `/databaseSchemaVersion` | 70 | On-disk `user_version` differs from the compiled schema. |
| `integrity_corrupt` | empty | 70 | The read-only open/probe/check cannot interpret the store safely. |

The error envelope contains no SQLite diagnostic string, holder identity, row
body, source text, or partial findings. Findings continue to expose only the
existing typed identifiers and cursor values.

## Distribution and identity

The final published install spelling is an exact pin:

```bash
cargo install fathomdb-cli --version '=0.8.26' --locked
fathomdb --version
```

The registry command becomes executable only after 0.8.26 is published. Slice
30 installs the current source candidate into a clean root with `cargo install
--path ... --locked`, checks discovery and `--version`, and records the
historically proven 0.8.25 registry route separately. Slice 50 repeats the
integrated final-candidate proof; the post-publication release smoke alone can
prove crates.io 0.8.26. The published 0.8.25 binary is schema-33-compatible but
does not satisfy this slice's immutable-inspection process boundary because it
still enters through ordinary `Engine::open`; an unqualified install is not a
Slice 30 witness.

## Public-surface impact

- `dev/interfaces/cli.md`, `docs/reference/cli.md`, `docs/install/rust.md`, and
  `src/rust/crates/fathomdb-cli/README.md` gain the exact install, quiescence,
  schema-pairing, error, and no-mutation contract or corrected verb inventory.
- `docs/operations/` gains the operator procedure and index link.
- The Rust facade exports the named free function and added integrity error
  reasons only behind the existing `operator` feature. `dev/interfaces/rust.md`
  records the signature. Operator-off Rust exposes neither function nor method.
- Python and TypeScript gain no method, type, parser, native binding, or error
  route. Their existing absence assertions remain the acceptance oracle.
- Existing doctor verbs keep their open behavior and JSON envelopes.

## Verification design

Process-level tests compare the database, `.lock`, `-wal`, `-shm`, and optional
`-journal` path/existence/bytes map before and after the CLI. Separate cases
cover a missing path, clean success, lower and higher schema mismatch, held
product lock, non-empty WAL/rollback journal, persistent clean `-shm`, invalid
bounds/checks, findings, and corrupt state. Existing engine suites remain
authoritative for all four check
algorithms, work/finding ceilings, canonical ordering, and privacy locators.
Focused CLI/engine tests, public-doc and governed-surface checks, a clean
Linux-x86_64 source-candidate install/discovery witness, and canonical
`agent-verify` close the slice. The five-target native and staged-candidate
matrix remains Slice 50 work; only post-publication smoke can prove crates.io
0.8.26.

## Adversarial remediation design

### Resolved database identity

The inspection-only path setup will retain `canonical_database_path` for its
existing parent-directory normalization, then canonicalize the now-required
database file itself. Every later operation—the product-lock lookup,
`-wal`/`-journal` checks, immutable URI construction, and SQLite open—uses that
single resolved path. Ordinary `Engine::open` is unchanged because it must
still support creating a database that does not exist.

The regression uses a real database opened once through a symlink so the alias
lock exists naturally. It then proves that inspection through the alias sees a
lock held on the target path. A second case proves that a WAL beside the target
cannot be hidden by the alias namespace. These are Unix tests because creating
unprivileged symlinks is a stable Unix facility; the path-resolution code is
platform-neutral and the existing Slice 50 matrix retains final multi-platform
ownership.

Blast radius is limited to the operator-gated free inspection function and the
Slice 30 CLI process test. No serving open, migration, schema, report, binding,
or integrity-query behavior changes. Hard-link aliases do not resolve through
filesystem canonicalization and remain outside this narrowly reported symlink
defect; this change introduces no new hard-link support claim.

### Truthful closeout state

After GREEN produces a stable implementation tip, update the manually
maintained Slice 30 ladder row and the Slice 30 entry in
`release-state-0.8.26.json` to that tip. Regenerate only declared views, retain
Slice 35 as `next_slice`, and update the Slice 30 status/review evidence with
the remediation and verification results.

Blast radius is documentation and release-state only. No decision, dependency,
remaining-ladder order, schema version, or `origin/main` claim changes.

### Exact crates.io selection

Change the smoke command to
`cargo install fathomdb-cli --version "=$VERSION" ...`. Keep the existing
SemVer input validation and post-install binary identity check. The structural
test will require the exact literal spelling, providing a network-free oracle
for the registry command assembled by the post-publication job.

Blast radius is one crates.io smoke command and its structural test. PyPI, npm,
Windows smoke scripts, release workflow routing, publication, and registry
state are unchanged.
