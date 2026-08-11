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
issue-0007 column sequence, not an upgrade path for arbitrary historical
FathomDB databases.

```bash
python scripts/experimental/upgrade_legacy_op_store.py \
  --input /path/to/legacy.sqlite \
  --output /path/to/verified-copy.sqlite
```

The tool does all of the following:

- Opens the input with SQLite read-only mode and runs `integrity_check` plus
  `quick_check`.
- Refuses every shape except the exact eight-column legacy table above.
- Uses SQLite's backup API to create the explicit new output path, so it never
  writes the input database or its sidecars.
- Adds only `schema_id TEXT` and `write_cursor INTEGER NOT NULL DEFAULT 0` to
  the output. It does not infer or rewrite the legacy `mutation_order` data.
- Rechecks output integrity, then requires the invoking FathomDB Python runtime
  to open the output with no default embedder and execute `read.collection`.

The output is a candidate for deliberate review and extraction only. The
defaulted zero cursors are sufficient for current read-back but are not a
reconstructed historical cursor sequence. Archive the original first; do not
interpret this experiment as official backwards-compatibility support.

The focused synthetic test takes a SHA-256 witness of its input before and
after the tool runs; the original input bytes are unchanged. This proves the
tool's copy-only boundary without reading or operating on the Memex database.
