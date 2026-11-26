# Immutable integrity inspection

Use `fathomdb doctor data-plane-integrity` to check dependency and serving
projection authority without opening a serving engine or granting repair
authority.

## Before you run it

1. Install the exact CLI version paired with the store and confirm
   `fathomdb --version`. For published 0.8.31, use
   `cargo install fathomdb-cli --version '=0.8.31' --locked` or install the
   lockstep `fathomdb-cli` PyPI binary wheel via `pip install 'fathomdb[cli]'`
   (`pip install fathomdb-cli==0.8.31`).
2. Stop every FathomDB process using the database. The product `.lock` must
   exist and be acquirable. Raw external SQLite writers do not participate in
   this lock protocol and must also be stopped.
3. Do not delete or checkpoint sidecars to make the check pass. A non-empty
   `-wal` or `-journal` is a non-quiescent refusal requiring investigation. A
   persistent `-shm` is normal and is left unchanged.
4. Keep the database, `.lock`, `-wal`, `-shm`, and `-journal` together. The
   inspector requires the database `user_version` to exactly equal its
   compiled schema and never upgrades an older store.

## Run the bounded check

```bash
fathomdb doctor data-plane-integrity \
  --max-work 10000 --max-findings 100 \
  --json ./store.sqlite
```

Omit `--check` to run all five checks in canonical order. To narrow the run,
repeat `--check` with `dependency_chain`, `active_searchable_orphans`,
`projection_generation`, `mutation_readiness`, or `edge_endpoint_integrity`.
Bounds are 1–10,000 work units and 1–100 findings. A bounded or failed run
returns no partial report.

The inspection process validates the request first, takes the existing product
lock without changing its contents, checks the sidecars, then opens SQLite with
`immutable=1`, read-only, and query-only enforcement. It does not create a
database or lock, migrate, repair, rebuild projections, checkpoint, or start
workers.

## Interpret the result

| Exit | Meaning | Operator action |
| ---: | --- | --- |
| `0` | All selected checks are clean. | Retain the JSON report as the witness. |
| `65` | One or more typed findings were returned. | Investigate the identifiers and cursors in `report.findings`. |
| `70` | Request, availability, schema, runtime, or corruption refusal. | Branch on `reason`; do not retry by modifying the store. |
| `71` | The product lock is held or recovery sidecar state is non-empty. | Stop the owning process or investigate the pending sidecar, then retry. |

Errors use the `fathomdb.doctor.data-plane-integrity.v1` envelope. Stable
inspection reasons are `inspection_unavailable`, `inspection_lock_missing`,
`inspection_not_quiescent`, `runtime_configuration`,
`database_schema_mismatch`, and `integrity_corrupt`. The envelope intentionally
omits raw SQLite diagnostics and row content.

The report is diagnostic evidence, not repair authorization. Use a separately
reviewed recovery procedure if a finding requires mutation.

## Upgrading a Schema 37 or 38 database before inspection

On the `0.8.32` target line (Schema 39; published `0.8.31` used Schema 38),
`fathomdb doctor data-plane-integrity` and `Engine::open` both require
`user_version == 39`. To upgrade an existing quiescent Schema 38 store (or
chained Schema 37 store) before inspecting or opening it under `0.8.32`, run the
offline migrate command:

```bash
fathomdb migrate --backup-dir ./backups --json ./store.sqlite
fathomdb doctor check-integrity --full --json ./store.sqlite
```

`fathomdb migrate` acquires the exclusive `.lock`, refuses pending erasures or
genuinely dangling edge endpoints before mutating anything, writes a
SHA-256-verified backup in `--backup-dir`, upgrades `38 -> 39` (or chained
`37 -> 38 -> 39`) atomically in a single transaction, and runs post-upgrade
`check-integrity --full`. See the
[CLI reference](../reference/cli.md#migrate-root) for full details and exit
codes.
