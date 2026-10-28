---
title: EP-16 — Governed change feed keyed by write cursor
date: 2026-10-17
status: PROPOSED
desc: Draft engine proposal. A read of governed record events (created, replaced, retracted, validity boundary passed) since a cursor, in commit order, under a ReadView, so agents can ask what changed without re-searching.
---

# EP-16 — Governed change feed keyed by write cursor

Status: Draft. Priority P2. Not an accepted decision.

## Problem

None of the existing reads is a governed change feed:

- `read.mutations` is the same read as `read.collection`: op-store rows of one
  collection after an id (`fathomdb-sdk/src/read.rs:39-59`). It does not cover
  canonical records.
- `read.crossed_boundary_since(since, view)` is keyed by a wall-clock `since` in
  epoch milliseconds (`read.rs:117-124`), not by cursor. It covers validity
  crossings only.
- `read.canonical_page` pages one kind in ascending `write_cursor` under a frozen
  context (`read.rs:81-91`;
  `ADR-0.8.25-frozen-pagination-and-operational-state.md`). It needs
  `freeze_read_context`, which takes the writer lock (EP-01). It returns the
  rows visible under the view, so a retraction shows up as an absence, not as an
  event.
- Supersession is tombstone-then-insert. The old row keeps its `write_cursor` and
  gains `superseded_at` (`fathomdb-engine/src/search.rs:1565-1569`). So a
  replacement creates a new cursor, but a retraction does not. It updates
  `state` in place (`record_lifecycle.rs:293`). Neither change leaves an event,
  ordered by cursor, that a reader could follow.

Adapter status: the change family is absent (interface §4, "Not present in this
release"). MR-055 is reported `unavailable`.

## Proposed surface (outline)

`read::changes(engine, after: ChangeCursorV1, options) -> Result<ChangesV1>`.
Events are `Created{l, cursor}`, `Replaced{l, old_cursor, new_cursor}`,
`Retracted{l, cursor, state}` and `BoundaryPassed{l, at}`, under one `ReadView`
and an optional kind filter. Each page ends with the next cursor and the EP-01
state.

Retractions need an ordered record. The two options are:

1. An append-only `_fathomdb_change_log(change_cursor, logical_id, event, ...)`
   written in the same transaction as each governed mutation. It is additive,
   but it is a new write-path obligation, and erasure must purge it.
2. A `state_changed_cursor` column stamped on lifecycle updates. That is
   cheaper, but it gives no history of repeated transitions.

## Invariants

- Commit order, no gaps for a fixed view, and per-store cursors (MR-005.6).
- Erasure removes erased records' events. A change log must not outlive the
  data it describes. This follows `ADR-0.8.27-offline-physical-erasure-completion.md`.

## Interaction with ADRs

This is a new governed read and possibly a new table. It needs its own ADR, and
it must state its relation to the frozen-pagination ADR ("No graph/search
cursor") and to the OPP-12 lifecycle protocol.

## Cost and performance

Option 1 adds one row per governed mutation and one index. Option 2 adds one
column write per lifecycle change. Neither has been measured.

## Test plan sketch

MR-055.1 on a real database: three writes and one replacement give exactly four
events in commit order. A retraction gives one `Retracted`. An erased source
leaves no events.

## Adapter change once it lands

A `changes` tool, with a `cursor` in every read response (MR-055.2). The cursor
is the EP-01 write boundary or a dedicated change cursor, depending on the
option chosen.

## Open questions for the owner

1. Change log table, or stamped column?
2. Should boundary crossings (validity time passing) be events in the same feed,
   even though no commit causes them?
