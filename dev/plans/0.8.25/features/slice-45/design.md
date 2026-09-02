---
title: 0.8.25 Slice 45 — minimal pagination and operational-state design
status: DRAFT_SCOPE_RECONCILED_BLOCKED_ON_SLICE_7
design_version: 3
target_release: 0.8.25
depends_on: 40
architecture: dev/design/fathomdb-data-plane-architecture-v2.md
---

# Slice 45 design

## Authority and boundary

Implements the retained subset of R25/AC25-45, Memex need 9, the current-state
integration need, and A25-03/A25-05. It adds bounded keyset continuation for
canonical records and existing `operational_state`, plus an operational-state
point read. Ranked top-K remains cursor-free.

Graph pagination, persisted cursor leases, and generalized long-lived replay
are allocated after 0.8.25. `latest_state` remains a collection-kind literal
and consumer term; it is not a new FathomDB storage authority.

## Contract

```text
PageCursor(string)
PageRequestV1 { schema_version: 1, limit: 1..250, cursor?, context? }
PageV1<T> { schema_version: 1, items, next_cursor?, read_boundary }
OperationalStateRecordV1 {
  schema_version: 1, collection, record_key, payload, schema_id?, write_cursor
}
```

Methods are `read_canonical_page(kind, page)`,
`read_operational_state(collection, key, context?)`, and
`read_operational_state_page(collection, page)`. Canonical order is
`(stable_id, artifact_revision_id)`; operational state order is
`(collection, record_key)`. Both use `LIMIT limit+1` keyset queries, never
OFFSET. Existing mutation-log `after_id` remains unchanged.

The first page captures a compact Slice 35 read boundary in the cursor even
when the caller did not request a reusable frozen context. A continuation
binds database, operation, normalized request, eligibility, boundary,
applicable projection generations, ordering version, and exclusive last key.
It is authenticated and contains no record content.

If immutable revisions can reproduce the bound page, continuation does so. If
canonical or operational-state generation changed in a way that cannot be
reproduced, the whole call returns `cursor_stale`; it never mixes pages or
silently skips a changed row. Supplying a caller frozen context additionally
requires exact normalized equality. Current access, lifecycle, erasure, and
closure barriers are rechecked and can return the same non-disclosing
`page_unavailable` for the whole continuation.

## Invariants and compatibility

- A successful walk has no duplicate or omission and one deterministic order.
- A concurrent change yields the bound result or one typed whole-page failure.
- Cursor possession never authorizes or widens eligibility.
- Point and first-page operational reads agree at the same boundary.
- Current-state reads use indexed `operational_state`; they do not collapse or
  scan the mutation log.
- Existing bare search and ranked top-K accept and emit no page cursor.

Failures are `cursor_invalid`, `cursor_mismatch`, `cursor_stale`,
`cursor_generation_unavailable`, `page_unavailable`, and `invalid_page_limit`.
All public/wire behavior follows Slice 15 rules.

## RED/GREEN and verification

Property tests cover codec/tamper, keyset concatenation, ordering, limits, and
request mismatch. Real-database races cover insert, replacement, delete,
supersede, erasure, state replacement, access revocation, close/reopen, and
generation changes; each produces exact pages or a typed whole-page failure.
Query-plan fixtures prove indexed state point/page reads and no mutation-log
scan.

Run fast, heavy, all/all-feature, Windows Rust/Python/Node, and locally packed
pagination routes. Operator, CUDA, live-model, and pre-publication registry
routes are N/A. A formal independent READY review remains required after Slice
7 and Slice 40 complete.
