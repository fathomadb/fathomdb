# Immutable integrity inspection

Use `fathomdb doctor data-plane-integrity` to check dependency and serving
projection authority without opening a serving engine or granting repair
authority.

## Before you run it

1. Install the exact CLI version paired with the store and confirm
   `fathomdb --version`. For 0.8.26, use
   `cargo install fathomdb-cli --version '=0.8.26' --locked` after the release
   is published.
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

Omit `--check` to run all four checks in canonical order. To narrow the run,
repeat `--check` with `dependency_chain`, `active_searchable_orphans`,
`projection_generation`, or `mutation_readiness`. Bounds are 1–10,000 work
units and 1–100 findings. A bounded or failed run returns no partial report.

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
