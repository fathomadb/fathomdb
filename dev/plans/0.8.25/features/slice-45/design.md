---
title: 0.8.25 Slice 45 — minimal pagination and operational-state design
status: DRAFT_REVIEW
design_version: 4
review_cycle: 0
target_release: 0.8.25
depends_on: 40
architecture: dev/design/fathomdb-data-plane-architecture-v2.md
---

# Slice 45 design

## Authority, outcome, and limits

This design implements the retained core of R25/AC25-45, Memex need 9, the
current-operational-state integration need, and A25-03/A25-05. It adds:

- bounded stable pages over logical canonical nodes;
- governed point and page reads over the existing `operational_state` table;
  and
- compact authenticated keyset cursors bound to the Slice 35 frozen-read
  authority.

It does not add graph pagination, ranked-search cursors, retained snapshot
leases, persisted cursor state, arbitrary collection queries, or a
`latest_state` authority. Those deferred contracts remain in
[`0.8.x-after-0.8.25-design-notes.md`](../../../../design/0.8.x-after-0.8.25-design-notes.md).
Ranked top-K search and the existing mutation-log `after_id` API are unchanged.

The existing `ReadView`, Slice 35 eligibility grammar and frozen token, Slice
15 revision identity, and `operational_state` table remain authoritative. This
design supersedes the unreviewed Slice 45 draft only; accepted predecessor
designs remain historical evidence.

## Requirements and acceptance criteria

| Requirement | Acceptance criterion |
| --- | --- |
| S45-R1 Stable canonical pages | S45-AC1 concatenating all pages equals one reference query in `(logical_id, write_cursor)` order, with no duplicate or omission. |
| S45-R2 Frozen continuation | S45-AC2 every page requires one valid Slice 35 frozen context; unchanged state succeeds across restart, while relevant drift refuses before returning any item. |
| S45-R3 Bound opaque cursor | S45-AC3 tamper, another database, operation/selector/context/limit mismatch, noncanonical encoding, and unsupported version fail typed; cursor bytes contain no record, filter value, source, or key text. |
| S45-R4 Governed operational state | S45-AC4 only a registered `latest_state` collection can be read; point and page results agree in one frozen context and never scan `operational_mutations`. |
| S45-R5 Concurrency safety | S45-AC5 each call linearizes on one SQLite reader snapshot; a concurrent relevant mutation yields the bound result or a typed whole-call failure, never a mixed page. |
| S45-R6 Compatibility and parity | S45-AC6 existing read/search shapes remain byte-compatible; Rust, Python, TypeScript, wire fixtures, Linux, and Windows agree on the new additive contract. |
| S45-R7 Bounded overhead | S45-AC7 matched 10k/50k cold and steady cells isolate context mint, validation, cursor, page-query, throughput, and RSS costs; a preregistered material effect receives causal analysis and reviewed disposition before closure. |

## Public contract

### Shared types

```text
PageCursor(string)

PageRequestV1 {
  schema_version: 1,
  limit: 1..250,
  cursor: PageCursor?
}

PageV1<T> {
  schema_version: 1,
  items: [T],
  next_cursor: PageCursor?
}

CanonicalNodePageItemV1 {
  schema_version: 1,
  artifact_revision_id: ArtifactRevisionId,
  logical_id: string,
  kind: string,
  body: string,
  source_id: SourceId?,
  write_cursor: u64
}

OperationalStateRecordV1 {
  schema_version: 1,
  collection: string,
  record_key: string,
  payload: string,
  schema_id: string?,
  write_cursor: u64
}
```

`CanonicalNodePageItemV1` is the paginated counterpart of the existing logical
node list family. It includes immutable revision identity but intentionally
does not duplicate Slice 50 source-version, locator, hash, or evidence
resolution. Anonymous content-ID nodes cannot satisfy the logical-node shape
and are excluded before the limit. Edges remain available through governed
graph reads and are not silently folded into this node page.

All new requests and responses carry `schema_version = 1`. The cursor is an
opaque scalar with its version in the prefix. Slice 15 request strictness,
response evolution, unknown-variant rejection, unsigned-decimal dynamic wire
encoding, typed-error, SDK, registry, and Windows rules apply. Python uses
snake case, TypeScript uses camel case, and canonical fixtures pin both.

### Methods

```text
Engine::read_canonical_page(
  kind: &str,
  context: &FrozenReadContextV1,
  page: &PageRequestV1
) -> Result<PageV1<CanonicalNodePageItemV1>, EngineError>

Engine::read_operational_state(
  collection: &str,
  record_key: &str,
  context: Option<&FrozenReadContextV1>
) -> Result<Option<OperationalStateRecordV1>, EngineError>

Engine::read_operational_state_page(
  collection: &str,
  context: &FrozenReadContextV1,
  page: &PageRequestV1
) -> Result<PageV1<OperationalStateRecordV1>, EngineError>
```

Python exposes the same snake-case methods. TypeScript exposes
`read.canonicalPage`, `read.operationalState`, and
`read.operationalStatePage`. A current point read may omit `context`; a stable
page always requires it. Passing a context to a point read allows exact
point/page comparison at the same boundary.

Canonical pages apply `context.context.view` and
`context.context.eligibility` before `LIMIT`. Operational-state reads require a
neutral context: strict/default `ReadView` and empty `SearchFilter`. Any other
context is rejected as `context_not_applicable`; FathomDB never suggests that
canonical eligibility predicates constrain arbitrary operational payloads.

## Ordering and query execution

Canonical pages select logical nodes of the requested kind and order by raw
SQLite text `logical_id ASC, write_cursor ASC`. The key is total because
`write_cursor` is unique for canonical rows. It is only an ordering coordinate,
not revision identity; the item carries `artifact_revision_id` separately.
The exclusive continuation predicate is:

```sql
logical_id > :last_logical_id OR
(logical_id = :last_logical_id AND write_cursor > :last_write_cursor)
```

Operational pages fix `collection_name` and order by `record_key ASC`, using
the existing `(collection_name, record_key)` primary key. Both execute
`LIMIT :limit_plus_one`; the extra row determines `next_cursor` and is not
returned. There is no OFFSET. A terminal or empty page has no next cursor.
Reusing one cursor is pure and returns identical bytes while its frozen state
remains valid.

Schema step 33 adds only:

```sql
CREATE INDEX canonical_nodes_kind_logical_cursor_idx
ON canonical_nodes(kind, logical_id, write_cursor)
WHERE logical_id IS NOT NULL;
```

Query-plan tests require this index for canonical pages, the operational-state
primary-key index for state point/pages, and no temp-order B-tree or
`operational_mutations` access. Exact-attribute eligibility may add indexed
semi-joins but must still execute before keyset truncation.

## Cursor and frozen-state binding

Stable pagination composes with Slice 35 rather than creating another snapshot
authority. The caller mints one `FrozenReadContextV1` and supplies it on every
page. Each page authenticates and validates that context, pins one reader
transaction, validates the frozen binding on that snapshot, and runs the page
query inside the same transaction.

The cursor is:

```text
fdbpg1.<lower-hex-canonical-payload>.<lower-hex-hmac-sha256>
```

HMAC-SHA256 uses the existing database-local read-context key with the distinct
ASCII domain `fathomdb.page-cursor.v1\0`. The canonical payload uses Slice 35's
length-prefixed scalar codec and contains, in order:

1. schema version;
2. database ID;
3. operation (`canonical_node_page` or `operational_state_page`);
4. SHA-256 of the operation selector (`kind` or `collection`);
5. SHA-256 of the complete canonical frozen context, including token;
6. fixed ordering version `1`;
7. original page limit; and
8. exclusive last key (`logical_id` plus `write_cursor`, or `record_key`).

The payload contains digests rather than selectors and no record content. The
complete cursor is at most 2 KiB. MAC comparison is constant-time. Separate
operation and selector domains prevent a canonical cursor from becoming a
state cursor or crossing collections/kinds.

The cursor does not duplicate write, dependency, visibility, or projection
generations: the frozen token already binds them. There is no expiry field in
0.8.25 because no retained lease exists. Relevant state drift is the Slice 35
`FrozenReadError::StateDrifted`; missing/corrupt bound state is
`StateUnavailable`. This slice does not promise reproduction after drift.

## Operational-state governance

Every state read validates in the same snapshot that `collection` exists in
`operational_collections` and has physical kind `latest_state`. A missing
collection and a registered append-only collection fail typed; only a missing
record key returns `None`. The stored `payload_json` is returned as exact text
without parsing, semantic interpretation, or truth claims.

Step 33 adds the three checked Slice 35 visibility triggers for each of
`operational_collections` and `operational_state`. The frozen-read manifest and
open-time trigger validation advance from 16 to 18 tables. State replacement,
collection registration/schema change, record excision, and raw fault changes
therefore invalidate a frozen continuation. No existing row is rewritten and
no cursor or page is persisted.

`latest_state` remains the existing collection-kind literal and consumer
vocabulary. The public surface is named `operational_state`; it neither creates
a semantic-latest table nor decides which fact is true.

## Failures and precedence

`PageError { reason, field_path }` is wrapped by `EngineError::Page` and maps to
Python `PageError` and TypeScript `FDB_PAGE`. Reasons are:

```text
unsupported_schema_version | invalid_page_limit | cursor_malformed |
cursor_too_large | cursor_authentication_failed | database_mismatch |
cursor_mismatch | context_not_applicable | collection_not_found |
collection_kind_mismatch
```

Precedence is request version/limit; cursor shape/version/size; authentication;
database; operation/selector/context/limit binding; frozen-context
authentication and current-state validation; operational collection
governance; then storage/query execution. Existing `FrozenReadError` is
preserved rather than translated. Errors never contain cursor/token bytes,
HMAC material, selector values, record keys, payloads, bodies, eligibility
values, or source identifiers. Field paths are RFC 6901 camel-case wire paths.

## Concurrency, lifecycle, and compatibility

The frozen context is conservative. Any relevant committed mutation before the
page transaction causes a typed whole-call failure. A write after the reader
snapshot is pinned linearizes after that page; it invalidates the next page.
Consequently a fully successful walk has one bound state and exact keyset
concatenation. FathomDB does not hold an SQLite transaction between calls and
does not claim replay after drift.

Canonical pages inherit validity, lifecycle, dependency-closure, erasure, and
eligibility fences. State pages inherit operational record excision and
collection governance. Cursor possession grants no additional visibility.
Existing `read_list`, `read_collection`, `read_mutations`, `search*`,
`NodeRecord`, `OpStoreRow`, and `SearchHit` are unchanged.

## TDD and verification matrix

RED tests are committed before implementation and use real SQLite databases:

| Test target | Required proof |
| --- | --- |
| `slice45_page_codec` | property round-trip; canonical fixture; tamper, noncanonical, size, version, database, operation, selector, context and limit mismatch; content-absence scan. |
| `slice45_canonical_pages` | limit bounds; empty/terminal/repeated cursor; active/history order; revision identity; anonymous exclusion; kind and every Slice 35 eligibility term before limit; exact concatenation. |
| `slice45_operational_state` | registered latest-state point/page; missing key; missing/wrong-kind collection; replacement; exact payload; point/page agreement; no mutation-log fallback. |
| `slice45_page_races` | concurrent insert, supersede, lifecycle, dependency closure, erasure, attribute/projection change, state replace/excise, collection change, close/reopen; bound result or exact whole-call failure. |
| `slice45_query_plans` | canonical composite index, operational PK, no OFFSET/temp-order/mutation-log scan. |
| binding suites | strict request/response/error wire, canonical fixture, Python/TypeScript parity, source-independent packaged smoke. |

Implementation order is codec/types and RED properties; schema/index/trigger
RED; reader-transaction page primitives; operational governance; facade and
bindings; compatibility and race tests; then documentation. Tests remain
frozen during GREEN corrections.

Run focused Rust/schema, fast, heavy, all, applicable all-feature/operator,
fresh-wheel Python, npm/native, locally packed consumers, and Windows
CPU/native routes. A 10k/50k deterministic page workload records p50/p95/p99,
throughput, errors, query plans, RSS, database bytes, and token/cursor size.
Matched cells isolate mint, first-page, continuation, whole-walk, equivalent
unfrozen canonical read, and frozen/unfrozen state point costs. They use three
process-cold and five steady repetitions with at least 1,000 measured steady
operations, alternating treatment order and pinned input, build, SQLite, host,
CPU-affinity, reset, and warm-up state.

The preregistered materiality boundary is a p95 increase of both more than 10%
and more than 0.25 ms, or peak-RSS increase of both more than 5% and more than
8 MiB. Raw repetitions and confidence intervals are retained regardless of
outcome. A material effect requires causal analysis and explicit reviewed
disposition before closure; it cannot weaken frozen validation, eligibility,
or cursor authentication. Release-wide comparative policy remains in Slice
75. CUDA, CE, GPU allocation, and live-model routes are N/A because this slice
executes SQLite reads only.

The design becomes READY only after an independent review has no unresolved
P1/P2 implementation-shaping finding. Implementation then requires committed
RED/GREEN chronology, independent code review, separate verification, exact
evidence references, and a Slice 45 status record.
