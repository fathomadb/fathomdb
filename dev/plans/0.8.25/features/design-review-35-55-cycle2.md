---
title: 0.8.25 independent design re-review — Slices 35–55 cycle 2
status: COMPLETE
review_cycle: 2
reviewed_on: 2026-09-01
verdict: CHANGES_REQUIRED
---

# Independent design re-review — Slices 35–55 cycle 2

## Verdict

**CHANGES_REQUIRED.** FIX-2 resolves all four cycle-1 findings as written. One
new P2 boundary omission is introduced by the new reverse-dependency-index
generation contract. No P1 or P3 findings remain.

The designs continue to preserve historical records, keep semantic policy
outside FathomDB, and remain correctly `DRAFT_REVIEW`/Slice 7 gated.

## Cycle-1 finding verification

| Finding | Cycle-2 result | Evidence |
| --- | --- | --- |
| C1-50-01 evidence-receipt identity privacy | RESOLVED | Persisted receipt fields are exhaustively limited to opaque revision/set/generation/event IDs; free-form source/logical/path/session/owner/payload IDs are forbidden and covered by raw-table/WAL privacy tests. |
| C1-55-01 lost trace page | RESOLVED | Immutable response bytes and next cursor commit under `(lease,input_ordinal)` with frontier advancement; identical/lost/concurrent input-cursor use replays the stored page. |
| C1-55-02 integrity authority boundary | RESOLVED for canonical/Slice-40 state | Job/status/findings/plans share one Engine-minted database/write/time/liveness/projection boundary; pages reproduce it or terminate typed incomplete. |
| C1-55-03 partial reverse-index repair | RESOLVED | Generation-scoped shadow build, ordered dual writes/tombstones, reciprocal verification, atomic activation, fail-closed corrupt-old reads, restart, cleanup, and state receipts are defined. |

## Cycle-2 finding

### C2-55-01 — P2 — Integrity boundary omits the active reverse-index generation

**Location:** `slice-55/design.md`, **Frozen integrity job contract** and
**Reverse dependency index generation and cutover**.

`IntegrityBoundaryV1` binds canonical write/time/liveness and Slice 40
projection generation IDs. FIX-2 then creates a separate
`_fathomdb_reverse_index_generations` authority with its own active identity,
but that identity is absent from the job boundary, findings, and repair action.
An active reverse-index generation can change between integrity pages or
between finding and plan acceptance without violating the stated boundary.
The job could therefore combine generations or repair a different index than
the one found inconsistent.

**Required correction:** add `active_reverse_index_generation_id` (nullable
only before the substrate exists) and its authoritative forward-set boundary or
digest to `IntegrityBoundaryV1`. Persist it on status/findings/plans. Every
dependency reverse-index page must reproduce that exact generation; change or
absence ends typed `incomplete`. `RegenerateDependencyReverseIndex` must name
the found generation plus forward-set boundary, and plan acceptance must reject
if either changed. After activation, a new integrity job receives the new
generation; an existing job is never rebound.

## Final checks

- Cycle-1 privacy correction: **PASS**.
- Trace retry/concurrency correctness: **PASS**.
- Reverse-index rebuild/cutover correctness: **PASS**, subject to C2-55-01's
  job-boundary binding.
- Semantic boundary: **PASS**.
- Historical preservation: **PASS**.
- Wire/version rules: **PASS**.
- READY status: correctly blocked on Slice 7 and this unresolved P2.
