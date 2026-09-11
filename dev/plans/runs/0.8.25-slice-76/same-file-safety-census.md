---
title: Slice 76 same-file safety census
status: COMPLETE_WITH_SLICE80_INPUT
---

# Same-file safety census

This is a contract and code census, not a new concurrency experiment.

| Route | Overlapping same-file lifetime | Current contract/evidence |
| --- | --- | --- |
| Rust `Engine` plus another FathomDB `Engine` | Forbidden | The canonical sidecar lock is acquired before SQLite I/O; a second open fails `DatabaseLocked`. |
| Python and Node FathomDB bindings, same or different process | Forbidden | Both construct the Rust engine and inherit the same canonical-path sidecar lock. Cross-binding identity does not bypass it. |
| CLI plus any SDK engine | Forbidden | CLI also uses the engine open path and sidecar lock. |
| Raw read-only SQLite plus a live engine | Used by internal tests/evaluation tooling | `mode=ro`, never `immutable=1`, deliberately reads committed WAL frames while the engine remains open. Raw SQLite ignores the FathomDB sidecar. |
| Raw writable SQLite or direct migration plus a live engine | Unsupported | The governed Rust/Python/TypeScript surfaces expose no raw SQL. `fathomdb-schema::migrate` is an internal leaf API whose module contract directs applications to the facade; no sidecar is acquired by a caller-owned `rusqlite::Connection`. |
| Migration followed by engine reopen | Supported on the engine open path | Engine owns migration under the sidecar before normal operation. Slice 85 owns the populated schema-26 upgrade/reopen witnesses. |
| Different database files in one process | Supported | Canonical sidecar locks are per path; simultaneous bindings may therefore operate on distinct files. |

Platform locking is the accepted contract: per-open-file-description `flock`
on Unix and `LockFileEx` on Windows. WAL uses normal locking and a shared-memory
wal-index; the sidecar, not SQLite exclusive locking, enforces one engine per
file. Network/VFS behavior beyond that accepted contract is not newly proved by
Slice 76.

## Slice 80 input

An isolated SQLite design must preserve two separate properties:

1. The FathomDB sidecar must continue excluding every FathomDB engine route,
   independent of which private runtime a binding contains.
2. Existing same-file raw read-only WAL observers must either remain compatible
   across the selected SQLite builds/VFS implementations or be explicitly
   narrowed by an approved contract change.

This census finds no supported overlapping raw writer. It does find intentional
overlapping raw readers, so ELF-local symbol isolation alone is not a complete
same-file safety argument for a future private-runtime design. Slice 80 must
consult on this boundary before selecting packaging or ABI changes.
