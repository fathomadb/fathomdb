# fathomdb-mcp

Prebuilt `fathomdb-mcp` Model Context Protocol (MCP) server binary wheel for
**FathomDB**, a local-first embedded retrieval engine built on SQLite.

This package installs the `fathomdb-mcp` stdio server executable directly into
your Python environment without requiring a Rust toolchain, compiled with
`--features default-embedder` so `search_mode: "hybrid"` works out of the box.
The base `fathomdb` Python SDK wheel stays free of console scripts; for Python
application APIs, install `fathomdb` (`pip install fathomdb`).

## Install

Install alongside the Python SDK:

```bash
pip install 'fathomdb[mcp]'
```

Or install the standalone MCP server wheel directly:

```bash
pip install fathomdb-mcp
```

Verify configuration without opening any store:

```bash
fathomdb-mcp --config ./mcp-config.json --check-config
```

## Security posture

- **Read-only by default (`allow_writes: false`):** Read-only profiles expose
  exactly 7 tools (`list_stores`, `use_store`, `orient`, `search`, `fetch`,
  `expand`, `feedback`) with bounded frozen-evidence retrieval.
- **Opt-in scoped writes:** Enabling writes requires both per-store
  `allow_writes: true` and an explicit non-empty writable caller `scope`
  policy. Write-enabled profiles expose 9 tools (adding `actuate` and
  `put_latest_state`). Auto-discovered `roots` directories are always strictly
  read-only.
- **Stdio channel purity:** `stdout` carries MCP JSON-RPC frames only; all
  diagnostics go to `stderr`.

## Exit codes

| Code | Meaning |
| --- | --- |
| `0` | Normal shutdown (`stdin` closed) or valid `--check-config` |
| `1` | Startup or MCP handshake protocol failure |
| `2` | Invalid CLI arguments or configuration conflict (`SC-01`–`SC-10`) |

## License

MIT. See `LICENSE` in this distribution.

Source and documentation: <https://github.com/fathomadb/fathomdb>
