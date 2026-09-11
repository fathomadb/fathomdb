---
title: ADR-0.8.25-sqlite-runtime-configuration
date: 2026-09-10
target_release: 0.8.25
desc: Application-owned SQLite runtime selects performance or diagnostics mode before first Engine open.
blast_radius: fathomdb-engine startup; Rust/Python/TypeScript admin APIs; SQLite memory statistics and heap limits
status: accepted (HITL seq-276)
---

# ADR-0.8.25 — SQLite runtime configuration

**Status:** accepted by `seq-276` for Slice 79.

## Decision

FathomDB owns the SQLite runtime used by its application. Before any Engine is
opened, an application may select one of two process-lifetime modes:

- `performance` configures `SQLITE_CONFIG_MEMSTATUS=0`;
- `diagnostics` configures `SQLITE_CONFIG_MEMSTATUS=1`.

The first ordinary Engine open selects `performance` when no explicit choice
was made. Repeating the effective choice succeeds. A conflicting choice or a
choice made after SQLite initialization fails with a typed error. Changing
mode requires a process restart. FathomDB never calls `sqlite3_shutdown()` to
force a reconfiguration.

Rust exposes `fathomdb::admin::configure_runtime`, Python exposes
`admin.configure_runtime`, and TypeScript exposes `admin.configureRuntime`.
Python and TypeScript classify these as runtime controls rather than governed
application commands. The CLI adds no option and uses the default.

## Consequences

Performance mode disables SQLite's global memory-accounting and soft/hard
heap-limit facilities for every user of that loaded SQLite runtime. Diagnostics
mode retains them and may be slower. Neither mode restricts direct database
file access or authenticates connections. Separate accidental-connection
controls are outside this decision.

The configuration lock orders FathomDB configuration and first Engine open; it
cannot serialize arbitrary external SQLite calls. Callers needing diagnostics
must configure it during single-threaded startup.

The bounded reader statement cache selected in Slice 79 is independent of the
mode and does not change SQL, binding, eligibility, or snapshot semantics.

## Rejected alternatives

- A private SQLite packaging redesign is unnecessary under the application-
  owned-runtime decision.
- A compatibility mode or silent fallback would make AC-020 depend on
  initialization order.
- Live mode changes or shutdown/reinitialize would invalidate other SQLite
  users and existing handles.
- Connection permits and ownership tokens are separate controls and are not
  introduced by this runtime API.
