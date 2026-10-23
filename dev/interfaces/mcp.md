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
- Exit codes: 0 normal shutdown: stdin closed, before or after `initialize`;
  2 invalid configuration or arguments; 1 any other startup failure,
  including a handshake protocol error (a first message that is not an
  `initialize` request, for example a notification) and stdout closed during
  the handshake. Unparsable input lines are ignored by the transport, so
  garbage followed by stdin close exits 0. Store open failures never exit the
  process (MR-003.3).

## 2. Configuration (JSON; ADR-0.6.0-operator-config-json-only)

| Key | Type | Default | Rule |
| --- | --- | --- | --- |
| `stores` | object `name → {path, create?, eager?}` | `{}` | Explicit entries. `path` absolute or relative to the config file. `create` (default `false`) permits creating the file; when false the registry checks the file exists before `Engine::open` (which would otherwise create it) and fails `unavailable/missing_file`. `eager` opens at launch. |
| `roots` | array of `{path, suffix?}` | `[]` | `path` (absolute or relative to the config file) must be an existing directory. Every regular file directly in it ending in `suffix` (default `.sqlite`) whose name without the suffix matches the store-name grammar is addressable by that name. No recursion; symlinked files are not addressable. An explicit `stores` name wins over a root name. |
| `default_store` | string or null | null | Session default when a call omits `store`. Must be addressable. |
| `idle_timeout_secs` | integer ≥ 0 | 60 | 0 = never close (MR-005.4). |
| `max_open_stores` | integer | 1 | Must be 1 in this release (MR-005.7); any other value is invalid. |
| `admission_cap` | integer ≥ 1 | 4 | Engine calls in flight per store, including abandoned ones (MR-033.2). Must be < 8. A call that finds every permit held waits for one in arrival (FIFO) order while its deadline allows; it is refused as `busy/admission` only when the line is full or it cannot start in time (§ 6). |
| `queue_limit` | integer ≥ 1 | 32 | Calls that may wait for a permit per store. A call that arrives when no permit is free and `queue_limit` calls already wait is refused at once as `busy/admission`, `busy.reason` `queue_full`. |
| `call_deadline_ms` | integer ≥ 100 | 10000 | Per tool call (MR-033), measured from arrival. It bounds the wait for an opening store and for a permit: a waiting call starts as soon as it gets a permit before this deadline; otherwise it is refused as `busy/admission`, `busy.reason` `deadline`. There is no start check (owner, 2026-10-19). It does not bound a started call: that call runs to completion and returns its result, even after this deadline (`run_limit_ms`). |
| `run_limit_ms` | integer ≥ `call_deadline_ms` | 30000, or `call_deadline_ms` when that is larger (owner, 2026-10-19) | Bound on a started call, measured from its admission permit. It exists to report stuck work: a call still running at the limit returns `timeout/deadline` and is abandoned (its read keeps its permit until it returns, and it is counted in `busy.abandoned`). Engine reads cannot be interrupted, so the limit frees nothing. A call with a progress token gets a progress notification `queued: q waiting, c running` each second while it waits for a permit and `running` each second while it runs, numbered in one increasing sequence. |
| `default_max_tokens` | integer ≥ 256 | 4000 | Budget when a call omits `max_tokens` (MR-041.3). |
| `snippet_bytes` | integer 40..2000 | 300 | Snippet cap in UTF-8 bytes, ellipses included (MR-040). Each search requests the engine's match-centred snippets (EP-17) with `max_bytes` = this value, one window, and a record head of `min(48, snippet_bytes / 2)` bytes. 48 holds a LoCoMo-style header such as `(unknown date) [8 May 2023] Caroline:` (37 bytes). The old key `snippet_chars` is unknown (no alias). |
| `search_mode` | `"text"` or `"hybrid"` | `"text"` | Default search path. `"text"` until the MR-021 evaluation shows hybrid at or above BM25 precision@3. |
| `use_default_embedder` | bool | false | Passed to `OpenOptions`. Required for `"hybrid"`. |
| `telemetry_dir` | path or null | null | Absolute or relative to the config file. When set, a fresh sink `<dir>/<store>.<pid>.<epoch>.jsonl` is enabled on every open and feedback is recorded (MR-024, MR-024.7). |
| `hints` | bool | true | Whether results carry the `hints` sidecar of suggested next calls (§ 3) when a call does not pass its own `hints` argument. Precedence: the call's argument, then this key, then the default. |
| `write` | bool | false | Must be false in this release. |
| `scope` | null | null | Must be null in this release (MR-014.4). |

Store names match `^[A-Za-z0-9][A-Za-z0-9._-]{0,63}$` and must not be `.` or
`..`. Unknown keys are invalid.

## 3. Common call arguments and envelope

Every tool except `list_stores` takes `store` (string, optional: falls back to
`use_store` or `default_store`; error if none).

Every tool, including a continuation call of `search`, takes `hints` (bool,
optional): whether the result carries the `hints` sidecar. It defaults to the
configured `hints` key (§ 2). It is not part of the canonical request, so it
never changes a `response_id` or `continuation` hash.

Every successful result carries `structuredContent` (an object, schema
declared as the tool's `outputSchema`) and one `text` content block that is a
compact rendering of the same data. Each `outputSchema` is
`anyOf[<result>, {error}]` with a root `"type": "object"`, because tool errors
also carry `structuredContent` (§6, MR-071.1). Field order is fixed; maps are sorted by
key; no wall-clock or random values appear except `validity_instant` (MR-043).
In text blocks, record content and every `ref` (hit, item, target and hint
lines) are JSON-quoted, so a writer-chosen id reads as data.

Common envelope fields (present on every store-bound result):

| Field | Type | Meaning |
| --- | --- | --- |
| `store` | string | The store answered. |
| `epoch` | integer | Open epoch of that store; increments on each open. |
| `validity_instant` | integer | Epoch seconds the read was answered at. The adapter resolves now itself when the caller gives no `as_of` and always passes it as `ReadView.valid_as_of`, so this is never null (MR-023.2). It is the only time-derived field (MR-043.2). `orient` and `feedback` take no `as_of`; theirs is the call time. A continuation page reuses the instant of its first page. |
| `state` | `{ "status": "unavailable" }` | MR-050; the state read is an engine follow-up. |
| `degradation` | array of `{arm, reason}` | MR-032. Empty when nothing degraded. |
| `budget` | `{max_tokens, used_tokens, cut}` | `used_tokens` = ceil(bytes ÷ 4) of the serialized structured content (with `budget` itself elided) plus the text block. It is computed before `budget` is inserted, so the value is stable (MR-041). |

### Hints (ADR-0.8.29-agent-hints-sidecar)

`search`, `fetch`, `expand` and `list_stores` results may carry `hints`: an
array of suggested next calls authored by FathomDB, absent when there are none
or hints are off (`hints` argument, else the `hints` config key, else on).
Every string in a hint is a closed-enum word, an adapter handle
(`continuation`), an operator store name or a `ref` from the same response;
a hint never carries record content. Hints are advisory and never replace a
typed field (`snippet_cut`, `continuation`, `completeness`, `degradation`).
Clients must ignore a hint whose `action` or `reason` they do not know; new
reasons, actions and fields are additive. There is no per-response version
field: the vocabulary version is this document's revision, and the output
schemas list the closed enums.

This is the one vocabulary table (MR-072):

| `reason` | `action` | Fields | Tools | When |
| --- | --- | --- | --- | --- |
| `degraded` | `none` | `arm`, `cause` | `search`, `expand` | a degradation entry is present; names the first entry in canonical order |
| `no_results` | `search` | `mode?` | `search` | zero hits on the page; `mode: "hybrid"` only for a text search on a store with a usable embedder the configuration permits and the dense arm on, otherwise rephrase |
| `snippet_cut` | `fetch` | `refs`, `refs_omitted?` | `search` | kept hits with `snippet_cut: true` |
| `body_cut` | `fetch` | `refs`, `refs_omitted?`, `max_tokens` | `fetch`, `expand` | items or targets with `body_cut: true`; `max_tokens` is the whole-response estimate with every body uncut (at least 256) |
| `cut_by_budget` | `search` | `continuation`, `max_tokens` | `search` | hits were dropped to fit `max_tokens` (this call's) and a continuation exists |
| `more_results` | `search` | `continuation` | `search` | the page reached its limit (below 100) and a continuation exists |
| `default_store_set` | `search` | `store` | `list_stores` | the session has a default store |

- Closed values: `action` is `fetch`, `search` or `none`; `mode` is `text` or
  `hybrid`; `arm` is `vector`, `text`, `text_edge`, `graph_arm` or `graph`;
  `cause` is `soft_fallback`, `dense_disabled`, `embedder_unavailable`,
  `embedder_overloaded`, or a graph code (`query_seed_text_fallback`,
  `projection_legacy_unverified`, `projection_processing`,
  `projection_blocked`, `projection_deferred`, `projection_degraded`).
- Bounds: at most one hint per reason, in the table's order. `cut_by_budget`
  and `more_results` exclude each other. A cut-ref hint lists every cut ref
  in result order up to 50; refs beyond 50, and refs longer than 128 bytes
  (left out of hints, still in results), are counted in `refs_omitted`.
- Budget: `hints` counts against `max_tokens`; with hints on, a page may keep
  fewer results. Each response stays internally consistent: its continuation
  resumes at its own offset plus kept count. Hints never cost a search page
  every hit: a page that would keep no hit with hints, or an empty page that
  would not fit, is shaped again without them.
- Text block: one line per hint after the header and before the hits, from a
  fixed template per reason, for example `hint: fetch ["l:a","l:b"]
  (snippet_cut: snippets are centred on the match; fetch to read the full
  record)`. The `snippet_cut` sentence follows the cut hits' `snippet_anchor`:
  "snippets are centred on the match", "snippets are the record start", or
  "snippets are centred on the match or are the record start".
  The paging sentences name the continuation-only call, because a
  continuation call refuses `query`, `limit` and filters: `hint: search
  continuation=c1-… (more_results: the page reached its limit; for more call
  search {"continuation": "<c>"}: pass only the continuation; do not repeat
  query or limit)`. The `cut_by_budget` sentence also names `max_tokens`.

## 4. Tools

### `list_stores`

Args: `hints`. Result: `stores` — array of `{name, open, readiness, default}`
in name order, where `readiness` is one of `closed`, `opening`, `reopening`,
`ready`, `draining`, `failed`. Never includes paths (MR-005.2). `hints`:
`default_store_set` (§ 3).

### `use_store`

Args: `store` (required). Sets the session default. Result: `{store}`.
Opening is not forced; the next store-bound call opens it.

### `orient`

Args: `store`. Result adds:

- `projections`: from `read.projections` — `{name, roles, fts, vector}` (whether
  each is configured) in name order.
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
| `as_of` | integer | now | `ReadView.valid_as_of` (MR-023). |
| `limit` | integer 1..100 | 10 | Ranked cap is 100 (MR-041.2). |
| `max_tokens` | integer ≥ 256 | config | MR-041. |
| `explain` | bool | false | Include per-hit explanation. |
| `highlights` | bool | false | Add per-hit `highlights`: `[start, end)` byte ranges into `snippet` of the matched words. Presentation only: not part of the canonical arguments, so references, `response_id` and continuations are unchanged. |
| `continuation` | string | none | From a prior result. Only `store`, `max_tokens`, `hints` and `highlights` may accompany it; `store` selects the handle table, so a handle from another store is `stale_handle`. |

`include_superseded` and `include_inactive` are not search arguments: the
engine rejects them on every search path (`reject_existence_relaxation_on_search`)
because projection indexes are not version-complete. History is a `fetch`
concern. Unknown arguments are `invalid_request/bad_argument`.

Result adds:

- `response_id`: string `r<epoch>-<hash>`, for `feedback`, where `<hash>` is
  the first 16 hex digits of SHA-256 over the store, the canonical arguments and
  the result references. Identical requests on identical state get identical
  ids (MR-043).
- `results`: array of `{ref, kind, snippet, snippet_cut, snippet_anchor, highlights?, source_id, score, arm, ce_score?}` in engine rank order.
  - `snippet` is at most `snippet_bytes` bytes, chosen by the engine (EP-17).
    It is centred on the matched words, keeps the record head, and uses `…`
    where text is left out.
  - `snippet_anchor` is `whole` (the record fitted), `match` (a window
    centred on the matched words) or `start` (the record start: the hit has
    no lexical match, as for vector-only hits, or it ranked past the
    engine's per-search snippet cap).
  - `snippet_cut` is true when any of the record is left out.
  - The budget loop drops whole hits; it never re-cuts a snippet.
  - Cost: the engine's per-search snippet cap (`max_point_queries`) is set
    to the run window, `offset + limit` (at most 100). A search therefore
    runs at most one combined `highlight()` point query per hit in the run
    window: per long hit, and with `highlights` also per hit that fits.
    Multi-term queries add per-term queries from a second pool of the same
    size, so a search runs at most 2 × the run window point queries. A
    continuation page shows the snippets of an unpaged search over the same
    window. M1 has not yet measured this cost; the engine receipt is
    `dev/experiments/agent-access-snippet-cost/`.
  - The text line for a hit carries `anchor=<snippet_anchor>`.
- `completeness`: one of `complete` (fewer results than `limit`, no
  degradation), `limit_reached` (exactly `limit`; total unknown),
  `cut_by_budget`, `empty`, `empty_not_ready` (empty with a degradation entry),
  or `partial` (fewer results than `limit` while a degradation entry is present,
  so `complete` cannot be claimed, MR-032.3) (MR-030).
- `degradation` (hybrid only): `{arm, reason}` entries from the engine's soft
  fallback (`soft_fallback`) and dense-disabled state (`dense_disabled`), plus
  `{arm: "vector", reason: "embedder_unavailable"}`, which the adapter adds when
  the store's embedding readiness, read once per open epoch, reports no usable
  embedder (the engine reports no soft fallback in that case, MR-032.1), plus
  each entry of the engine's own `degradations` list. Today that list has one
  reason: `{arm: "vector", reason: "embedder_overloaded"}`, reported when this
  search's query embedding was refused because the embed queue was full or it
  expired in that queue. The search still returns its text-arm hits (EP-03).
- `verdict`: `unavailable` in this release (MR-031).
- `continuation`: string `c<epoch>-<hash>` or null, `<hash>` over the store,
  canonical arguments and offset. Present when the page was cut by the budget or
  reached its limit, kept at least one hit, and the next offset is below the
  ranked cap.
- `continuation_exact`: `false`, present only in embedded mode (§5, §7).
- `explanation`: present only when `explain` is true.
- `hints`: `degraded`, `no_results`, `snippet_cut`, then `cut_by_budget` or
  `more_results` (§ 3).

`ref` is the hit's `IdSpace` prefixed form (`l:<logical_id>`, `h:<hash>`,
`p:<n>`). Only `l:` references are accepted by other tools in this release
(MR-042.2).

### `fetch`

Args: `refs` (array of 1..50 strings), `as_of`, `include_superseded`,
`include_inactive`, `max_tokens`. Result adds `items`: one per ref, in request
order, each `{ref, record: {kind, body, body_cut}}` or `{ref, error: {...}}`
(MR-052). A missing record is a per-item `not_found`; a malformed reference is
a per-item `invalid_request/bad_argument`; an `h:` or `p:` reference is a
per-item `invalid_request/unsupported_ref`. Bodies are shortened (`body_cut`)
to fit `max_tokens`. When the items cannot fit even with every body empty
(for example 50 refs at 256 tokens), the call fails as
`invalid_request/bad_argument`, because `fetch` must return one item per ref
and has no continuation. `hints`: `body_cut` (§ 3), which is sized with the
response; if its own bytes keep the items from fitting, the same refusal
applies.

### `expand`

Args: `refs` (1..10 `l:` references), `direction` (`out`, `in`, `both`;
default `out`), `max_depth` (0..3, default 1), `limit` (1..50, default 20),
`edge_kinds`, `target_kinds`, `as_of`, `max_tokens`. Calls `graph.expand` with
an `Explicit` seed, a `Current` read context, `include_explanation: true` (for
hop counts), `result_limit = limit`, and
`max_work_units = 10000`, the engine maximum; `run_limit_ms` bounds the time.
A seed whose expansion examines more edges fails as
`invalid_request/bound_exceeded` (edge and target kinds are filtered after the
edges are counted, so they do not help). The ranges are the engine's own
validation bounds. Result adds `targets` in engine order, each `{ref, kind,
hop_count, terminal_edge_kind, body, body_cut}` (hop count and edge kind from
`GraphOriginV1`), `complete` (the engine's flag, which is always true on
success), `degradation` from the engine's degradation codes (`arm: "graph"`),
and `bound`, the cap that was reached (MR-053.2):

- `"result_limit"`: the target count equals `limit` (more may exist).
- `"budget"`: trailing targets were dropped to fit `max_tokens`, after every
  body was cut.
- `"unspecified"`: the engine reported `complete: false`.
- `null`: no cap was reached.

`hints`: `degraded` (graph codes), then `body_cut` (§ 3).

A non-`l:` seed is `invalid_request/unsupported_ref`; a seed that is not
visible is `not_found`.

### `feedback`

Args: `response_id` (required), `used` (array of refs from that response),
`unused` (optional array). Result: `{status}` where status is `recorded` or
`not_recorded` with `reason` in:

- `response_expired`: no such response in this store's table (unknown or
  dropped from the last 256).
- `store_reopened`: the store was closed or reopened since the response, so
  its sink no longer holds that query (MR-006.5).
- `telemetry_disabled`: no `telemetry_dir`, or the sink could not be enabled
  for the epoch that answered.
- `text_mode`: the response came from the text path, which telemetry never
  captures.

Checks run in that order. Once the response is found, and before the other
reasons, every ref in `used` and `unused` must belong to it
(`invalid_request/bad_argument` otherwise). A response from another store is `invalid_request/wrong_store`
(MR-024.5). `used` may be empty. Recorded feedback reaches `record_feedback`
with label source `agent` (MR-024.1). Feedback never alters later results.
Feedback takes an admission permit but never queues for one: with no free
permit it is refused at once as `busy/admission`, `busy.reason`
`deadline`.

### Not present in this release

Change queries (MR-055), remember/replace/retract (MR-060–062) and scope
(MR-014) have no tools. Their absence is the contract, so `tools/list` is
fixed and identical for every configuration (MR-012.1, MR-006.7).

## 5. Continuations and response ids

Both are short server-side handles, not encoded state. The session keeps the
last 256 of each per store (least recently used dropped; an identical hash
overwrites an identical entry). A handle from another store, another epoch, or
one that was dropped fails as `invalid_request/stale_handle` with remedy
"re-run the search".
A continuation call takes only `store`, `continuation`, `max_tokens` and
`hints`; any other argument is `invalid_request/bad_argument`.

- **Standalone, read-only:** the adapter is the sole opener and nothing writes,
  so a store cannot change within an epoch and a same-epoch continuation is
  exact.
- **Embedded:** the host may write between pages, so a same-epoch continuation
  is best effort and carries `continuation_exact: false`. Standalone results
  omit the field.
- In this release `stale_handle` is `invalid_request`. The `drift` class and
  re-derivation (MR-041.4, MR-051.3) arrive with the engine state read.

## 6. Errors

Tool errors are MCP tool results with `isError: true` and structured content
`{error: {class, code, message, remedy, candidates?, detail?, busy?}}`. `detail` is
the SDK `ErrorKind` class name (for example `DatabaseLockedError`) when the
error came from the SDK. Messages are adapter templates and never quote SDK
message text. JSON-RPC errors are reserved for protocol faults; a panicking
tool handler is a JSON-RPC internal error for that request (MR-002.2).

| `class` | Typical `code`s | Source |
| --- | --- | --- |
| `invalid_request` | `unknown_store`, `bad_store_name`, `bad_argument`, `unsupported_ref`, `stale_handle`, `wrong_store`, `bound_exceeded` | Adapter validation; SDK `InvalidArgument`, `WriteValidation`, `SchemaValidation`, filter errors; `bound_exceeded` is an expansion past 10,000 edges |
| `not_found` | `not_found` | Absent record (per item in `fetch`) |
| `not_ready` | `opening`, `reopening`, `draining`, `embedder_not_ready`, `vector_disabled` | Readiness gate: the code is the phase word. Every call waits for an open or reopen in progress up to its deadline before returning it; calls with a progress token also get progress notifications (MR-003.2, MR-006.4). SDK `EmbedderNotConfigured`, `KindNotVectorIndexed` (`embedder_not_ready`); SDK `VectorEquivalenceMismatch` (`vector_disabled`): a hybrid search on a store whose dense side the open-time equivalence check disabled; remedy: use mode `"text"` |
| `unavailable` | `open_failed`, `database_locked`, `corrupt`, `migration_refused`, `missing_file` | Store open failure (MR-003.3) |
| `timeout` | `deadline`, `cancelled` | `deadline`: the call was still running at `run_limit_ms` after its admission (MR-033.1); it is abandoned and keeps its admission permit until its engine work returns. `cancelled`: the client cancelled the call (`notifications/cancelled`). A waiting call leaves the queue; a running call is abandoned at once and its read runs to its end. rmcp discards the response of a cancelled request, so clients do not see this result |
| `busy` | `admission`, `store_switch` | `admission`: the call could not get an admission permit (MR-033.2); `busy.reason` says why. `store_switch`: another store is open with calls in flight or mid-open, so this one cannot open (MR-005.7); use the open store or retry in a later turn |
| `internal` | `internal`, `panic` | Anything else; message never contains record content (MR-012.2) |

`candidates` is present only for `unknown_store` (from the registry). Kind
candidates wait for the engine kind enumeration.

`busy` is present only for `busy/admission`:

```json
{"error": {"class": "busy", "code": "admission", "message": "…", "remedy": "…",
  "busy": {"reason": "deadline", "in_flight": 4, "queued": 8, "abandoned": 0, "stuck": false}}}
```

| Field | Meaning |
| --- | --- |
| `reason` | `queue_full`: `queue_limit` calls already waited on arrival. `deadline`: no permit came before the wait deadline (§ 2 `call_deadline_ms`). `engine_overloaded`: SDK `Overloaded`. The action is the same for all three: retry in a later turn, or issue fewer calls at once |
| `in_flight` | Admission permits held, including abandoned calls |
| `queued` | Calls waiting for a permit |
| `abandoned` | Calls past their deadline whose engine work still holds a permit |
| `stuck` | `true` when abandoned work holds every permit |

No busy result carries an advised wait (`retry_after_ms`); the object admits
no other field.

## 7. Embedding (Rust) API

The `fathomdb-mcp` library serves the same tools to a Rust host over any rmcp
transport (MR-004). Its public items are the contract below; everything else
is crate-private.

- **`Adapter::new(config: config::Config) -> Result<Adapter, AdapterError>`.**
  Validates `config` again (its fields are public), starts the idle clock and
  begins opening `eager` stores. It must run inside a Tokio runtime. Errors:
  `AdapterError::InvalidConfig(ConfigError)` for any §2 rule, and
  `AdapterError::NoRuntime` outside a runtime. `Adapter` is `Clone`; clones
  share one registry.
- **`Adapter::register_engine(&self, name: &str, engine: Arc<fathomdb_sdk::Engine>) -> Result<(), AdapterError>`.**
  Serves a host-owned open engine under `name`. Errors:
  `AdapterError::BadStoreName` for a name outside the grammar and
  `AdapterError::DuplicateStore` for a name already registered or configured.
  A rejected call leaves the engine untouched. Hosted-engine semantics:
  - The adapter never opens, drains or closes the engine.
  - Its epoch is fixed at 1, it never goes idle, and it is excluded from the
    open-store limit.
  - When `telemetry_dir` is set, a sink `<dir>/<name>.<pid>.1.jsonl` is
    enabled on it, replacing any sink the host enabled.
  - Its search results carry `continuation_exact: false`.
- **`Adapter::connection(&self) -> edge::FathomServer`.** A server handler for
  one MCP connection, with its own session (default store, continuations,
  response ids). Serve it with rmcp's `ServiceExt::serve` on any transport.
- **`Adapter::shutdown(&self)` (async).** Stops admitting calls, then drains
  and closes every adapter-opened store, waiting at most five seconds past
  `call_deadline_ms`; a store whose started or abandoned call is still
  running then is not waited for. Hosted engines stay open.
- **`edge::FathomServer`.** The rmcp `ServerHandler` (tools capability only,
  the static tool list of §4, server info `fathomdb-mcp`). It is `Clone`;
  clones share one session.
- **`AdapterError`.** The four variants above; `Display` names the cause.
- **`config::Config`.** The §2 configuration, with public fields and
  `Default` equal to parsing `{}`.
  - `Config::from_json_str(text, base_dir)` and `Config::load(path)` parse
    strictly and resolve relative paths against `base_dir` or the file's
    directory. Errors are `ConfigError { message }`, naming the key.
  - `Config::validate(&self)` re-checks the value rules.
  - `Config::resolve(&self, name)` returns a `Resolution { path, create, eager }`,
    or `NameError::BadName` (grammar) or `NameError::Unknown`.
  - `Config::addressable_names(&self)` lists every addressable name, sorted.
  - Also public: `StoreEntry`, `Root`, `SearchMode`, `SearchMode::as_str`, and
    `config::NameError`.
- **`seams::Seams`** (feature `test-seams` only) is a test hook, not part of
  the contract.
