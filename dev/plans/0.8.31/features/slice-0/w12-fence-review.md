---
title: Slice 0 W12 erasure ergonomics and checkpoint-fence technical review
status: COMPLETE
target_release: 0.8.31
slice: 0
date: 2026-11-22
---

# Slice 0 W12 erasure ergonomics and checkpoint-fence technical review

Source-grounded evaluation of Memex Ask 3 (W12 erasure ergonomics) in
`release/0.8.31`, honoring owner's 2026-11-04 ruling (*physical erasure is done
— defects only*; no new erasure research campaign).

## 1. Checkpoint-only owed closure write-fence on unrelated writers (Ask 3a)

### Observed behavior in `0.8.30`

- In `Engine::finish_physical_dependency_closures_inner`
  (`src/rust/crates/fathomdb-engine/src/erasure.rs`), FTS sanitation and
  telemetry redaction run first, then `physical_erasure::prepare_redacted`
  (`physical_erasure.rs`) sets `debt.redacted = true` and **clears**
  `debt.denied_sources`, `debt.denied_identities`, `debt.cursors`, and
  `debt.retry` *before* calling `Engine::checkpoint_erasure_at_rest`
  (`wal_checkpoint(TRUNCATE)`).
- While `!debt.redacted`, `physical_erasure::guard_plain_write_inner`
  (`physical_erasure/admission.rs`) *already allows* unrelated plain `Node`/
  `Edge` writes in Autonomous mode, advancing `debt.safe_boundary` via
  `certify_plain_write`.
- Once `prepare_redacted` sets `debt.redacted = true` and a `wal_checkpoint`
  BUSY occurs, `guard_plain_write_inner` rejects all writes and falls through
  to `dependency_closure::guard_no_pending_physical` (`dependency_closure.rs`),
  fencing even unrelated plain writes until the checkpoint retry succeeds.

### Safety analysis and Slice 10 disposition

- **Unsafe to bypass `guard_no_pending_physical` directly after
  `prepare_redacted`**: once `denied_sources`/`denied_identities` are cleared,
  the engine can no longer distinguish unrelated writes from writes re-inserting
  erased identities, and advancing `write_cursor` without `certify_plain_write`
  breaks `validate_physical_zero` (`current_write_boundary <= safe_boundary`).
- **Safe, bounded Slice 10 fix (Autonomous mode only)**:
  - In Autonomous mode, `RootDebt` stores only SHA-256 hashes and integer
    cursors (no plaintext selectors). Perform the first `wal_checkpoint(TRUNCATE)`
    **before** `prepare_redacted` clears `denied_sources`/`denied_identities`
    and flips `debt.redacted = true`.
  - If that `wal_checkpoint(TRUNCATE)` fails BUSY, `debt.redacted` remains
    `false`, so `guard_plain_write_inner` + `certify_plain_write` continue
    admitting unrelated plain canonical writes cleanly while keeping denied
    sources/identities and dependency-mutating/actuation/admin writes strictly
    fenced!
  - Once the first truncate succeeds, run `prepare_redacted` + final
    `wal_checkpoint(TRUNCATE)` + finalize so zero debt metadata remains in the
    WAL at rest.
  - **Targeted amendment to `ADR-0.8.28-resumable-physical-cleanup.md`**: this
    introduces a bounded two-truncate Autonomous completion sequence (truncate
    before `prepare_redacted`, then `prepare_redacted` + final truncate +
    finalize). Implementation must audit existing `ActualCheckpointObserver`
    and checkpoint-count assertions so they match the amended Autonomous
    sequence without weakening single-truncate guarantees where applicable.
  - ExplicitMaintenance tickets (which hold plaintext selectors) keep their
    existing scrub-before-truncate ordering untouched.

## 2. Completing-retry `EraseReport` indistinguishable from already-erased (Ask 3b)

### Observed behavior in `0.8.30`

In `Engine::erase_source_shared` (`erasure.rs`), both:

1. the `if root_retry || !pending.is_empty()` retry path, and
2. the subtle `finish_redacted_before_request` path (when `debt.redacted == true`
   finishes the pending closure *before* `excise_source_inner` runs on
   already-deleted rows)

return `ExciseReport { nodes_excised: 0, edges_excised: 0, projections_invalidated: 0 }`
— indistinguishable from calling `erase_source` on a source that never existed
or was already erased.

### Slice 10 fix

- Add additive boolean field `completed_pending_erasure: bool` (TypeScript:
  `completedPendingErasure: boolean`) to `ExciseReport` / `EraseReport` across
  `fathomdb-engine`, `fathomdb`, `fathomdb-sdk`, `fathomdb-py` (`_fathomdb.pyi`),
  and `fathomdb-napi` / `src/ts/`.
- Set `completed_pending_erasure = true` whenever either
  `finish_redacted_before_request` or the `root_retry || !pending.is_empty()`
  branch completed an owed physical closure during the call.
- **Cross-source audit guard**: because `prepare_redacted` clears `debt.retry`,
  `finish_redacted_before_request` can finish a *prior* source's redacted
  closure before `excise_source_inner` excises real rows for a *different*
  `source_id`. Skip writing `excise_source_audit` **only** when
  `completed_pending_erasure == true && nodes_excised == 0 && edges_excised == 0`
  — never skip the audit row when real rows were excised for the current call.
- Set `completed_pending_erasure = false` on normal fresh erasures and true
  already-erased no-ops.

## 3. BUSY checkpoint error text unconditionally blames caller reader (Ask 3c)

### Observed behavior in `0.8.30`

`Engine::checkpoint_erasure_at_rest` (`erasure.rs`) computes `classification`
via `WalAttributionCollector::classification` (`wal_attribution.rs`:
`"local_checkpoint_overlap"`, `"owned_reader_snapshot"`,
`"owned_runtime_transaction"`, or `"unclassified_external"`) and
`snapshot.active_roles`, logs them, and then discards them when formatting
`EngineError::ErasureIncomplete { stage: "wal_checkpoint", detail }` — always
claiming `"a concurrent reader is pinning a WAL snapshot"` and `"`{verb}`
deleted its rows"` even on completing retries or internal runtime transactions.

**Critical production catch (Slice 0 review P1-1):** `WalAttributionCollector::new()`
sets `enabled: cfg!(test) || std::env::var_os("FATHOMDB_WAL_ATTRIBUTION").is_some()`.
When `!enabled`, `register()`, `set()`, and `checkpoint_begin()` early-return
without recording anything — so in production release builds without
`FATHOMDB_WAL_ATTRIBUTION`, `classification()` would always return
`"unclassified_external"` even when an owned reader or projection worker is
pinning the WAL snapshot. Enabling full trace logging unconditionally would
acquire a `Mutex` and write `eprintln!("slice65_wal ...")` on every transaction.

### Slice 10 fix

- Decouple **always-on lightweight atomic active-role and checkpoint-overlap
  counters** (zero `Mutex` contention and zero `eprintln!` on the hot path)
  from the opt-in `FATHOMDB_WAL_ATTRIBUTION` stderr trace sink.
- Drive `Engine::checkpoint_erasure_at_rest`'s `ErasureIncomplete.detail` from
  those always-on atomic counters so release builds truthfully report
  `"local_checkpoint_overlap"`, `"owned_reader_snapshot"`,
  `"owned_runtime_transaction"`, or `"unclassified_external"` plus active role
  counts, distinguish initial erasure vs. completing retry wording, and avoid
  blaming a caller reader when the blocker is internal or unclassified.
