---
title: CLI Public Interface
date: 2026-07-29
target_release: 0.8.21
desc: Public CLI surface for 0.8.21
blast_radius: src/rust/crates/fathomdb-cli/src/lib.rs; design/recovery.md; design/errors.md
status: locked
---

# CLI Interface

Public CLI surface for the 0.8.21 operator binary. The canonical verb table and
recovery semantics are owned by `design/recovery.md`; this file owns concrete
flag spelling, root command paths, and exit-code classes.

## Roots

- `fathomdb recover --accept-data-loss <sub-flag>...`
- `fathomdb doctor <verb> ...`

The CLI is **operator-only**. It does not mirror the SDK application
surface and does not ship `search` / `get` / `list` query verbs.

The 0.8.0 `doctor dump-mutations` verb is **not** an exception to this: it is a
read-only operator **diagnostic over the mutation log** (`operational_mutations`),
in the same `dump-*` family as `dump-row-counts` / `dump-schema` / `trace`, reading
op-store rows back over the existing engine read seam. It is distinct from — and
does not introduce — the still-absent `search` / `get` / `list` application query
surface over `canonical_nodes` (Option B in `ADR-0.6.0-cli-scope`, still rejected;
see that ADR's 2026-06-06 amendment).

## Output posture

- `--json` is the normative machine-readable contract on every verb.
- `doctor check-integrity` emits a single JSON object.
- `doctor check-integrity --full` may emit doctor-only finding codes such as
  `E_CORRUPT_INTEGRITY_CHECK`.
- `recover` JSON output is a progress stream plus summary, owned by
  `design/recovery.md`.
- `--pretty` is a human-only formatter on verbs that explicitly document it;
  it is not a separate machine schema.

## Exit-code classes

| Code | Stable meaning                                                       | Primary owner                    |
| ---- | -------------------------------------------------------------------- | -------------------------------- |
| `0`  | successful completion with no findings that require a non-zero exit  | this file                        |
| `64` | recovery completed only because lossy action was explicitly accepted | this file + `design/recovery.md` |
| `65` | doctor/verification surface found actionable non-clean state         | this file + `design/recovery.md` |
| `66` | export/materialization failure on an artifact-producing doctor verb  | this file + `design/recovery.md` |
| `70` | unrecoverable command failure                                        | this file                        |
| `71` | lock-held or equivalent precondition-blocked outcome                 | this file + `design/bindings.md` |

## Doctor verbs

| Verb              | Synopsis                                                                       | Exit class                          |
| ----------------- | ------------------------------------------------------------------------------ | ----------------------------------- |
| `check-integrity` | `fathomdb doctor check-integrity [--quick] [--full] [--round-trip] [--pretty]` | `doctor-check-*` = 0 / 65 / 70 / 71 |
| `safe-export`     | `fathomdb doctor safe-export <out> [--manifest <path>]`                        | `doctor-export-*` = 0 / 66 / 71     |
| `verify-embedder` | `fathomdb doctor verify-embedder --identity <s> --dimension <n>`               | `doctor-check-*` = 0 / 65           |
| `trace`           | `fathomdb doctor trace --source-ref <id>`                                      | `doctor-check-*`                    |
| `dump-schema`     | `fathomdb doctor dump-schema`                                                  | `doctor-check-*`                    |
| `dump-row-counts` | `fathomdb doctor dump-row-counts`                                              | `doctor-check-*`                    |
| `dump-profile`    | `fathomdb doctor dump-profile`                                                 | `doctor-check-*`                    |
| `gpu`             | `fathomdb doctor gpu [--json]`                                                 | `doctor-gpu` = 0 / 65 / 70          |
| `reranker-gpu`    | `fathomdb doctor reranker-gpu [--json]`                                        | `0 / 65 / 70`                       |
| `recompute-mean`  | `fathomdb doctor recompute-mean <db_path> [--json]`                            | `doctor-check-*` = 0 / 70 / 71      |
| `dump-mutations`  | `fathomdb doctor dump-mutations <collection> [--after-id <n>] [--limit <n>] [--json] <db_path>` | `0 / 70 / 71`      |
| `warm-cache`      | `fathomdb doctor warm-cache ...` (EU-5b)                                       | `doctor-check-*`                    |
| `orphan-provenance` | `fathomdb doctor orphan-provenance [--json] <db_path>`                       | `doctor-check-*` = 0 / 65 / 70 / 71 |

`doctor-check-*` means the verb may use the exit-code class set `{0, 65, 70,
71}` depending on clean/findings/unrecoverable/lock-held outcome.

`doctor-gpu` (0.8.23 Slice 70) emits one canonical compact stdout JSON object
with keys in exact order: `schema_version`, `policy`, `cuda_compiled`, `status`,
`effective_device`, `devices`, `reason`, `selected_uuid`. Device-object keys
are exactly `visible_ordinal`, `uuid`, `name`, `compute_capability` in that
order. `schema_version` is `"fathomdb.doctor.gpu.v1"`.
`reason` and `selected_uuid` are always present and are JSON `null` when absent. The object
has exactly one trailing newline. The command reads
`FATHOMDB_EMBED_DEVICE` but exposes no setter or configuration writer;
it does not open a database, load or download a model, write configuration, or
initialize an engine. Its ordered `devices` inventory contains process-visible
CUDA UUIDs and their `CUDA_VISIBLE_DEVICES`-relative ordinals only; it neither
reports nor infers physical host ordinals. A selected UUID must match one
inventory member.

`doctor reranker-gpu` (0.8.23 Slice 71) is a separate database-free,
model-loader-free policy/provider diagnostic. Its JSON `subsystem` is always
`"reranker"`; its schema is `fathomdb.doctor.reranker-gpu.v1`. A successful
record names only cross-encoder policy/device evidence; it never attests
embedding, SQLite candidate retrieval, exact database scoring, or inference
over a real model. A typed error object reports malformed/forced policy
failure. This separation preserves the Slice 70 doctor v1 schema.

With `FATHOMDB_RERANK_DEVICE=cpu`, both text and `--json` exit `0` and emit
only the reranker v1 record (`effective_device: "cpu"`, empty inventory,
`reason: null`). Forced-policy/device refusals exit `65`; malformed policy or
an artifact without the default reranker exits `70`. This command never opens a
database, loads/downloads a model, or writes its current directory/cache.

Normal `Engine::open` resolution and `doctor gpu` have distinct result mappings.
Open uses `DeviceResolution`, which may describe automatic CPU selection with a
`cuda_probe_failed` reason. The CLI produces `DoctorGpuDiagnosticResult` instead;
it consumes raw driver, inventory, and allocation/provider-probe evidence and
must not serialize `DeviceResolution` as the diagnostic. Thus
`CudaProbeError::ProbeFailed` maps to `probe_failed`, not a policy-satisfied
automatic result, even though the automatic diagnostic reports typed CPU as its
effective device.

The production Candle classifier is closed and code-based. Missing dynamic
driver, `CUDA_ERROR_NO_DEVICE`, and `CUDA_ERROR_STUB_LIBRARY` are unavailable.
Exactly `CUDA_ERROR_SYSTEM_DRIVER_MISMATCH`,
`CUDA_ERROR_COMPAT_NOT_SUPPORTED_ON_DEVICE`, `CUDA_ERROR_NO_BINARY_FOR_GPU`,
`CUDA_ERROR_UNSUPPORTED_PTX_VERSION`, `CUBLAS_STATUS_ARCH_MISMATCH`, and
`CUBLAS_STATUS_NOT_SUPPORTED` are incompatible. Unknown errors,
`CUDA_ERROR_OUT_OF_MEMORY`, and `CUBLAS_STATUS_ALLOC_FAILED` are
`probe_failed`. Build target, CUDA toolkit, and driver-version provenance are
artifact-witness facts and are not fields in the doctor v1 schema.

`devices` is `[]` if enumeration did not succeed; otherwise it is the observed
ordered inventory. `selected_uuid` is `null` unless CUDA was selected. The
twelve-row semantic outcome matrix is:

| Requested policy / observation | CUDA activity | Status | Effective device | Devices | Selected UUID | Exit |
| --- | --- | --- | --- | --- | --- | --- |
| `cpu` (any artifact) | none | `selected_cpu_no_cuda` | `cpu` | `[]` | `null` | `0` |
| `auto`, CPU-only artifact | none | `cuda_not_compiled` | `cpu` | `[]` | `null` | `0` |
| `auto`, unavailable evidence | driver-presence, enumeration, or ordinal mapping | `cuda_unavailable` | `cpu` | `[]` before inventory; otherwise observed inventory | `null` | `0` |
| `auto`, listed compatibility/architecture evidence | enumeration or mapped-device probe | `cuda_incompatible` | `cpu` | `[]` before inventory; otherwise observed inventory | `null` | `0` |
| `auto`, unknown, OOM, or allocation/provider failure | enumeration or mapped-device probe | `probe_failed` | `cpu` | `[]` before inventory; otherwise observed inventory | `null` | `70` |
| `auto`, selected device allocation/provider probe succeeds | enumeration + mapped-device probe | `selected_cuda` | selected `cuda:N` | observed inventory | matching UUID | `0` |
| forced `cuda:N`, CUDA not compiled | none | `cuda_not_compiled` | `null` | `[]` | `null` | `65` |
| forced `cuda:N`, unavailable evidence | driver-presence, enumeration, or ordinal mapping | `cuda_unavailable` | `null` | `[]` before inventory; otherwise observed inventory | `null` | `65` |
| forced `cuda:N`, listed compatibility/architecture evidence | enumeration or mapped-device probe | `cuda_incompatible` | `null` | `[]` before inventory; otherwise observed inventory | `null` | `65` |
| forced `cuda:N`, unknown, OOM, or allocation/provider failure | enumeration or mapped-device probe | `probe_failed` | `null` | `[]` before inventory; otherwise observed inventory | `null` | `70` |
| forced `cuda:N`, selected device allocation/provider probe succeeds | enumeration + mapped-device probe | `selected_cuda` | selected `cuda:N` | observed inventory | matching UUID | `0` |
| malformed, legacy, or otherwise invalid policy | none | `invalid_policy` | `null` | `[]` | `null` | `70` |

Forced CUDA never becomes a CPU report. Invalid policy invokes no CUDA provider
code.

The normative text output is exactly this newline-terminated sequence; the
`device=` line repeats once per ordered inventory member and contains the same
canonical compact device JSON used by JSON mode:

```text
doctor gpu
policy=<JSON string>
cuda_compiled=<true|false>
status=<status>
effective_device=<cpu|cuda:N|null>
reason=<reason|null>
devices=<decimal count>
device={"visible_ordinal":N,"uuid":"...","name":"...","compute_capability":"..."|null}
selected_uuid=<UUID|null>
```

`dump-mutations` (0.8.0; gap F4-READ / reserved-gap-34) is a read-only operator
diagnostic that pages op-store (`operational_mutations`) rows for one
`append_only_log` collection back over the existing `read.collection` /
`read.mutations` engine seam (Slice 30, index-driven by Slice 33). It is
**CLI-only** (no SDK-parity obligation; the SDK `read.*` verbs are the separate
application surface). An **empty page** — empty / unknown / unregistered
collection, or `--after-id` past the end — is a normal absence and exits `0`
(never `65`/Findings). Exit class set `{0, 70, 71}`.

`orphan-provenance` (0.8.20 Slice 5d / R-20-E8) is a read-only per-`source_id`
census over the canonical tables. It reports each provenance bucket with its
row count, how many of those rows are also `logical_id`-addressable, and
whether the bucket is in the engine's reserved `_`-prefixed namespace. The
load-bearing field is `unerasable_rows` — canonical rows carrying NEITHER a
`source_id` (for `erase_source`) NOR a `logical_id` (for `purge`), i.e.
reachable by no erasure verb. A non-zero count exits `65`
(`DOCTOR_FOUND_ISSUES`). Rows under `_legacy:pre-0.8.20` are reported but are
NOT an issue — they are fully erasable through the CLI recovery seam.
CLI-only; no SDK parity.

## Recover root

`recover` is the only lossy / non-bit-preserving root.

```text
fathomdb recover --accept-data-loss
  [--truncate-wal]
  [--rebuild-vec0]
  [--rebuild-projections]
  [--excise-source <id>]
  [--excise-collection <name> --excise-record-key <key>]
  [--json]
  <db_path>
```

Exit class: `recover-*` = 0 / 64 / 70 / 71.

`--excise-collection` and `--excise-record-key` (0.8.20 Slice 5b / R-20-E7) are
declared with `requires` on each other, so clap rejects either alone. Together
they erase every append-only-log version of one op-store record key plus its
latest-state row.

### The CLI is no longer the only erasure route

Since 0.8.20 the SDK ships `purge` (one governed node, by `logical_id`) and
`erase_source` (every row carrying one provenance) in all three bindings, so an
embedded consumer with no `fathomdb` binary on `PATH` can discharge a deletion
obligation without the CLI. Two capabilities stay CLI-only, deliberately:

- `--excise-source` is the ONLY route into the engine's reserved `_`-prefixed
  provenance namespace (`_engine:*`, `_legacy:pre-0.8.20`). `SourceId::new`
  refuses those spellings, so the governed verb cannot reach them — a single
  SDK-reachable call against `_legacy:pre-0.8.20` would wipe every pre-0.8.20
  anonymous row at once.
- `--excise-collection` / `--excise-record-key`, which has no SDK peer.

`excise_source` therefore REMAINS deliberately non-allowlisted: it stays the
recovery seam.

`--accept-data-loss` is declared on the `recover` parser only. `doctor` verbs
reject it as unknown.

`--rebuild-projections` is the canonical regenerate workflow for failed or
stale projections. The docs may refer to "regenerate" as the workflow name, but
there is no separate `fathomdb regenerate` command.

## Error to exit-code mapping

The CLI dispatcher translates engine error variants (and CLI-detected
preconditions) to the exit-code classes above. This table binds each variant
to its class.

| Source error variant                             | Exit code | Class         |
| ------------------------------------------------ | --------- | ------------- |
| (clean completion)                               | 0         | success       |
| recover sub-action gated by `--accept-data-loss` | 64        | data-loss-ack |
| `doctor check-integrity` findings non-empty      | 65        | findings      |
| `doctor safe-export` failed manifest/export step | 66        | artifact-fail |
| `EngineError::Storage`                           | 70        | unrecoverable |
| `EngineError::Projection`                        | 70        | unrecoverable |
| `EngineError::Vector`                            | 70        | unrecoverable |
| `EngineError::Embedder`                          | 70        | unrecoverable |
| `EngineError::Scheduler`                         | 70        | unrecoverable |
| `EngineError::OpStore`                           | 70        | unrecoverable |
| `EngineError::Overloaded`                        | 70        | unrecoverable |
| `EngineError::SchemaValidation`                  | 70        | unrecoverable |
| `EngineError::WriteValidation`                   | 70        | unrecoverable |
| `EngineError::EmbedderNotConfigured`             | 70        | unrecoverable |
| `EngineError::KindNotVectorIndexed`              | 70        | unrecoverable |
| `EngineError::EmbedderDimensionMismatch{..}`     | 70        | unrecoverable |
| `EngineOpenError::DatabaseLocked{..}`            | 71        | lock-held     |
| `EngineError::Closing`                           | 71        | lock-held     |
| `EngineOpenError::Corruption(..)`                | 70        | unrecoverable |
| `EngineOpenError::IncompatibleSchemaVersion{..}` | 70        | unrecoverable |
| `EngineOpenError::MigrationError{..}`            | 70        | unrecoverable |
| `EngineOpenError::EmbedderIdentityMismatch{..}`  | 70        | unrecoverable |
| `EngineOpenError::EmbedderDimensionMismatch{..}` | 70        | unrecoverable |
| `EngineOpenError::Io{..}`                        | 70        | unrecoverable |

## JSON output wrapping

`fathomdb-cli` owns top-level discriminator wrapping. The engine returns
typed report structs; the CLI serializes them under a `verb` discriminator.

- All `--json` output is one JSON object (or an NDJSON stream for `recover`).
- Doctor verb wrapping pattern: `{ "verb": "<verb-name>", ...flattened_engine_report_fields... }`.
- Non-flat reports nest naturally. For example, `IntegrityReport` serializes
  as `{ "verb": "check-integrity", "physical": {...}, "logical": {...}, "semantic": {...} }`.
- `doctor dump-mutations` (0.8.0) serializes as `{ "verb": "dump-mutations",
  "collection", "after_id" (or null), "limit", "count", "rows": [ { "id",
  "collection", "record_key", "op_kind", "payload", "schema_id", "write_cursor" }
  … ordered by`id`], "next_after_id" }`. The CLI serializes the engine
  `OpStoreRow` rows inline; `next_after_id` is the last row's `id` iff a full page
  was returned (resume with `--after-id <next_after_id>`, exclusive cursor → no
  overlap), else `null`.
- Field name policy: serde default `snake_case`. Any divergence from engine
  field spellings lives in the CLI serialization layer; the engine report
  structs are not renamed to satisfy CLI spelling requirements.
