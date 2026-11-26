# MCP Server (`fathomdb-mcp`)

> **Documentation target: 0.8.32 (pre-publish, pre-1.0 beta); newest published release: 0.8.31.**

`fathomdb-mcp` exposes FathomDB databases to Model Context Protocol (MCP)
clients over stdio (or embedded in a Rust host via `fathomdb_mcp::Adapter`). It
is built exclusively over `fathomdb-sdk`, opens stores **read-only by default**,
and supports per-store opt-in governed writes bound to an explicit caller
`scope`.

## Default posture: single-scope databases and read-only MCP

- **Database default:** A FathomDB database is single-scope / single-agent by
  default (`scope = ""`). Single-agent SDK applications do not need to
  configure `ScopePolicy`.
- **MCP server default (`allow_writes: false`):** Every configured store and
  every auto-discovered `roots` directory is read-only unless explicitly opted
  in. Read-only MCP profiles advertise the exact `7`-tool read-only surface
  (`list_stores`, `use_store`, `orient`, `search`, `fetch`, `expand`,
  `feedback`) and refuse direct calls to write tools with
  `invalid_request/writes_disabled` and zero database mutation.

## Installation channels (targeted in 0.8.32, pre-publish)

`fathomdb-mcp` is packaged across three synchronized channels for the `0.8.32`
release line (npm wrapper packaging is deferred):

1. **GitHub Releases (5 native targets, includes `--features default-embedder`):**
   Prebuilt stdio binary archives
   `fathomdb-mcp-<version>-<target>.{tar.gz,zip}` plus `.sha256` checksums for
   Linux x86_64/AArch64 (`manylinux_2_28`), macOS x64/arm64, and Windows x64.
2. **PyPI binary wheel (`fathomdb[mcp]`, includes `--features default-embedder`):**
   Lockstep binary wheel (`fathomdb-mcp`) across the same five targets, leaving
   the base `fathomdb` Python SDK wheel free of console scripts.
3. **`crates.io` (`fathomdb-mcp`, `default = []`):**
   Source builds default to lexical-only (`default = []`); pass
   `--features default-embedder` for built-in CPU hybrid search or
   `--features embed-cuda` on CUDA hosts.

## Quickstart: read-only server

Create a JSON configuration file (`mcp-config.json`):

```json
{
  "stores": {
    "notes": {
      "path": "./notes.sqlite"
    }
  },
  "default_store": "notes",
  "search_mode": "text"
}
```

Validate the configuration without opening any database or creating a `.lock`
file:

```bash
fathomdb-mcp --config ./mcp-config.json --check-config
```

`--check-config` prints a JSON validity report to `stderr`, leaves `stdout`
empty, and exits `0` (valid) or `2` (invalid).

Launch the server over stdio:

```bash
fathomdb-mcp --config ./mcp-config.json
```

`stdout` carries MCP JSON-RPC frames exclusively; all logs go to `stderr`
(configured via `FATHOMDB_MCP_LOG=error|warn|info|debug|trace`, default `warn`).

## Bounded exact frozen evidence

Read-only profiles support exact, revision-aware source evidence without adding
extra tools:

1. Call `search` with `"freeze": true` to mint a caller-scoped frozen read
   context and receive a `frozen_context` handle plus per-hit `evidence_ref`s.
2. Optionally pass `"frozen_context"` and `"include_evidence_refs": true` to
   `expand` to obtain `graph_evidence_ref`s.
3. Call `fetch` with `"frozen_context"` and `"evidence_refs"` (or
   `"graph_evidence_refs"`) to resolve exact UTF-8 bytes,
   `artifact_revision_id`, caller-scope filtering, and typed `state_drift`,
   `erased`, `truncated`, or `unavailable` outcomes matching `fathomdb-sdk`.

Frozen contexts in `fathomdb-mcp` use Schema 39 partitioned visibility
validation directly rather than comparing global visibility counters or
expiring handles on idle store reopen. Disjoint-`kind` writes,
`append_only_log` / `latest_state` updates, and mutations (including
transitions, purges, and source erasures) in other caller scopes leave active
`frozen_context` handles valid across idle close/reopen cycles. When an
affecting mutation does drift the frozen context—including edge drift during
`graph_evidence_refs` resolution—`fathomdb-mcp` fails closed with
`conflict/state_drift` so the caller can remint with `search` (`freeze: true`).

## Enabling per-store scoped writes

To allow governed writes on a specific store, opt in explicitly with both
`allow_writes: true` and a non-empty writable caller `scope`:

```json
{
  "stores": {
    "agent-a": {
      "path": "./shared.sqlite",
      "create": true,
      "allow_writes": true,
      "scope": "agent-a",
      "allowed_node_kinds": ["note", "decision"],
      "allowed_collections": ["checkpoints"]
    }
  },
  "default_store": "agent-a"
}
```

When at least one store enables `allow_writes: true`, `tools/list` exposes `9`
tools by adding `actuate` and `put_latest_state`:

- Every mutating call requires a caller `operation_id` for idempotent replay
  within `(store, scope, operation_id)`. Reusing an `operation_id` with a
  different payload fails atomically with `conflict/operation_id_conflict`.
- Pass `expected_write_boundary` (from the live `state.canonical_write_boundary`
  envelope field) for compare-and-swap preconditions (`conflict/stale_write_boundary`
  on concurrent drift).
- Writes are strictly atomic and fail-closed: bulk/non-strict flags and operator
  commands (`recover`, `restore`, `repair`, `fix`, `rebuild`, `purge`,
  `erase_source`, `migrate`, `safe_export`, raw SQL) are never exposed and are
  always refused.
- Auto-discovered `roots` directories remain strictly read-only (`SC-05`).
- Conflicting or legacy configurations (`SC-01`–`SC-10`, including the removed
  evaluation key `"write"`) fail fast at startup before touching disk.

## Embedding in a Rust host

Rust applications already holding an open `fathomdb_sdk::Engine` can embed the
same MCP server without a second database opener using `fathomdb_mcp::Adapter`,
`HostedStorePolicy`, and `Adapter::register_engine_with_policy`. See the
[MCP reference](../reference/mcp.md) for the full configuration schema, tool
contracts, startup conflict matrix, and embedded API.

## See also

- [Reference — MCP Server](../reference/mcp.md)
- [Guide — Frozen Evidence](frozen-evidence.md)
- [Operations — Concurrency and Durability Limits](../operations/index.md#concurrency-and-durability-limits)
- [Positions — Recovery Surface](../positions/recovery-surface.md)
