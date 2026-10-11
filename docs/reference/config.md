# Config

## Process-start SQLite mode (0.8.25)

Before any Engine opens, a process may select one of two SQLite runtime modes:

| Mode | Python | TypeScript | Rust |
| ---- | ------ | ---------- | ---- |
| Performance | `admin.configure_runtime(sqlite_mode="performance")` | `admin.configureRuntime({ sqliteMode: "performance" })` | `admin::configure_runtime(RuntimeSqliteMode::Performance)` |
| Diagnostics | `admin.configure_runtime(sqlite_mode="diagnostics")` | `admin.configureRuntime({ sqliteMode: "diagnostics" })` | `admin::configure_runtime(RuntimeSqliteMode::Diagnostics)` |

Performance is the default selected by the first Engine open. It disables
SQLite global memory statistics and heap-limit enforcement for lower shared
runtime overhead. Diagnostics preserves those facilities for measurement and
troubleshooting. The 0.8.25 runtime also performs bounded prepared-statement
reuse internally; there is no caller tuning knob for its cache.

The choice is process-wide and lasts until restart. Repeating the effective
mode succeeds, including after an Engine opens. A conflicting request raises
`RuntimeConfigurationError` with the requested and effective modes. A first
request made after another user of SQLite has initialized the runtime fails
as too late, with an optional SQLite code. This API does not call
`sqlite3_shutdown()` and is not a database permission or file-access boundary.

## Engine configuration

Engine-owned runtime knobs (0.8.27). The same five knobs are exposed by
every binding in idiomatic spelling (Python snake_case, TS camelCase,
Rust snake_case).

Authoritative spec: [`dev/design/engine.md`](https://github.com/fathomadb/fathomdb/blob/main/dev/design/engine.md);
cross-binding symmetry pinned by `dev/design/bindings.md` § 6.

## Knob matrix

| Knob | Python | TypeScript | Range | Default | Runtime effect |
| --- | --- | --- | --- | --- | --- |
| Projection workers | `scheduler_runtime_threads` | `schedulerRuntimeThreads` | `1..=64` | `2` | Starts that many projection workers and worker-owned SQLite connections; projection admission is `64 × workers`. |
| Embedder workers | `embedder_pool_size` | `embedderPoolSize` | `1..=64` | `5` | Bounds simultaneous provider calls; the waiting queue holds `4 × workers`. With no provider, no embedder worker or queue starts. |
| Embedder deadline | `embedder_call_timeout_ms` | `embedderCallTimeoutMs` | `1..=4,294,967,295` ms | `30,000` ms | One deadline covers queue wait plus provider service on production embedding paths. |
| Provenance retention | `provenance_row_cap` | `provenanceRowCap` | `0..=2^53-1` rows | `1,000,000` rows | Bounds retained provenance rows; `0` disables retention. |
| Slow event threshold | `slow_threshold_ms` | `slowThresholdMs` | `0..=2^53-1` ms | `100` ms | Governs operation and SQLite-statement slow signals; `0` accepts every positive duration. |

`None` (Python) or an omitted TypeScript field selects the listed default.
Python rejects booleans as integer settings; TypeScript requires finite safe
integers. Invalid values fail before database files, locks, workers, or provider
calls are started. Rust owns the effective defaults and final validation;
Python and TypeScript also check malformed or out-of-range input before native
open. The deadline starts before enqueue, covers queue wait and provider
service, and stays fixed for a batch. A full queue or queued expiry reports
`Overloaded` for direct embedding and leaves projection work pending; a
started provider failure or timeout consumes a projection retry. Hybrid search
may fall back to sparse results. Closing cancels pending requests, and a
provider still running after the shared 30-second post-quiescence drain budget
causes `SchedulerError` on explicit close. The requested configuration is frozen at open
and does not change when the slow threshold setter changes the effective value.

## Python — two equivalent forms

Object form:

```python
from fathomdb import Engine, EngineConfig

config = EngineConfig(embedder_pool_size=4, slow_threshold_ms=200)
engine = Engine.open("./mydb.fdb", config=config)
```

Keyword form:

```python
engine = Engine.open(
    "./mydb.fdb",
    embedder_pool_size=4,
    slow_threshold_ms=200,
)
```

The two forms are mutually exclusive within a single `Engine.open`
call. Unknown keyword arguments are rejected with `TypeError`.

## TypeScript

```ts
import { Engine } from "fathomdb";

const engine = await Engine.open("./mydb.fdb", {
  engineConfig: {
    embedderPoolSize: 4,
    slowThresholdMs: 200,
  },
});
```

`engine.config` is a frozen snapshot of the values requested at open. Changing
the object originally passed to `Engine.open` does not change the snapshot, and
the snapshot itself is readonly. It reports the open request rather than live
effective state, so a later `setSlowThresholdMs` call changes profiling behavior
without rewriting `engine.config.slowThresholdMs`.

NAPI uses a separate Tokio `spawn_blocking` handoff for blocking engine calls.
There is no binding handoff-pool sizing option in `EngineOpenOptions`.

## Non-fields

Python executor usage is caller-owned and is not an engine config
field. Path is positional on `Engine.open` and is not a config field. The
process-start SQLite mode above is also intentionally separate from
`EngineConfig` because it applies to the loaded runtime before any connection
exists.

## Mutable-at-runtime

Only `slow_threshold_ms` / `slowThresholdMs` is mutable post-open via
`engine.set_slow_threshold_ms` / `engine.setSlowThresholdMs`. All
other knobs are open-time only. The setter changes the effective threshold,
not the requested-open configuration snapshot.

## See also

- [Python API — Engine.open](python-api.md)
- [TypeScript API — Engine.open](typescript-api.md)
- Authoritative spec: [`dev/design/engine.md`](https://github.com/fathomadb/fathomdb/blob/main/dev/design/engine.md)
