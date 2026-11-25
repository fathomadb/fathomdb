# CLI

Binary: `fathomdb` (documentation target `0.8.32`, pre-publish; newest
published release: `0.8.31`). Operator-only; the CLI does **not** ship
application-surface verbs like `search`, `get`, or `list`. Use the SDK
for those.

Authoritative spec:
[`dev/interfaces/cli.md`](https://github.com/fathomadb/fathomdb/blob/release/0.8.31/dev/interfaces/cli.md);
recovery semantics owned by `dev/design/recovery.md`.

## Roots

- `fathomdb doctor <verb> ...` — read-only or artifact-producing
  diagnostics (17 subcommands).
- `fathomdb recover --accept-data-loss <sub-flag> ...` — the only
  lossy / non-bit-preserving root.
- `fathomdb migrate --backup-dir <dir> [--json] <db_path>` — offline,
  backup-verified Schema `38 -> 39` (and chained `37 -> 38 -> 39`) upgrade root
  (`0.8.32`).

## Output

- `--json` is the normative machine-readable contract on every verb.
- `--pretty` is a human-only formatter on verbs that explicitly
  document it.
- `recover --json` emits an NDJSON progress stream plus a final
  summary object. All other verbs emit a single JSON object.
- Doctor wrap shape: `{ "verb": "<verb-name>", ...flattened_report_fields... }`.
- Field names: serde `snake_case`.

## Doctor verbs

| Verb              | Synopsis                                                                       | Exit codes        |
| ----------------- | ------------------------------------------------------------------------------ | ----------------- |
| `check-integrity` | `fathomdb doctor check-integrity [--quick] [--full] [--round-trip] [--pretty]` | `0` / `65` / `70` / `71` |
| `data-plane-integrity` | `fathomdb doctor data-plane-integrity [--check <kind>]... [--max-work <n>] [--max-findings <n>] [--json] <db_path>` | `0` / `65` / `70` / `71` |
| `safe-export`     | `fathomdb doctor safe-export <out> [--manifest <path>]`                        | `0 / 66 / 70 / 71` |
| `verify-embedder` | `fathomdb doctor verify-embedder --identity <s> --dimension <n>`               | `0 / 70 / 71` |
| `trace`           | `fathomdb doctor trace --source-ref <id>`                                      | `0 / 70 / 71` |
| `dump-schema`     | `fathomdb doctor dump-schema`                                                  | `0 / 70 / 71` |
| `dump-row-counts` | `fathomdb doctor dump-row-counts`                                              | `0 / 70 / 71` |
| `dump-profile`    | `fathomdb doctor dump-profile`                                                 | `0 / 70 / 71` |
| `gpu`             | `fathomdb doctor gpu [--json] [pool flags]`                                    | `0` / `65` / `70`        |
| `reranker-gpu`    | `fathomdb doctor reranker-gpu [--json] [pool flags]` | `0` / `65` / `70` |
| `platform`        | `fathomdb doctor platform [--json]`                                            | `0` / `70`               |
| `cuda-allocator`  | `fathomdb doctor cuda-allocator [--json] [pool flags]`                         | `0` / `70`               |
| `cuda-libraries`  | `fathomdb doctor cuda-libraries [--json]`                                      | `0` / `70`               |
| `dump-mutations`  | `fathomdb doctor dump-mutations <collection> [--after-id <n>] [--limit <n>] [--json] <db_path>` | `0` / `70` / `71` |
| `orphan-provenance` | `fathomdb doctor orphan-provenance [--json] <db_path>`                        | `0` / `65` / `70` / `71` |
| `warm-cache`      | `fathomdb doctor warm-cache ...` — pre-fetch + verify the pinned default-embedder weights so the next open runs offline | `0 / 70` |
| `recompute-mean`  | `fathomdb doctor recompute-mean <db_path>` — re-derive and re-pin the corpus mean, re-quantizing every row in one transaction | `0 / 70 / 71` |

`verify-embedder` reports identity/dimension match or mismatch in its JSON and
returns 0 for either completed comparison; a mismatch is not exit 65. Ordinary
Engine-open failures still return 70 or 71. `trace` is the source-event diagnostic,
not governed `trace_dependency` / `traceDependency`.

Most database doctor commands use an ordinary Engine open: they may initialize a
missing path, start workers and change operational metadata. Stop the application
Engine before running them. `data-plane-integrity` instead uses the immutable,
quiescent route below. `safe-export` includes committed WAL data via SQLite
`VACUUM INTO`, then writes the export manifest; occupied output/export failure
returns 66, while an earlier open refusal retains 70/71.

`check-integrity --full` may emit doctor-only finding codes such as
`E_CORRUPT_INTEGRITY_CHECK`, `dangling_edge_endpoint`, and
`cross_scope_edge_endpoint` (semantic read-only edge verification).

### Reranker device diagnostic

`doctor reranker-gpu` resolves `FATHOMDB_RERANK_DEVICE` without opening a
database or loading model weights. Its JSON schema is
`fathomdb.doctor.reranker-gpu.v1`; it reports the requested policy, compiled
CUDA support, available devices, and effective device. An invalid CUDA
ordinal returns 65; an unavailable runtime or a build without
`default-reranker` returns 70.

### Data-plane integrity

`fathomdb doctor data-plane-integrity` runs bounded read-only checks over
dependency, serving-projection, and edge-endpoint authority. The five check
kinds—`dependency_chain`, `active_searchable_orphans`, `projection_generation`,
`mutation_readiness`, and `edge_endpoint_integrity`—run in canonical order;
omit `--check` to select all. `edge_endpoint_integrity` detects unresolved
same-scope active edge endpoints (`dangling_edge_endpoint`, severity `error`,
e.g. after incomplete `bulk_non_strict` ingestion) and cross-scope active edge
anomalies (`cross_scope_edge_endpoint`, severity `critical`) read-only without
repairing or mutating the database.
`--max-work` accepts 1 through 10,000 and `--max-findings` accepts 1 through
100. A failure returns no partial report. Stop every FathomDB process using the
store first. The command requires the existing product `.lock`, refuses a
non-empty `-wal` or `-journal`, and permits an existing `-shm` without changing
it. It opens the database through SQLite's immutable read-only mode with
query-only enforcement; it does not migrate, repair, rebuild, start projection
workers, create files, or rewrite lock metadata. The database `user_version`
must exactly match the installed binary's compiled schema.

```bash
fathomdb doctor data-plane-integrity \
  --check dependency_chain --check projection_generation \
  --max-work 10000 --max-findings 100 --json ./store.sqlite
```

The versioned JSON success envelope is
`{schemaVersion:"fathomdb.doctor.data-plane-integrity.v1",status,report}`.
Errors use the same schema with `status:"error"`,
`verb:"data-plane-integrity"`, `code:"FDB_DATA_PLANE_INTEGRITY"`, a stable
lower-snake `reason`, and RFC 6901 `fieldPath`. Clean exits `0`, findings exit
`65`, request/bound/integrity, unavailable-input, runtime, and schema failures
exit `70`; a held product lock or non-empty recovery sidecar exits `71`.
Inspection failures use `inspection_unavailable`, `inspection_lock_missing`,
`inspection_not_quiescent`, `runtime_configuration`,
`database_schema_mismatch`, or `integrity_corrupt` without exposing raw SQLite
diagnostics.

Install the 0.8.31 CLI at the exact version and confirm its identity:

```bash
cargo install fathomdb-cli --version '=0.8.31' --locked
fathomdb --version
```

In 0.8.31, the operator CLI is also published as a lockstep
`fathomdb-cli` PyPI binary wheel installable via `pip install 'fathomdb[cli]'`
(`pip install fathomdb-cli==0.8.31`) without requiring a Rust toolchain (while
keeping the base `fathomdb` SDK package free of console scripts).

During source qualification, use the [Rust install procedure](../install/rust.md).
Public 0.8.31 opens require a fresh or already-current schema-38 database;
an older non-empty database is rejected before product mutation. Upgrade
committed schema-37 stores explicitly offline using
`fathomdb migrate --backup-dir <dir> <db_path>`.

### GPU and platform diagnostics

`fathomdb doctor gpu [--json]` reports the existing
`fathomdb.doctor.gpu.v1` record. Its schema is unchanged. On confirmed ARM64
SBSA hardware, automatic policy reports `cuda_incompatible` with
`arm64_sbsa_unsupported` and continues on CPU; forced `cuda:N` exits `65`.
Explicit `cpu` never probes CUDA or the platform subprocess. An indeterminate
`nvidia-smi` platform probe does not make `doctor gpu` fail; it uses the normal
CUDA-provider diagnostic instead.

`fathomdb doctor platform [--json]` is the companion database-free classifier.
Its JSON record is `fathomdb.doctor.platform.v1` with ordered fields
`schema_version`, `platform_class`, `tegra_family`, `sbsa_capable`,
`l4t_release`, and `reason`. `platform_class` is one of `tegra`,
`arm64_sbsa`, `generic_aarch64`, `non_aarch64`, or `unknown`. The classifier
uses filesystem Tier 1 plus an ordered absolute `nvidia-smi` Tier 2 probe; an
unavailable, timed-out, or nonzero Tier 2 result is `unknown` and exits `70`.

`fathomdb doctor cuda-allocator [--json]` (0.8.28) reports the CUDA
allocator this process uses, as an SDK process would decide it: the Jetson
private memory pool or the earlier default-pool/synchronous path. Its JSON
record is `fathomdb.doctor.cuda-allocator.v1` with ordered fields
`schema_version`, `built`, `module_load_init`, `pool_mode`,
`pool_mode_source`, `path`, `reason`, `pool_max_size_bytes`,
`pool_max_size_source`, `release_threshold`,
`pool_release_threshold_source`, and `cuda_context_state`; each `*_source`
is `sdk` (a flag), `env` or `default`. `module_load_init` is `ran` because
the command runs the Node and Python load hook's helper itself; a Rust SDK
process on the same path reports `not_at_load`. An
explicit `cpu` policy, confirmed ARM64 SBSA hardware, or a binary without
CUDA returns `reason: "not_applicable"` without touching CUDA. On a Jetson
AGX Orin 64 GB with a pool-enabled build it reports `"path":"private"`,
`"reason":"private_pool"`, a 3 GiB pool and release threshold `"0"`. It exits
`0`, or `70` when the device probe fails. `doctor gpu` is unchanged.

The pool flags of `gpu`, `reranker-gpu` and `cuda-allocator` (0.8.28) are
`--cuda-pool <auto|on|off>`, `--cuda-pool-max-size <bytes|<n>G|<n>M>` (at
least 32 MiB) and `--cuda-pool-release-threshold <0|max>`, the same settings
an SDK open takes ([configuration](config.md#engine-configuration)); each
wins over its `FATHOMDB_POOL_*` variable, and an invalid value exits `2`.
No database verb builds a GPU device, so none takes them.

`fathomdb doctor cuda-libraries [--json]` (0.8.28) reports the CUDA
libraries this binary loads for a GPU: the driver, cuBLAS and cuRAND (and
cuBLASLt or NVRTC when loaded), each with the file opened, its path and
version, and whether one is refused. Its JSON record is
`fathomdb.doctor.cuda-libraries.v1` with ordered fields `schema_version`,
`built`, `built_major`, `libraries`, `refusal` and `maps`. A refusal
(`cuda_library_unavailable`: reason `missing`, `wrong_major` or
`missing_symbol`) carries the files tried and the remedy, and the command
exits `70`; otherwise it exits `0`. See
[GPU acceleration](../embedder.md#cuda-library-and-gpu-requirements).

For a confirmed classic Jetson/Tegra CUDA host, use the separate 0.8.31
[Python installation route](../install/python.md#jetson-tegra-cuda) after
publication and manifest verification: `fathomdb==0.8.31+tegra` from
`https://fathomadb.github.io/fathomdb/tegra/simple/`. It uses one index and an
exact version; do not use a floating requirement or `--extra-index-url`.
Unsupported targets have no supported Tegra CUDA route.

### `dump-mutations` — op-store read-back

A read-only operator diagnostic that pages the op-store mutation log
(`operational_mutations`) for one `append_only_log` collection. It is a
diagnostic dump over operator/log data (like `dump-row-counts` / `trace`), **not**
an application query verb — there is no `search` / `get` / `list` surface over
application content. `--limit` bounds the page (default `1000`; the engine clamps
to a ~1M cap); `--after-id` is an exclusive cursor for the next page. An empty,
unknown, or unregistered collection (or an `--after-id` past the end) is a normal
absence and exits `0`.

```bash
fathomdb doctor dump-mutations events --limit 2 --json ./store.sqlite
```

```json
{
  "verb": "dump-mutations",
  "collection": "events",
  "after_id": null,
  "limit": 2,
  "count": 2,
  "rows": [
    { "id": 1, "collection": "events", "record_key": "k0", "op_kind": "append",
      "payload": "{\"n\":0}", "schema_id": null, "write_cursor": 1 },
    { "id": 2, "collection": "events", "record_key": "k1", "op_kind": "append",
      "payload": "{\"n\":1}", "schema_id": null, "write_cursor": 2 }
  ],
  "next_after_id": 2
}
```

Resume the next page with `--after-id 2`. When a page is short (fewer rows than
`--limit`), `next_after_id` is `null` — the log is exhausted at that cursor.

### `orphan-provenance` — per-`source_id` census

Read-only. Reports every provenance bucket in the canonical tables and, load-
bearingly, how many rows are reachable by **no** erasure verb.

```bash
fathomdb doctor orphan-provenance --json ./app.sqlite
```

```json
{
  "verb": "orphan-provenance",
  "sources": [
    { "source_id": "tenant-a", "rows": 3, "governed_rows": 0, "reserved": false },
    { "source_id": "tenant-b", "rows": 2, "governed_rows": 1, "reserved": false },
    { "source_id": "_legacy:pre-0.8.20", "rows": 9, "governed_rows": 0, "reserved": true }
  ],
  "total_rows": 14,
  "unerasable_rows": 0
}
```

`unerasable_rows` counts canonical rows carrying **neither** a `source_id` nor a
`logical_id`. `purge` keys on `logical_id` and `erase_source` keys on
`source_id`, so such a row can never be deleted on request. It exits `65`
(`DOCTOR_FOUND_ISSUES`) when that count is non-zero, `0` otherwise.

`reserved: true` marks the engine's `_`-prefixed namespace. Those buckets are
erasable only through `fathomdb recover --excise-source`, never through the SDK
`erase_source` verb. They are reported, but they are **not** an issue.

See [Erasure](../operations/erasure.md) for the full erasure boundary.

## Recover root

```text
fathomdb recover --accept-data-loss
  [--truncate-wal]
  [--complete-erasures]
  [--rebuild-vec0]
  [--rebuild-projections]
  [--excise-source <id>]
  [--excise-collection <name> --excise-record-key <key>]
  [--json]
  <db_path>
```

Exit codes: `0` / `64` / `70` / `71`.

`--accept-data-loss` is declared on the `recover` parser only;
`doctor` verbs reject it as unknown.

`--rebuild-projections` is the canonical regenerate workflow for failed
or stale projections. There is no separate `fathomdb regenerate`
command.

`--excise-collection` and `--excise-record-key` are required together;
they erase every append-only-log version of one op-store record key
plus its latest-state row.

### Complete activated erasure debt

`fathomdb recover --accept-data-loss --complete-erasures --json ./store.sqlite`
finishes owed activated cleanup; it does not activate arbitrary queued intent.
It retains exact current-schema admission, original-sink reconciliation and WAL
truncation prerequisites. Unavailable original sinks or pinned external readers
remain incomplete outcomes, not success. Ordinary recovery exit classes below
apply. See [erasure](../operations/erasure.md#schedule-an-explicit-maintenance-window).

Projection/vector rebuild is not certified numerical compatibility repair. In
particular, no-provider rebuild of previously dense state has a known limitation:
it can invalidate that state without a usable provider.

### `--excise-source` vs the SDK erasure verbs

Since 0.8.20 the SDK ships its own erasure verbs, so the CLI is no
longer the only route:

| Need | Use |
| ---- | --- |
| erase one **governed** node by `logical_id` | SDK `purge` |
| erase every row from one **provenance** | SDK `erase_source` |
| erase inside the engine's reserved `_`-prefixed namespace | `fathomdb recover --excise-source` (CLI only) |
| erase one op-store record key | `fathomdb recover --excise-collection … --excise-record-key …` (CLI only) |

```bash
fathomdb recover --accept-data-loss --excise-source <id> --json ./app.sqlite
```

See [Erasure](../operations/erasure.md) for the full boundary — what
erasure reaches and what it does not.

## Migrate root

```text
fathomdb migrate --backup-dir <dir> [--json] <db_path>
```

Upgrades a quiescent committed Schema `38` database offline to Schema `39`, or
chains `37 -> 38 -> 39` for a committed Schema `37` database (`0.8.32`).
`Engine::open` never auto-migrates and continues refusing `user_version != 39`.

Execution guarantees:

1. Acquires `<db_path>.lock` exclusively (`exit 71` when held) and refuses
   non-empty rollback journals.
2. Verifies integrity and dispatches on `PRAGMA user_version`: `38` runs
   `38 -> 39`; `37` chains `37 -> 38` then `38 -> 39` (`exit 70` on `36` and
   earlier, `39` `E_MIGRATE_ALREADY_CURRENT`, `40+`, or corrupt inputs).
3. Fails closed (`exit 70`, `E_MIGRATE_PENDING_ERASURE`) before backup or DDL if
   any pending physical erasure closure, active/queued erasure request, root/FTS
   sanitization debt, or pending telemetry redaction row exists.
4. Runs read-only edge-endpoint preflight reporting both
   `dangling_edge_endpoints` and `superseded_edge_endpoints`, failing closed
   (`exit 70`, `E_MIGRATE_DANGLING_EDGES`) whenever `dangling_edge_endpoints > 0`
   while admitting stores where `dangling_edge_endpoints == 0` and
   `superseded_edge_endpoints > 0`.
5. Checkpoints WAL (`TRUNCATE`, `busy == 0`), writes a SHA-256-verified backup
   and `.sha256` receipt into `--backup-dir` without overwriting prior backups
   (chained `37 -> 39` runs preserve the original pre-38 backup and report
   `from_version: 37`, `to_version: 39`).
6. Applies Schema 39 DDL (`fathomdb_schema::STEP_39_SQL`), offline 8-column
   `vec0` reshape (`scope TEXT`), and `PRAGMA user_version = 39` inside a single
   `BEGIN IMMEDIATE` transaction (running `STEP_38_SQL` first when starting from
   Schema `37` and rolling back cleanly on any SQL failure).
7. Verifies current-schema admission and runs post-upgrade
   `check-integrity --full` (`exit 0` clean, `exit 65` if findings exist).

Exit codes: `0` (upgraded and clean), `65` (upgraded with post-upgrade integrity
findings), `70` (unrecoverable preflight/version/DDL/scope-violation refusal,
mapping `EngineError::ScopeViolation` to `"ScopeViolationError"`), `71` (lock
held or busy WAL checkpoint).

## Exit-code classes

| Code | Meaning                                                              |
| ---- | -------------------------------------------------------------------- |
| `0`  | successful completion; no findings requiring non-zero exit           |
| `64` | recovery completed because lossy action was explicitly accepted      |
| `65` | doctor / verification surface found actionable non-clean state       |
| `66` | export / materialization failure on an artifact-producing doctor verb|
| `70` | unrecoverable command failure                                        |
| `71` | lock-held or equivalent precondition-blocked outcome                 |

The full engine-error → exit-code mapping is in the locked spec.

## Logical-id verbs

Logical-id lifecycle landed as **SDK** verbs, not CLI verbs:
`transition` (soft-delete / undelete / promote) and `purge`
(irreversible hard-erase, deleted-first). There is no
`fathomdb purge-logical-id` command, and **no restore verb of any
kind** — a purge is not reversible.

See [Python API](python-api.md) / [TypeScript API](typescript-api.md).

## See also

- [Errors](errors.md)
- [Install — Rust / CLI](../install/rust.md)
- Locked spec: [`dev/interfaces/cli.md`](https://github.com/fathomadb/fathomdb/blob/release/0.8.31/dev/interfaces/cli.md)
