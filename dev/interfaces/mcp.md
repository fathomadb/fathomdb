---
title: MCP Server Interface (fathomdb-mcp)
date: 2026-10-17
target_release: unscheduled (evaluation build)
desc: Tool contract, launch configuration, result envelope and error model for the fathomdb-mcp server, first release (read-only, adapter-only).
blast_radius: src/rust/crates/fathomdb-mcp/**; dev/design/mcp-adapter/*.md
status: proposed
---

# MCP Server Interface (`fathomdb-mcp`)

Contract for the first, adapter-only release of the FathomDB MCP server. It
uses `fathomdb-sdk` 0.8.27 as is; nothing here needs an engine change. Semantics
are owned by `dev/design/mcp-adapter/requirements.md` (MR-###); this file owns
names, shapes, defaults and codes. The server is read-only and unscoped in this
release.

## 1. Launch

```text
fathomdb-mcp --config <path.json>
fathomdb-mcp --config <path.json> --check-config
```

- Transport is stdio only. stdout carries MCP JSON-RPC and nothing else;
  diagnostics go to stderr (MR-002).
- `--check-config` validates the file, prints a JSON report to stderr, and
  exits 0 (valid) or 2 (invalid) without opening any store.
- Exit codes: 0 normal shutdown (stdin closed); 2 invalid configuration or
  arguments; 1 any other startup failure. Store open failures never exit the
  process (MR-003.3).

## 2. Configuration (JSON; ADR-0.6.0-operator-config-json-only)

| Key | Type | Default | Rule |
| --- | --- | --- | --- |
| `stores` | object `name → {path, create?, eager?}` | `{}` | Explicit entries. `path` absolute or relative to the config file. `create` (default `false`) permits creating the file. `eager` opens at launch. |
| `roots` | array of `{path, suffix?}` | `[]` | Every regular file directly in `path` ending in `suffix` (default `.sqlite`) is addressable by its name without the suffix. No recursion. An explicit `stores` name wins over a root name. |
| `default_store` | string or null | null | Session default when a call omits `store`. Must be addressable. |
| `idle_timeout_secs` | integer ≥ 0 | 60 | 0 = never close (MR-005.4). |
| `max_open_stores` | integer | 1 | Must be 1 in this release (MR-005.7); any other value is invalid. |
| `admission_cap` | integer ≥ 1 | 4 | Engine calls in flight per store, including abandoned ones (MR-033.2). Must be < 8. |
| `call_deadline_ms` | integer ≥ 100 | 10000 | Per tool call (MR-033). |
| `default_max_tokens` | integer ≥ 256 | 4000 | Budget when a call omits `max_tokens` (MR-041.3). |
| `snippet_chars` | integer 40..2000 | 300 | Snippet cap (MR-040). |
| `search_mode` | `"text"` or `"hybrid"` | `"text"` | Default search path. `"text"` until the MR-021 evaluation shows hybrid at or above BM25 precision@3. |
| `use_default_embedder` | bool | false | Passed to `OpenOptions`. Required for `"hybrid"`. |
| `telemetry_dir` | path or null | null | When set, feedback is recorded (MR-024). |
| `write` | bool | false | Must be false in this release. |
| `scope` | null | null | Must be null in this release (MR-014.4). |

Store names match `^[A-Za-z0-9][A-Za-z0-9._-]{0,63}$` and must not be `.` or
`..`. Unknown keys are invalid.

## 3. Common call arguments and envelope

Every tool except `list_stores` takes `store` (string, optional: falls back to
`use_store` or `default_store`; error if none).

Every successful result carries `structuredContent` (an object, schema
declared as the tool's `outputSchema`) and one `text` content block that is a
compact rendering of the same data. Field order is fixed; maps are sorted by
key; no wall-clock or random values appear except `validity_instant` (MR-043).

Common envelope fields (present on every store-bound result):

| Field | Type | Meaning |
| --- | --- | --- |
| `store` | string | The store answered. |
| `epoch` | integer | Open epoch of that store; increments on each open. |
| `validity_instant` | integer or null | Epoch seconds the read was answered at; null when not pinned and resolved by the engine as now. |
| `state` | `{ "status": "unavailable" }` | MR-050; the state read is an engine follow-up. |
| `degradation` | array of `{arm, reason}` | MR-032. Empty when nothing degraded. |
| `budget` | `{max_tokens, used_tokens, cut}` | `used_tokens` = ceil(bytes ÷ 4) of the serialized structured content plus text block (MR-041). |

## 4. Tools

### `list_stores`

No arguments. Result: `stores` — array of `{name, open, readiness, default}`
in name order, where `readiness` is one of `closed`, `opening`, `reopening`,
`ready`, `draining`, `failed`. Never includes paths (MR-005.2).

### `use_store`

Args: `store` (required). Sets the session default. Result: `{store}`.
Opening is not forced; the next store-bound call opens it.

### `orient`

Args: `store`. Result adds:

- `projections`: from `read.projections` (name, kind, type) in name order.
- `projection_status`, `embedding_readiness`: summarized from the SDK.
- `dense_disabled`: `{disabled, reason}`.
- `kinds`, `counts`: `{ "status": "unavailable" }` (MR-022 **[engine]**).

### `search`

| Arg | Type | Default | Notes |
| --- | --- | --- | --- |
| `query` | string | required | Non-empty. |
| `mode` | `"text"` or `"hybrid"` | config `search_mode` | `text` → `search_text_only`; `hybrid` → `search`. |
| `kind`, `source_type`, `status` | string | none | `SearchFilter` fields; `hybrid` only, `text` rejects them. |
| `created_after` | integer | none | Epoch seconds; `hybrid` only. |
| `attributes` | object string→string | none | Declared exact projections; `hybrid` only. |
| `as_of` | integer | none | `ReadView.valid_as_of` (MR-023). |
| `include_superseded`, `include_inactive` | bool | false | `ReadView`. |
| `limit` | integer 1..100 | 10 | Ranked cap is 100 (MR-041.2). |
| `max_tokens` | integer ≥ 256 | config | MR-041. |
| `explain` | bool | false | Include per-hit explanation. |
| `continuation` | string | none | From a prior result; all other args except `max_tokens` must be omitted. |

Result adds:

- `response_id`: string `r<epoch>-<n>`, for `feedback`.
- `results`: array of `{ref, kind, snippet, snippet_cut, source_id, score, arm, ce_score?}` in engine rank order.
- `completeness`: one of `complete` (fewer results than `limit`, no degradation),
  `limit_reached` (exactly `limit`; total unknown), `cut_by_budget`, `empty`,
  `empty_not_ready` (MR-030).
- `verdict`: `unavailable` in this release (MR-031).
- `continuation`: string `c<epoch>-<n>` or null.
- `explanation`: present only when `explain` is true.

`ref` is the hit's `IdSpace` prefixed form (`l:<logical_id>`, `h:<hash>`,
`p:<n>`). Only `l:` references are accepted by other tools in this release
(MR-042.2).

### `fetch`

Args: `refs` (array of 1..50 strings), `as_of`, `include_superseded`,
`include_inactive`, `max_tokens`. Result adds `items`: one per ref, in request
order, each `{ref, record: {kind, body, body_cut}}` or `{ref, error: {...}}`
(MR-052). A missing record is a per-item `not_found`.

### `expand`

Args: `refs` (1..10 `l:` references), `direction` (`out`, `in`, `both`;
default `out`), `max_depth` (0..3, default 1), `limit` (1..50, default 20),
`edge_kinds`, `target_kinds`, `as_of`, `max_tokens`. Calls `graph.expand` with
an `Explicit` seed, a `Current` read context, `result_limit = limit`, and
`max_work_units = 1000` (the engine accepts 1..10,000; the adapter fixes 1,000
so one call stays well inside the deadline). The ranges are the engine's own
validation bounds. Result adds `targets`
(reference, kind, depth, via edge kind, in engine order), `complete` (the
engine's flag) and `degradation` from the engine's degradation codes. When
`complete` is false the result says the bound was reached (MR-053).

### `feedback`

Args: `response_id` (required), `used` (array of refs from that response),
`unused` (optional array). Result: `{status}` where status is `recorded` or
`not_recorded` with `reason` in `telemetry_disabled`, `store_reopened`,
`response_expired` (MR-024, MR-006.5). Never alters later results.

### Not present in this release

Change queries (MR-055), remember/replace/retract (MR-060–062) and scope
(MR-014) have no tools. Their absence is the contract, so `tools/list` is
fixed and identical for every configuration (MR-012.1, MR-006.7).

## 5. Continuations and response ids

Both are short server-side handles, not encoded state. The session keeps the
last 256 of each per store (least recently used dropped). A handle from another
store, another epoch, or one that was dropped fails as `invalid_request` with
remedy "re-run the search". A continuation on an unchanged state continues
exactly; on a store whose epoch changed it fails (state identity is
`unavailable`, so re-derivation per MR-041.4 waits for the engine state read).

## 6. Errors

Tool errors are MCP tool results with `isError: true` and structured content
`{error: {class, code, message, remedy, candidates?}}`. JSON-RPC errors are
reserved for protocol faults.

| `class` | Typical `code`s | Source |
| --- | --- | --- |
| `invalid_request` | `unknown_store`, `bad_store_name`, `bad_argument`, `unsupported_ref`, `stale_handle`, `wrong_store` | Adapter validation; SDK `InvalidArgument`, `WriteValidation`, `SchemaValidation`, filter errors |
| `not_found` | `not_found` | Absent record (per item in `fetch`) |
| `not_ready` | `opening`, `reopening`, `draining`, `embedder_not_ready` | Readiness gate; SDK `EmbedderNotConfigured`, `KindNotVectorIndexed` |
| `unavailable` | `open_failed`, `database_locked`, `corrupt`, `migration_refused` | Store open failure (MR-003.3) |
| `timeout` | `deadline` | Call deadline (MR-033.1) |
| `busy` | `admission` | Admission cap (MR-033.2) |
| `internal` | `internal`, `panic` | Anything else; message never contains record content (MR-012.2) |

`candidates` is present only for `unknown_store` (from the registry). Kind
candidates wait for the engine kind enumeration.
