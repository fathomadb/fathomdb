---
title: Slice 79 — admin.configure_runtime design
status: READY
---

# Design — startup SQLite memory-statistics selection

## Scope and code grounding

Implements the direction captured in the [plan](plan.md), not an authentication
boundary. Runtime means one loaded SQLite implementation and its global state;
several Engines/databases may share it. Exclusivity is not proved.

Python `src/python/fathomdb/admin.py` and the PyO3 `admin_configure` wrapper in
`src/rust/crates/fathomdb-py/src/lib.rs` currently submit schema writes through
an existing Engine. Extend the namespace without overloading that operation.
The engine's `init_perf_experiments_runtime` contains a legacy
shutdown/configure/initialize sequence; it is not the production design.

## Public contract

```python
from fathomdb import admin

admin.configure_runtime(sqlite_mode="performance")
# Open Engines afterward using the existing API.
```

Only `performance` and `diagnostics` are accepted (MEMSTATUS 0 and 1).
Rust uses a typed enum; TypeScript follows existing SDK naming conventions.
Thin wrappers call the common Rust owner in their loaded native runtime.
No Engine, database path, raw connection or access token is an input.

Rust exposes `fathomdb::admin::configure_runtime(RuntimeSqliteMode)` returning
`RuntimeConfiguration { sqlite_mode }`. Python exposes the synchronous
`admin.configure_runtime(sqlite_mode=...)`; TypeScript exposes synchronous
`admin.configureRuntime({ sqliteMode })`. The two string values are
`performance` and `diagnostics`. Return objects contain only the effective
mode. Stable errors distinguish `invalid_mode`, `too_late`, `conflict`, and
`sqlite_failure`; conflict includes requested/effective modes and native
failure includes SQLite's numeric result code.

Rust's exact types are `RuntimeSqliteMode::{Performance, Diagnostics}` and
`RuntimeConfigurationError::{TooLate, Conflict { requested, effective },
SqliteFailure { code }}`; default `Engine::open` maps the latter into
`EngineOpenError::RuntimeConfiguration`. Python returns frozen
`RuntimeConfiguration(sqlite_mode: Literal[...])` and raises
`RuntimeConfigurationError` with `reason`, `requested_mode`,
`effective_mode`, and `sqlite_code` attributes. Invalid Python strings are
`ValueError` before native mutation. TypeScript returns
`RuntimeConfiguration { sqliteMode }`; native failures use
`FDB_RUNTIME_CONFIGURATION` with data fields `reason`, `requestedMode`,
`effectiveMode`, and `sqliteCode`, mapped to `RuntimeConfigurationError`.
Invalid TypeScript strings are `RangeError` before native mutation.

The operation is classified as a runtime control, not a governed application
command. `governed-surface-allowlist.json` adds exactly
`runtime_controls: ["admin.configure_runtime", "admin.configureRuntime"]`;
both SDK surface tests enumerate every public `admin` member, subtract this set
from application commands, and assert the set is live. The CLI adds no flag and
therefore uses the performance default. Applications that need diagnostics call
the API before opening any Engine.

## Initialization state and lifecycle

Use one small synchronized native state owner:

| State/request | Outcome |
| --- | --- |
| Unconfigured, valid startup request | Apply MEMSTATUS, initialize SQLite, record successful mode |
| Configured, identical mode | Return existing configuration without reconfiguring SQLite |
| Configured, different mode | Conflict error; restart required |
| Unconfigured, SQLite already initialized | Too-late error; no fallback or shutdown |
| Invalid input | Validation error without runtime mutation |
| Configuration/initialization failure | Preserve failure; do not report readiness |

Serialize FathomDB configuration and first-use entry points, not queries. Keep
SQLite threading/mutex behavior unchanged. No caller registry, per-connection
permit or query-time check. Multiple correct Engine opens remain allowed.

SQLite configuration is not thread-safe: the application must call at startup
before other threads use that runtime. The internal lock coordinates FathomDB,
not arbitrary co-tenant C calls. Check configuration/initialization return codes;
never force `sqlite3_shutdown()` to rescue a late call. Closing all Engines does
not reset the mode. Restart is the supported mode change.
See [SQLite configuration](https://www.sqlite.org/c3ref/config.html) and
[MEMSTATUS options](https://www.sqlite.org/c3ref/c_config_covering_index_scan.html).

## Sealed default and entry points

`Engine::open` without a prior explicit call selects `performance` through the
same state machine. The ordinary Rust, Python, Node and CLI path therefore
continues to satisfy the unqualified AC-020 contract. Applications needing
SQLite memory accounting call `diagnostics` before any Engine open. A prior
arbitrary host SQLite call is outside the lock; default open or an explicit
call then receives `SQLITE_MISUSE` and maps to `too_late`. Standalone schema
APIs operating on a caller connection remain unchanged and do not carry the
Engine performance guarantee.

## Existing initialization and experiment paths

One process-global `Mutex<RuntimeState>` owns explicit configuration and every
first Engine open. `Unconfigured`, `Configured(mode)`, and `Failed(code)` are
terminal transitions. `configure_runtime` calls
`sqlite3_config` and then `sqlite3_initialize` while holding the lock, recording
success only after both return `SQLITE_OK`. `Engine::open` configures
`performance` when still unconfigured.
The sqlite-vec auto-extension registration follows this transition. Identical
calls return the existing configuration; conflicts and late calls do no work.
Failures are sticky. No shipping code calls `sqlite3_shutdown`.

Remove the conflicting global branch of `init_perf_experiments_runtime`; do not
port PAGECACHE/PCACHE2 experiments. Historical receipts remain evidence rather
than hidden overrides of the admin choice. Do not substitute process-wide build
flags for the production state machine.

## Deliberate tradeoff and limitations

Performance mode disables the affected SQLite global accounting and soft/hard
heap-limit interfaces for all users of that runtime. Diagnostics enables those
facilities at restart, not retroactively. It can reduce read performance.
Neither mode promises whole-process memory accounting or a process memory cap.
Configuration success does not prove there are no additional runtime consumers.

No restriction blocks direct database-file access or ordinary Engine connections.
This does not repair SQLite's existing
[multiple-copy/file-locking hazards](https://www.sqlite.org/howtocorrupt.html).
No encryption, ownership marker, external-open rejection test or sharing probe
is required. Document limitations without introducing a guard framework.

## Focused test design

- Port only the reviewed `9e913517`/`b432d24d` statement-reuse product behavior:
  each reader owns a ten-entry rusqlite cache for its lifetime and the exact
  search/dependency statement sites use `prepare_cached`. Both modes use the
  same SQL/bindings and cache. Profiler, census and experiment features are not
  ported. REDs cover alternating bindings, row release/error recovery,
  automatic schema reprepare and focused concurrent DDL/reprepare behavior.
- The same uninstrumented binary runs seven fresh processes with statistics on
  and seven with statistics off in the plan's sealed counterbalanced order.
  Only admin startup mode differs; retain each arm's real AC-020 outcomes and
  compare absolute sequential/concurrent times as well as scaling ratios.
- Each global-state case launches a fresh child process. No database mocks or
  shutdown-to-reset test isolation.
- Diagnostics: controlled live SQLite allocation increases memory accounting;
  freeing it restores the counter; a bounded hard-limit test rejects an
  over-limit allocation. Performance: behaviorally witness intended accounting
  unavailability. Compile flags alone are insufficient.
- Cover identical calls before/after opens, differing-mode conflicts, invalid
  input without mutation, multiple Engines/databases, close/reopen and deliberate
  prior SQLite initialization returning the too-late error.
- Cover concurrent FathomDB first use/configuration. Native initialization
  failure has no stable test seam and is reviewed structurally rather than
  supported by a synthetic failure hook.
  Correct opens/searches/closes must work in both modes.
- Exercise Rust and fresh installed Python/Node admin operations and equivalent
  errors. Native addon proof must not be labeled proof of Rust isolation.
- Preserve exact source/binary digests, feature flags, SQLite build, mode and
  executor. Keep witness allocations and profiling outside AC-020 timing.

The plan owns the matched AC-020, AC-072 and 71B campaigns. Update public interfaces, ADR,
changelog and affected governed-surface checks with required approval. Record
changed artifacts and defaults for Slice 85; no broad matrix in this slice.

## Deferred complexity

Additional logging, heap limits, page-cache provisioning, connection timeouts
and statement-cache sizing remain in [ROADMAP.md](../../../../../ROADMAP.md).
No compatibility mode, ownership token, opening guard, fork, live toggle,
forced shutdown or tuning sweep. Separate accidental-connection controls are
external to this slice. If the chosen candidate fails AC-020, report
the result for consultation rather than weakening the oracle.
