# fathomdb-mcp

Model Context Protocol (MCP) server for **FathomDB**, built over `fathomdb-sdk`
and served over stdio (standalone binary) or embedded as a Rust library via
`Adapter`.

## Security posture

- **Default read-only (`allow_writes: false`):** Read-only profiles expose
  exactly 7 tools (`list_stores`, `use_store`, `orient`, `search`, `fetch`,
  `expand`, `feedback`) including bounded exact frozen-evidence retrieval.
  Direct invocation of write tools against a read-only store fails with
  `invalid_request/writes_disabled` and zero database mutation.
- **Per-store write opt-in with bound caller `scope`:** Enabling writes on a
  store requires both `allow_writes: true` and an explicit non-empty writable
  caller `scope` policy. Write-enabled profiles expose 9 tools (adding
  `actuate` and `put_latest_state`), optionally configure per-store WAL
  durability (`sync_mode: "normal" | "full"`), enforce strict-only atomic writes
  with idempotent `operation_id` replay and CAS `expected_write_boundary`
  preconditions, and never expose operator recovery or raw SQL escape hatches.
- **Fail-fast startup validation:** Conflicting or legacy configurations
  (`SC-01`–`SC-10`) are rejected before opening any database or lock file.
  Auto-discovered `roots` directories are always strictly read-only.

## Install

`fathomdb-mcp` is distributed across three synchronized channels:

- **crates.io (source build, `default = []`):**

  ```bash
  cargo install fathomdb-mcp --locked
  # Or with the built-in CPU default embedder for hybrid search:
  cargo install fathomdb-mcp --features default-embedder --locked
  ```

- **PyPI binary wheel (includes `--features default-embedder`):**

  ```bash
  pip install 'fathomdb[mcp]'
  # Or standalone:
  pip install fathomdb-mcp
  ```

- **GitHub Releases (5 native targets, includes `--features default-embedder`):**
  Download the `fathomdb-mcp-<version>-<target>.{tar.gz,zip}` archive and
  matching `.sha256` checksum from GitHub Releases.

## Launch

Validate a configuration without opening any store:

```bash
fathomdb-mcp --config ./mcp-config.json --check-config
```

Serve over stdio:

```bash
fathomdb-mcp --config ./mcp-config.json
```

`stdout` carries MCP JSON-RPC frames exclusively; all diagnostics go to
`stderr` (controlled by `FATHOMDB_MCP_LOG`, default `warn`).

## Cargo features

| Feature | Effect |
| --- | --- |
| `default` (`[]`) | Minimal source build without model weights; supports `search_mode: "text"`. |
| `default-embedder` | Enables the built-in `bge-small-en-v1.5` CPU embedder for `search_mode: "hybrid"`. Prebuilt PyPI wheels and GitHub Release binaries enable this feature. |
| `embed-cuda` | Enables CUDA acceleration for `default-embedder` (`FATHOMDB_EMBED_DEVICE=cuda:N`). |

## License

MIT. See the `LICENSE` file shipped in this crate.

Source and documentation: <https://github.com/fathomadb/fathomdb>
