# MCP Server Reference (`fathomdb-mcp`)

> **Documentation target: 0.8.32 (pre-publish, pre-1.0 beta); newest published release: 0.8.31.**

Client-facing reference for the `fathomdb-mcp` stdio server and Rust embedding
library. The normative wire specification lives in `dev/interfaces/mcp.md`.

## CLI launch and exit codes

```text
fathomdb-mcp --config <path.json>
fathomdb-mcp --config <path.json> --check-config
```

| Exit code | Meaning |
| --- | --- |
| `0` | Normal shutdown (`stdin` closed before or after `initialize`) or valid `--check-config` |
| `1` | Startup or MCP handshake protocol failure |
| `2` | Invalid CLI arguments or configuration / startup conflict (`SC-01`–`SC-10`) |

`stdout` carries MCP JSON-RPC frames exclusively; all diagnostics go to
`stderr`. `--check-config` writes a single JSON object (`{"valid": ...}`) to
`stderr`, writes nothing to `stdout`, and never opens a database or creates a
`.lock` file.

## Configuration schema (JSON)

Unknown keys and the removed evaluation key `"write"` are rejected fail-closed.

| Key | Type | Default | Description |
| --- | --- | --- | --- |
| `stores` | object `name -> StoreEntry` | `{}` | Explicit named stores (see below). |
| `roots` | array of `{path, suffix?}` | `[]` | Non-recursive directory discovery (default suffix `.sqlite`); strictly read-only (`SC-05`). |
| `default_store` | string or null | `null` | Session default when a tool call omits `store`; must be addressable (`SC-09`). |
| `allow_writes` | bool | `false` | Global write opt-in guard; cannot pair with `roots` (`SC-05`). Per-store writes still require per-store `allow_writes: true` + writable `scope`. |
| `idle_timeout_secs` | integer >= 0 | `60` | Idle close timeout (`0` disables idle close). |
| `max_open_stores` | integer | `1` | Single open store at a time (`1`); multiple `eager: true` stores fail (`SC-07`). |
| `admission_cap` | integer 1..7 | `4` | Max concurrent engine calls per store. |
| `queue_limit` | integer >= 1 | `32` | Max queued calls waiting for an admission permit per store. |
| `call_deadline_ms` | integer >= 100 | `10000` | Max wait for store open and admission permit. |
| `run_limit_ms` | integer >= `call_deadline_ms` | `max(30000, call_deadline_ms)` | Bound on a started call before returning `timeout/deadline` (`SC-09`). |
| `default_max_tokens` | integer >= 256 | `4000` | Default response token budget. |
| `snippet_bytes` | integer 40..2000 | `300` | Match-centred snippet cap in UTF-8 bytes. |
| `search_mode` | `"text"` or `"hybrid"` | `"text"` | Default search mode; `"hybrid"` requires `use_default_embedder: true` (`SC-09`). |
| `use_default_embedder` | bool | `false` | Open stores with the built-in default embedder (requires `default-embedder` feature). |
| `telemetry_dir` | path or null | `null` | Directory for per-open JSONL telemetry sinks and `feedback` recording. |
| `result_format` | `"text"` or `"structured"` | `"text"` | Compact text (default) or JSON-schema-validated `structuredContent`. |
| `hints` | bool | `true` | Include advisory `hints` sidecar unless overridden per call. |

### `StoreEntry` fields

| Field | Type | Default | Rule |
| --- | --- | --- | --- |
| `path` | string | required | Absolute or relative to the config file. |
| `create` | bool | `false` | Passes `OpenOptions.create_if_missing` (`EP-07`); requires `allow_writes: true` (`SC-03`). |
| `eager` | bool | `false` | Open at launch (at most one when `max_open_stores == 1`, `SC-07`). |
| `allow_writes` | bool | `false` | Enables `actuate` and `put_latest_state` for this store; requires writable `scope` (`SC-01`, `SC-02`). |
| `scope` | string or `{read: string[], write?: string}` or null | `null` | Caller `ScopePolicy`. String shorthand `"alpha"` maps to `read_write(["alpha"], "alpha")` when `allow_writes: true` or `read_only(["alpha"])` when `allow_writes: false`. |
| `allowed_kinds` | string[] or null | `null` | Optional write-only allowlist of canonical node and edge kinds (`SC-03`). |
| `allowed_node_kinds` | string[] or null | `null` | Optional write-only allowlist of canonical node kinds (`SC-03`). |
| `allowed_edge_kinds` | string[] or null | `null` | Optional write-only allowlist of canonical edge kinds (`SC-03`). |
| `allowed_collections` | string[] or null | `null` | Optional write-only allowlist of `latest_state` collections (`SC-03`). |

## Fail-fast startup conflict matrix (`SC-01`–`SC-10`)

| Rule | Rejected condition |
| --- | --- |
| `SC-01` | `allow_writes: true` with `scope` null, omitted, or empty `""` |
| `SC-02` | `allow_writes: true` with a read-only `scope` grant (`write` omitted/null) |
| `SC-03` | `allow_writes: false` with `scope.write`, `allowed_*`, or `create: true` |
| `SC-04` | Legacy evaluation key `"write"` present |
| `SC-05` | `roots` auto-discovery paired with write enablement |
| `SC-06` | Two store names resolving (via canonical path or symlink) to the same database file with incompatible settings, or any duplicate alias where either enables `allow_writes: true` |
| `SC-07` | Multiple `eager: true` stores when `max_open_stores == 1` |
| `SC-08` | Malformed or contradictory `scope` grammar (empty/duplicate tokens, wildcard `"*"`, write scope not in read scopes, control/NUL bytes, bounds exceeded) |
| `SC-09` | Non-addressable `default_store`, `search_mode: "hybrid"` with `use_default_embedder: false`, or `run_limit_ms < call_deadline_ms` |
| `SC-10` | Embedded `Adapter::register_engine[_with_policy]` conflicts with configured `stores[name]` or host `engine.config().scope_policy` (`AdapterError::PolicyConflict`, leaving the host engine untouched) |

## Tool surface

Read-only profiles expose **7 tools**. Write-enabled profiles (at least one
store with `allow_writes: true` + writable `scope`) expose **9 tools**.

| Tool | Profile | Purpose |
| --- | --- | --- |
| `list_stores` | Read + Write | List addressable store names and readiness (never filesystem paths). |
| `use_store` | Read + Write | Set session default store. |
| `orient` | Read + Write | Report projections (keyed by `(entity_class, name)`), projection status, embedding readiness, and `vector_coverage`. |
| `search` | Read + Write | Lexical or hybrid search with closed filters, continuations, explanations, highlights, and optional `freeze` / `frozen_context` exact evidence. |
| `fetch` | Read + Write | Logical-ref retrieval (`l:<id>`) or exact frozen-evidence resolution (`frozen_context` + `evidence_refs` / `graph_evidence_refs`). |
| `expand` | Read + Write | Bounded graph expansion with optional `frozen_context` and `include_evidence_refs`. |
| `feedback` | Read + Write | Record agent relevance labels against a cached hybrid `response_id`. |
| `actuate` | Write-enabled only | Atomic strict-only `ActuationBatchV1` (`PutCanonicalNode`, `PutDerivedNode`, `PutDerivedEdge`, `RegisterSourceDependency`, `TransitionLifecycle`, `PutLatestState`) with `operation_id` replay and CAS `expected_write_boundary`. |
| `put_latest_state` | Write-enabled only | Ergonomic atomic `latest_state` checkpoint wrapper over `ActuationBatchV1` (`PutLatestState`). |

Store-bound responses populate `state` from `read::visibility_state` (`EP-01`):
`database_id`, `generation`, `nonce`, `canonical_write_boundary`, and
`projection_cursor`.

## Error classes

Tool errors return MCP `isError: true` using adapter-owned templates only (never
quoting record bodies, secrets, or raw SQL):

| `class` | Representative `code`s |
| --- | --- |
| `invalid_request` | `unknown_store`, `bad_store_name`, `unknown_argument`, `bad_argument`, `unsupported_ref`, `stale_handle`, `wrong_store`, `bound_exceeded`, `refs_not_in_response`, `writes_disabled`, `kind_not_allowed`, `forbidden_operation`, `scope_violation` |
| `not_found` | `not_found` (also used nondisclosingly for cross-scope read/continuation references) |
| `conflict` | `stale_write_boundary` (optional content-free `observed_write_boundary`), `operation_id_conflict`, `state_drift` (Schema 39 partitioned frozen-context drift, surviving disjoint-kind/cross-scope writes and idle store reopen), `lifecycle_conflict` |
| `not_ready` | `opening`, `reopening`, `draining`, `embedder_not_ready`, `vector_disabled` |
| `unavailable` | `open_failed`, `database_locked`, `corrupt`, `migration_refused`, `missing_file` |
| `timeout` | `deadline`, `cancelled` |
| `busy` | `admission` (`queue_full`, `deadline`, `engine_overloaded`), `store_switch` |
| `internal` | `internal`, `panic` |

## Embedded Rust API (`fathomdb_mcp`)

- `Adapter::new(config: config::Config) -> Result<Adapter, AdapterError>`
- `Adapter::register_engine(&self, name: &str, engine: Arc<fathomdb_sdk::Engine>) -> Result<(), AdapterError>` (read-only default policy)
- `Adapter::register_engine_with_policy(&self, name: &str, engine: Arc<fathomdb_sdk::Engine>, policy: HostedStorePolicy) -> Result<(), AdapterError>`
- `Adapter::connection(&self) -> edge::FathomServer`
- `Adapter::shutdown(&self)` (enforces the blocking drain fence across active blocking SDK closures before closing adapter-opened stores)

## See also

- [Guide — MCP Server](../guides/mcp-server.md)
- [Reference — Rust SDK](rust-sdk.md)
- [Reference — CLI](cli.md)
- [Reference — Errors](errors.md)
