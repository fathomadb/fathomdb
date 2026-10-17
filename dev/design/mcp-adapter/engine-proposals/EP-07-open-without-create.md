---
title: EP-07 — Open an existing database without creating it
date: 2026-10-17
status: PROPOSED
desc: Draft engine proposal. An OpenOptions flag that makes Engine::open fail with a typed not-found error, checked under the sidecar lock, instead of bootstrapping a fresh database at a missing path.
---

# EP-07 — Open an existing database without creating it

Status: Draft. Priority P1. Not an accepted decision.

## Problem

- `Engine::open` documents "Open (creating if absent)"
  (`fathomdb-sdk/src/engine.rs:45`). `OpenOptions` has only `config` and
  `use_default_embedder` (`fathomdb-sdk/src/options.rs:13-18`).
- The engine treats a missing path as a "legal fresh-path bootstrap"
  (`fathomdb-engine/src/open.rs:381-384`). It canonicalizes the parent
  (`open.rs:397-410`), takes the sidecar lock with `create_new`
  (`open.rs:436-466`), and then admits the database. A missing or zero-length
  file passes admission (`open.rs:338-350`) and is created by migration.

Adapter workaround removed: before `Engine::open`, the registry checks
`path.try_exists()` itself, and returns `unavailable/missing_file`
(`src/rust/crates/fathomdb-mcp/src/registry.rs:661-668`). The comment there
accepts the check-then-open race. Between the check and the open, the file can
be removed, and then the adapter creates a database the operator never allowed
(MR-005.5).

## Proposed surface

```rust
pub struct OpenOptions {
    pub config: EngineConfig,
    pub use_default_embedder: bool,
    pub create_if_missing: bool,   // new; default true (today's behaviour)
}
```

- With `create_if_missing: false`, the existence check moves into
  `admit_current_database` (`open.rs:338`). That function already runs after the
  sidecar lock is held (`open.rs:794-802`). A `NotFound` or zero-length file
  returns a new `EngineOpenError::DatabaseNotFound`. SDK `ErrorKind` is
  `DatabaseNotFound`, with Python and TypeScript classes of the same name.
- The lock file is created before admission. On refusal, the engine removes the
  lock file it just created, but only if it created it (`create_new` succeeded).
  That way a refused open leaves nothing behind.
- The dangling-symlink refusal (`open.rs:385-396`) keeps precedence and its
  `Io` error.

## Invariants and consistency

- Under the lock, no other FathomDB opener can create the file between the check
  and the open. A non-FathomDB process can still create or delete it. That is
  outside the engine's control, and it no longer leads to creation.
- With the default `true`, behaviour is byte-identical to today.
- A zero-length file counts as "missing" for this flag, because admission
  already treats it as fresh (`open.rs:348-350`).

## Interaction with ADRs

- `ADR-0.8.26-breaking-v1-contract-and-fresh-database-boundary.md` sets the
  admission rule. This adds a refusal mode, not an upgrade path.
- `ADR-0.6.0-error-taxonomy.md`: a new `#[non_exhaustive]` variant, not a
  semver break, but every binding's error table changes.
- `ADR-0.6.0-database-lock-mechanism-reader-pool-revision.md`: unchanged. The
  check runs under the existing sidecar lock.
- `ADR-0.6.0-no-shims-policy.md`: the default stays `true` because changing it
  would break every SDK caller. That is an owner choice, not a shim.

## Cost and performance

One `metadata` call that admission already makes. Nothing else.

## Test plan sketch

- Real filesystem: `create_if_missing: false` on a missing path returns
  `DatabaseNotFound`, and leaves no database, `-wal`, `-shm` or `.lock` file.
- On an existing current database it opens normally.
- On a zero-length file it returns `DatabaseNotFound` and leaves the file
  untouched.
- A race test in the engine's unit tests, using the `#[cfg(test)]`
  `run_admission_locked_hook_for_test` seam (`open.rs:799-800`): delete the file
  while the lock is held, and get `DatabaseNotFound`, not a created database.
- Default `true` keeps today's open tests green.

## Adapter change once it lands

Pass `create_if_missing: entry.create` and delete the `try_exists` check and its
race comment. Map `DatabaseNotFound` to `unavailable/missing_file`.

## Open questions for the owner

1. Default `true` (compatible) or `false` (safer, breaking for every SDK user)?
2. Should a refused open remove a `.lock` file it created, or leave it, as other
   open failures may do today? I did not verify what other failure paths do with
   a freshly created lock file.
