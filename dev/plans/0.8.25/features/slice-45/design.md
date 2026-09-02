---
title: 0.8.25 Slice 45 — governed pagination and operational state design
status: REVIEWED_MAX_ENVELOPE_SCOPE_NARROWED
design_version: 2
target_release: 0.8.25
depends_on: 40
readiness_blocked_on: Slice 7 architecture activation
architecture: dev/design/fathomdb-data-plane-architecture-v2.md
---

# Slice 45 — governed pagination and operational state design

> **0.8.25 implementation boundary:** Retain compact stable canonical and
> `operational_state` continuation plus point reads. General graph pagination
> and the full cursor-lease envelope below are deferred by the
> [scope adjustment](../../scope-adjustment-2026-09-02.md). Reconcile this
> maximum-envelope design before READY review.

## Authority and comparison

Owns R25/AC25-45, Memex need 9, current operational-state integration, and
A25-03/A25-05. `operational_state` is the governed FathomDB surface.
`latest_state` remains the existing collection-kind literal/consumer concept.

| Need | Existing | Decision |
| --- | --- | --- |
| Stable pages | Mutation `after_id`; bounded vectors elsewhere | Opaque keyset pages for canonical, direct adjacency, and operational state. |
| Authority | No bound continuation | Every page supplies exact current frozen context and bound cursor. |
| Current state | Existing authoritative table, no governed read | Direct point/page reads; never collapse mutation log. |
| Ranked search | Top-K | Remains separate and cursor-free. |

Locked op-store/same-file designs are preserved; append-log `after_id` stays.
This is the successor for generalized pages/current-state reads. Slice 60 alone
owns combined search expansion and path continuation.

## Public and wire contract

```text
PageCursor(string)
PageRequestV1 { schema_version: 1, limit: 1..1000, cursor?,
                context: ReadContextV1 }
PageV1<T> { schema_version: 1, items, next_cursor?, snapshot }
OperationalStateRecordV1 { schema_version: 1, collection, record_key,
  payload, schema_id?, write_cursor }
DirectAdjacencyRequestV1 { schema_version: 1, seed: IdSpace,
  direction: incoming | outgoing | both, page: PageRequestV1 }
DirectNeighborV1 { schema_version: 1, neighbor: NodeRecordV1 }
```

Methods: `read_canonical_page(kind, page)`,
`read_direct_adjacency_page(request)`, `read_operational_state(collection,key,
context)`, and `read_operational_state_page(collection,page)`. Direct adjacency
is exactly one hop over the existing governed graph store, has no edge-kind,
target-kind, or path expansion constraint, and returns each visible neighboring
stable node once even when multiple edges reach it. Combined expansion belongs
to Slice 60.

Every continuation supplies `ReadContextV1` and must exactly equal the cursor's
originating frozen view/eligibility/snapshot under Slice 35. Point/page share
one operational record codec. All records inherit Slice 15 request/response/
unknown/u64/error/SDK/Windows/registry rules and canonical fixtures; opaque
cursors carry an internal version.

Failures: `CursorInvalid`, `CursorMismatch`, `CursorExpired`,
`CursorSnapshotDrifted`, `CursorGenerationUnavailable`, and `PageUnavailable`.

## Cursor, ordering, and authority flow

The HMAC-SHA-256 cursor uses the Slice 35 key and contains version, database,
operation, canonical request digest, snapshot ID, generation digest, order ID,
exclusive last key, and expiry. Its expiry equals the earlier of snapshot expiry
and 900 seconds after cursor issue; it cannot renew the snapshot.

Pagination is keyset, never OFFSET. Canonical order is `(stable_id,
record_revision_id)`. Direct adjacency deduplicates by neighbor stable ID and
orders by that ID. Operational state orders `(collection_name, record_key)`.
Eligibility precedes `ORDER BY ... LIMIT limit+1`; the extra row only decides
continuation. Existing mutation-log `after_id` remains unchanged.

Validation/error precedence is: authenticate/version/database/expiry; require
exact current context; recheck current access/lifecycle/erasure barriers;
verify generation; then read. Context mismatch returns `CursorMismatch` without
row lookup. Any newly hidden/revoked/erased state returns identical
non-disclosing `PageUnavailable` for the entire page; it is never skipped, so a
walk cannot silently omit rows. Drift/generation failures occur only after
authority succeeds. No error contains keys, predicates, or payloads.

## Invariants, compatibility, and performance

- One walk has no duplicate/omission under concurrent mutation; it reproduces
  the frozen boundary or fails typed.
- Cursor possession never authorizes or widens context.
- New walks see replacement; old walks reproduce or fail. Point/page agree.
- Ranked top-K never accepts/emits `PageCursor`.
- Existing `latest_state` wire literal remains; API/docs say current
  `operational_state`.
- SQL is covering-index/keyset O(page); cursor is at most 1 KiB.

## Tests and verification

Property tests cover codec/tamper/keyset/dedup/order/page concatenation. Races
cover insert/update/delete/supersede/erase/access revocation and prove either
exact pages or whole-page non-disclosure. Negative tests cover every binding,
expiry, version, limit, and generation mismatch. Real-table/query-plan tests
prove operational point/page and no mutation-log scan. Rust/Python/TypeScript/
wire goldens, Windows CPU/native, restart, registry pages, and compact-search
regressions pass.

Routes: fast/heavy/all/all-feature, Windows Rust/Python/Node, registry pages.
Operator/CUDA/live-model are N/A. Status remains `REVIEWED_BLOCKED_ON_SLICE_7`, blocked on
Slice 7/40; no implementation-shaping decision remains.
