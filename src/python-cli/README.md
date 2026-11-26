# fathomdb-cli

Prebuilt `fathomdb` operator CLI binary wheel for **FathomDB**, a local-first embedded retrieval
engine built on SQLite.

This package installs the `fathomdb` executable directly into your Python environment without
requiring a Rust toolchain. It is an operator tool rather than a Python library API; for
application reads and writes, install the `fathomdb` Python SDK (`pip install fathomdb`).

## Install

Install alongside the Python SDK:

```bash
pip install 'fathomdb[cli]'
```

Or install the standalone CLI wheel directly:

```bash
pip install fathomdb-cli
```

Verify installation:

```bash
fathomdb --version
fathomdb doctor platform --json
```

## Root commands

- `fathomdb doctor <verb> [--json] [<db_path>]` — diagnostics, integrity checks, exports, and maintenance
- `fathomdb migrate --backup-dir <dir> [--json] <db_path>` — explicit offline Schema 38 to Schema 39 (and chained 37 -> 38 -> 39) upgrade with SHA-256-verified backup and full post-upgrade integrity check
- `fathomdb recover --accept-data-loss [flags] <db_path>` — explicit lossy recovery operations

## Exit codes

| Code | Meaning |
| --- | --- |
| `0` | Clean completion |
| `64` | `recover` completed with explicit `--accept-data-loss` |
| `65` | Diagnostics or post-upgrade integrity check reported findings |
| `66` | Export artifact failure |
| `70` | Unrecoverable failure or fail-closed precondition refusal |
| `71` | Lock held or WAL checkpoint busy |

## License

MIT. See `LICENSE` in this distribution.

Source and documentation: <https://github.com/fathomadb/fathomdb>
