---
title: Slice 79 — admin.configure_runtime design
status: DRAFT
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

## API proposal

```python
from fathomdb import admin

admin.configure_runtime(sqlite_mode="performance")
# Open Engines afterward using the existing API.
```

Only `performance` and `diagnostics` are accepted (MEMSTATUS 0 and 1).
Rust uses a typed enum; TypeScript follows existing SDK naming conventions.
Thin wrappers call the common Rust owner in their loaded native runtime.
No Engine, database path, raw connection or access token is an input.

Proposed return: a small effective-configuration value containing the selected
mode, not a database WriteReceipt/cursor. It reports successful configuration,
not ongoing attestation against arbitrary external C calls. Exact return and
error shapes require interface review.

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

## Small choices to settle before READY

1. **Open without explicit configuration.** Proposed default: select performance
   through the same one-time initialization path, preserving normal SDK open
   ergonomics. Alternative: require an explicit admin call. The owner selected
   the API and modes, not this default; approve it before implementation.
2. **CLI and other entry points.** Define minimal CLI diagnostics selection and
   audit module imports, embedder startup, sqlite-vec registration, migrations
   and standalone schema use. Do not change the public
   `migrate(&rusqlite::Connection)` contract to pretend an existing connection
   can configure startup settings. Standalone schema use need not carry the
   Engine performance guarantee.
3. **Result/error mapping.** Fit existing conventions for invalid mode, conflict,
   too-late initialization and native failure. Keep SDK semantics equivalent
   without inventing a large error framework.

## Existing initialization and experiment paths

Inventory rusqlite first-use behavior, `init_perf_experiments_runtime` callers
and compile-time MEMSTATUS overrides. One production path owns the setting.
Remove or disable conflicting experimental shutdown/reconfiguration in shipping
flows; preserve historical receipts, not hidden overrides of the admin choice.
Do not substitute process-wide build flags for the explicit startup selection.

Do not port PAGECACHE/PCACHE2 experiments. If separating legacy hooks demands a
broader change, present the precise conflict for review instead of widening scope.

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

- Port the reviewed Slice 76 statement-reuse approach and semantic tests into
  the candidate. Both MEMSTATUS modes use exactly the same statement sites,
  SQL/bindings and bounded cache capacity. Statement reuse is not an optional
  arm, and no additional cache-sizing sweep is included.
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
- Cover concurrent FathomDB first use/configuration and initialization failure.
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
forced shutdown or tuning sweep. If the chosen candidate fails AC-020, report
the result for consultation rather than weakening the oracle.
