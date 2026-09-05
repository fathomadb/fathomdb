---
title: ADR-0.8.25-frozen-pagination-and-operational-state
date: 2026-09-05
target_release: 0.8.25
status: accepted by approved 0.8.25 scope and Slice 45 execution authorization
supersedes: none
---

# ADR-0.8.25 — frozen pagination and operational state

## Decision

FathomDB adds minimal keyset pagination for canonical logical nodes and
governed point/page reads for registered `latest_state` collections. Canonical
and operational pages require the existing Slice 35 frozen context. The Engine
mints an opaque HMAC-authenticated continuation bound to database identity,
operation, selector, frozen context, page limit, ordering version, and the last
returned write cursor.

Eligibility and lifecycle predicates execute before the page limit. Ordering
is ascending `write_cursor`; page limits are `1..=250`. Continuations never use
offsets or retain a server-side lease. Any context drift, cursor mismatch,
tampering, foreign database, unsupported collection shape, or unsupported
version fails through the closed `FrozenReadError` or `PageError` vocabulary.

Operational reads accept only a registered `latest_state` collection with
format version 1. A point read may be current or frozen; a page is frozen. They
read `operational_state` directly and never replay `operational_mutations`.

## Performance and state binding

Schema 33 uses the monotonic read-visibility generation as the authenticated
terminal-state drift signal instead of re-hashing every projection-terminal
row on each frozen read. Historical schema-31 and schema-32 token encodings
remain pinned. Dedicated write-cursor indexes keep frozen-boundary and
dependency-eligibility lookups indexed. This changes neither drift semantics
nor the token's content-free property.

The release records paired 10k/50k latency and peak-RSS measurements. A median
paired p95 increase is material only when it exceeds both 10% and 0.25 ms; a
peak-RSS increase is material only when it exceeds both 5% and 8 MiB.

## Consequences

- Ranked top-K search remains distinct from ordered pagination.
- No graph/search cursor, arbitrary ordering, retained snapshot, cursor lease,
  or semantic latest-state policy is introduced.
- `PageV1<NodeRecord>` is intentionally compact; source-complete evidence is
  owned by Slice 50.
- Python uses `fathomdb.read.*`; TypeScript uses the existing `read.*`
  namespace. Rust methods live on `Engine`.

The executable contract is
`dev/plans/0.8.25/features/slice-45/design.md`.
