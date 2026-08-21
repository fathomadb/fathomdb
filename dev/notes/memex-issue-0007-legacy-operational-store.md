# Memex issue 0007: legacy operational-store incompatibility

## Classification

This is a database-version incompatibility, not SQLite corruption or a Memex
provider failure. The observed store has the pre-rewrite
`operational_mutations` columns:

```text
id, collection_name, record_key, op_kind, payload_json, source_ref, created_at,
mutation_order
```

Current FathomDB requires `schema_id` and `write_cursor` in that table. Its
governed `read.collection` query selects both fields, so a healthy older SQLite
file fails with `no such column: schema_id` and the binding currently maps that
SQLite detail to `StorageError`.

The 0.6 rewrite deliberately made a clean compatibility break: see
`ADR-0.6.0-no-shims-policy.md`, "Decision" and "Consequences." Its bootstrap
migration creates `operational_mutations` only when it is absent; it does not
reshape an existing pre-rewrite table. This leaves the old table invisible to
the normal versioned migration path.

## Bounded experimental tool

`scripts/experimental/upgrade_legacy_op_store.py` is intentionally **NOT
OFFICIALLY SUPPORTED**. It is a copy-only, opt-in workaround for precisely the
issue-0007 column sequence with current-reader-compatible metadata, not an
upgrade path for arbitrary historical FathomDB databases.

```bash
python scripts/experimental/upgrade_legacy_op_store.py \
  --input /path/to/legacy.sqlite \
  --output /path/to/verified-copy.sqlite
```

The tool does all of the following:

- Never opens the input with SQLite. SQLite read-only mode can still mutate a
  live input's `-shm` sidecar while taking locks.
- Copies the main database plus any `-wal` and `-shm` sidecars through ordinary
  file reads into a private temporary directory beneath the requested output.
  It runs `integrity_check` and `quick_check` only against that private snapshot.
- Refuses every shape except the exact eight-column sequence with an `INTEGER`
  primary-key `id`, nullable `TEXT source_ref`, `TEXT NOT NULL` collection /
  record / operation / payload fields, and `INTEGER NOT NULL` timestamp /
  mutation-order fields. It rejects the actual pre-0.6 v0.5.x `TEXT PRIMARY
  KEY id` shape because the current reader requires an integer pagination id.
- Uses SQLite's backup API to create and alter a private candidate only. It adds
  only `schema_id TEXT` and `write_cursor INTEGER NOT NULL DEFAULT 0`, and does
  not infer or rewrite legacy `mutation_order` data.
- Rechecks candidate integrity, requires the invoking FathomDB Python runtime
  to open it with no default embedder and execute `read.collection`, then
  checkpoints private WAL state and atomically creates the requested output.
  Any failure before that publish leaves no altered requested output path.

The output is a candidate for deliberate review and extraction only. The
defaulted zero cursors are sufficient for current read-back but are not a
reconstructed historical cursor sequence. Archive the original first; do not
interpret this experiment as official backwards-compatibility support.

The focused synthetic live-WAL test takes SHA-256 witnesses of the input main
database, `-wal`, and `-shm` sidecars before and after the tool runs. All three
original artifacts remain byte-unchanged. This proves the tool's copy-only
boundary without reading or operating on the Memex database.
