---
title: EP-01 — Content-free store-state read on a reader connection
date: 2026-10-17
status: PROPOSED
desc: Draft engine proposal. An O(1)-class SDK read of the read-visibility generation, its branch nonce, the canonical write boundary and the projection cursor, taken on a reader snapshot without the writer lock. Prerequisite for the MCP state and drift contract.
---

# EP-01 — Content-free store-state read on a reader connection

Status: Draft. Priority P0. Not an accepted decision.

## Problem

The adapter reports `state: {status: "unavailable"}` on every response
(`src/rust/crates/fathomdb-mcp/src/tools.rs:267`). Drift labels (MR-051),
continuation re-derivation (MR-041.4) and the reopen check (MR-006.3) are off.

The engine already maintains the values, but exposes them only through
`freeze_read_context`:

- `Engine::freeze_read_context` locks the writer connection
  (`fathomdb-engine/src/search_api.rs:12`) and runs `frozen_read::mint`. That
  opens a transaction on the writer connection (`frozen_read.rs:224`), counts
  every canonical row (`frozen_read.rs:225`), validates filter attributes, and
  hashes the projection registry and serving state (`frozen_read.rs:470-484`).
- The generation lives in the singleton `_fathomdb_read_visibility_state`
  (`frozen_read.rs:338-347`). Row triggers on 18 tables advance it
  (`frozen_read.rs:34-53`, checked on open at `frozen_read.rs:376-411`). Batched
  canonical writes and projection commits advance it explicitly
  (`write_commit.rs:527`, `projection_commit.rs:447`).
- Each advance also rewrites `state_nonce` with 32 random bytes
  (`write_commit.rs:571-585`; the trigger form at `frozen_read.rs:396-401`).
- The generation aborts instead of wrapping at `i64::MAX`
  (`write_commit.rs:578`, `frozen_read.rs:399-401`).
- The canonical write boundary is `load_next_cursor`
  (`write_commit.rs:5-27`): a `MAX` over `canonical_nodes`, `canonical_edges`,
  `operational_mutations`, `operational_state`, a reserved key and
  `_fathomdb_dependency_closures.admitted_write_boundary`.
- The projection cursor is one `_fathomdb_open_state` key
  (`projection_commit.rs:34-46`).

Workaround removed: none exists. The adapter reports `unavailable` rather than
approximate, as `requirements.md` requires.

## Proposed surface

`fathomdb_sdk::read::visibility_state(engine) -> Result<VisibilityStateV1>`,
with Python `fathomdb.read.visibility_state` and TypeScript
`read.visibilityState`.

```rust
#[non_exhaustive]
pub struct VisibilityStateV1 {
    pub schema_version: u32,          // 1
    pub database_id: String,          // existing 32-hex open-state id
    pub generation: u64,              // _fathomdb_read_visibility_state.generation
    pub nonce: String,                // state_nonce, 64 hex
    pub canonical_write_boundary: u64,
    pub projection_cursor: u64,
}
```

- Runs as a new `ReaderRequest` on the reader pool, in one deferred read
  transaction, so all fields come from one snapshot. It takes no writer lock
  and returns no content.
- Errors: `Closing` after close, and `Storage` if the singleton is missing.
  That is the same as `load_visibility_generation` (`frozen_read.rs:338-347`).
- Callers derive an identifier from `(database_id, generation, nonce)`. The
  boundary and cursor are only cause hints, labelled `canonical` or `index`.

## Invariants and consistency

- Within one database file the generation never decreases. The nonce makes the
  pair branch-sensitive: if a backup is restored and then written, the
  generation can repeat, but the nonce differs.
- The engine already rejects a generation below the highest one this process
  observed (`search_api.rs:15-22`, `search.rs:1367-1384`). That high-water mark
  is in memory only and is reloaded from the file on open (`open.rs:904-969`). So
  after a reopen, the nonce is the only regression signal. The new read should
  apply the same `fetch_max` check and return `FrozenRead(StateUnavailable)` on
  regression, for consistency.
- Purge and erasure delete trigger-covered rows, so they advance the
  generation. The write boundary can fall after a purge. It is never a proof of
  sameness.
- Verified gap: there is no trigger on `operational_mutations`, and the trigger
  count check would reject one (`frozen_read.rs:408-415`). So an append to an
  append-only collection changes the write boundary without changing the
  generation. That is harmless for search, but MR-051 cause labels must allow
  it, and `requirements.md` overstates "op-store tables".
- Not verified: whether `Engine::open` advances the generation, for example
  during recovery or vector-equivalence probe registration. A test must pin it.

## Interaction with ADRs

- `ADR-0.8.25-eligibility-and-frozen-reads.md` defines the generation and keeps
  frozen tokens reproduce-or-fail. This read exposes the same values without
  minting. It is an additive operation, not a successor.
- `ADR-0.8.25-frozen-pagination-and-operational-state.md` made the generation the
  schema-33 terminal-state drift signal. No change.
- `ADR-0.8.27-rust-sdk-parity.md` requires the operation in all three SDKs and
  in `src/conformance/governed-operation-parity.json`. The governed-surface
  allowlist is HITL-signed (`ADR-0.8.0-supersede-five-verb-surface-cap.md`), so
  adding the verb needs that sign-off.

## Cost and performance

- Generation, nonce, cursor and database id are singleton or key lookups.
- `load_next_cursor` is not O(1). `canonical_nodes` and `canonical_edges` have
  `write_cursor` indexes (`fathomdb-schema/src/lib.rs:955-956`), and so does
  `operational_state` (`lib.rs:1309-1310`). No index found on
  `operational_mutations.write_cursor` or on `admitted_write_boundary`, so those
  two terms scan their tables. Options are additive indexes, or dropping the
  boundary from v1 and labelling cause from the projection cursor alone.
- Round-robin reader dispatch (see EP-09) means a probe can queue behind a long
  read on the same worker. This held before
  [EP-18](EP-18-reader-dispatch-and-pool-size.md) part A; now a request queues
  behind a busy worker only when all eight readers are busy.

## Test plan sketch

- Red first, real database: generation unchanged across two reads with no
  commit; advanced after `write`, after a projection commit, after `purge`.
- Same-snapshot test: a writer commits between the probe and a search; the
  probe never returns a mix of pre-commit and post-commit fields.
- Reopen test: values are equal across close and open with no writes in
  between.
- Restore test: copy the file, write to the original, restore the copy, write
  once. Generation may match; nonce differs.
- No-writer-lock test: the probe completes while the writer holds a long
  transaction.
- Property test over the `VisibilityStateV1` binding encoding, if it gets a
  wire codec.

## Adapter change once it lands

The bridge brackets each governed read with two probes, as `architecture.md`
§5 describes. Envelope `state` becomes `{id, cause?}`. `state_advanced`
labels, MR-041.4 re-derivation and MR-006.3 come on. `stale_handle` for
same-store, new-epoch continuations becomes re-derivation.

## Open questions for the owner

1. Bracketing costs two extra reader requests per call. Should the state also
   ride on search, `get_many` and `graph.expand` results from the same snapshot,
   as `SearchResult.projection_cursor` already does (`search_api.rs:528-535`)?
   Then no retry is needed.
2. Include the write boundary in v1 at its scan cost, or add the two indexes?
3. A scope-limited drift signal (architecture §7) is out of this proposal.
   Defer it?
