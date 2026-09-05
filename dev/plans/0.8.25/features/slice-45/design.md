---
title: 0.8.25 Slice 45 — minimal pagination and operational-state design
status: DRAFT_REVIEW_FIX_2
design_version: 6
review_cycle: 2
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
| S45-R1 Stable canonical pages | S45-AC1 concatenating all pages equals one reference query in `write_cursor` order, with no duplicate or omission; schema and open validation refuse duplicate page keys. |
| S45-R2 Frozen continuation | S45-AC2 every page requires one valid Slice 35 frozen context; unchanged state succeeds across restart, while relevant drift refuses before returning any item. |
| S45-R3 Bound opaque cursor | S45-AC3 tamper, another database, operation/selector/context/limit mismatch, noncanonical encoding, and unsupported version fail typed; cursor bytes contain no record, filter value, source, logical ID, record key, or other caller text. |
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

OperationalStateRecordV1 {
  schema_version: 1,
  collection: string,
  record_key: string,
  payload: string,
  schema_id: string?,
  write_cursor: u64
}
```

Canonical pages return `PageV1<NodeRecord>`. Reusing the existing item keeps the
new surface minimal and the performance comparison shape-equivalent. Slice 45
does not expose the internal deterministic legacy artifact-revision identity;
Slice 50 remains its first opt-in public resolver. Anonymous content-ID nodes
cannot satisfy `NodeRecord` and are excluded before the limit. Edges remain
available through governed graph reads and are not silently folded into this
node page.

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
) -> Result<PageV1<NodeRecord>, EngineError>

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

Python places `canonical_page`, `operational_state`, and
`operational_state_page` in the existing `fathomdb.read` namespace and passes
the Engine explicitly. TypeScript exposes the corresponding
`read.canonicalPage`, `read.operationalState`, and
`read.operationalStatePage`. A current point read may omit `context`; a stable
page always requires it. Passing a context to a point read allows exact
point/page comparison at the same boundary.

Canonical pages apply `context.context.view` and
`context.context.eligibility` before `LIMIT`. Operational-state reads require a
neutral context: all three `ReadView` relaxation flags are false and
`SearchFilter` is empty. The mint-resolved `valid_as_of` value is permitted but
unused. Any other context is rejected as `context_not_applicable`; FathomDB
never suggests that canonical eligibility predicates constrain arbitrary
operational payloads.

## Ordering and query execution

Canonical pages select logical nodes of the requested kind and order by
`write_cursor ASC`. The exclusive continuation predicate is:

```sql
write_cursor > :last_write_cursor
```

`write_cursor` is a persisted, Engine-minted total ordering coordinate for
canonical rows, not record identity; `NodeRecord.logical_id` remains the item
identity. Operational pages fix `collection_name` and use the same
`write_cursor ASC` order. Replacement gives the current state row a new cursor,
and the frozen context prevents that move from being mixed into an existing
walk. Both execute `LIMIT :limit_plus_one`; the extra row determines
`next_cursor` and is not returned. There is no OFFSET. A terminal or empty page
has no next cursor. Reusing one cursor is pure and returns identical items,
order, and deterministic cursor bytes while its frozen state remains valid.

Schema step 33 is one atomic additive migration. It adds these two page
indexes:

```sql
CREATE UNIQUE INDEX canonical_nodes_kind_cursor_page_idx
ON canonical_nodes(kind, write_cursor)
WHERE logical_id IS NOT NULL;

CREATE UNIQUE INDEX operational_state_collection_cursor_page_idx
ON operational_state(collection_name, write_cursor);
```

It also adds the Slice 35 insert/update/delete visibility triggers for
`operational_collections` (`oc`) and `operational_state` (`os`), then performs
one checked increment of `_fathomdb_read_visibility_state.generation`. The
increment invalidates every token minted under the narrower step-32 manifest;
at `i64::MAX` it aborts and rolls back the whole migration with
`read visibility generation exhausted`. Frozen open validation uses exactly 14
tables at schema 31, 16 at schema 32, and 18 at schema 33 or later.

The Engine already allocates `write_cursor` from one global monotonic sequence,
but the legacy tables did not encode that invariant. The unique page indexes
make the per-selector continuation coordinate a database-enforced total order.
Migration refuses and rolls back if legacy logical nodes or operational-state
rows contain duplicate `(selector, write_cursor)` keys. Post-migration open
validation independently detects either duplicate-key condition so a database
whose index or rows were changed outside the Engine fails closed rather than
serving an omitting walk.

Query-plan tests require the canonical page index, the operational primary key
for point reads, the operational page index for pages, and no temp-order B-tree
or `operational_mutations` access. Exact-attribute eligibility may add indexed
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
8. exclusive last `write_cursor`.

The payload contains digests rather than selectors and its sole page coordinate
is the already public unsigned write cursor. It contains no record content,
logical ID, record key, kind, collection, source, or filter value. The complete
cursor is at most 2 KiB. MAC comparison is constant-time. Separate operation
and selector domains prevent a canonical cursor from becoming a state cursor
or crossing collections/kinds. Authenticated encryption is unnecessary because
no caller text or non-public coordinate remains in the payload.

The cursor does not duplicate write, dependency, visibility, or projection
generations: the frozen token already binds them. There is no expiry field in
0.8.25 because no retained lease exists. Relevant state drift is the Slice 35
`FrozenReadError::StateDrifted`; missing/corrupt bound state is
`StateUnavailable`. This slice does not promise reproduction after drift.

## Operational-state governance

Every state read validates in the same snapshot that `collection` exists in
`operational_collections`, has physical kind `latest_state`, and has supported
`format_version = 1`. Missing, wrong-kind, and unsupported-format collections
fail typed; only a missing record key returns `None`. The stored `payload_json`
is returned as exact text without parsing, semantic interpretation, or truth
claims.

The step-33 triggers make state replacement, collection registration/schema
change, record excision, and raw fault changes invalidate a frozen
continuation. No existing row is rewritten and no cursor or page is persisted.
A migration fixture mints a schema-32 token, mutates operational state through
a raw connection, upgrades through step 33, and proves the old token refuses;
the migration-generation increment is the authority even when the raw
pre-trigger mutation preserved `write_cursor`.

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
collection_kind_mismatch | collection_format_unsupported
```

Exact precedence is bounded page-request parse/version/limit; cursor
shape/version/size; cursor authentication and database binding; frozen-token
parse/authentication/database binding; cursor operation/selector/context/limit
comparison; frozen snapshot validation; operational collection governance; then
storage/query execution. Existing `FrozenReadError` is preserved rather than
translated. Errors never contain cursor/token bytes, HMAC material, selector
values, record keys, payloads, bodies, eligibility values, or source
identifiers.

| Reason | RFC 6901 field path |
| --- | --- |
| `unsupported_schema_version` | `/schemaVersion` |
| `invalid_page_limit` | `/limit` |
| `cursor_malformed`, `cursor_too_large`, `cursor_authentication_failed`, `database_mismatch`, `cursor_mismatch` | `/cursor` |
| `context_not_applicable` | `/context` |
| `collection_not_found`, `collection_kind_mismatch`, `collection_format_unsupported` | `/collection` |

Combined-malformation fixtures pin this precedence and the same reason/path in
Rust, Python, and TypeScript.

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
| `slice45_page_codec` | property round-trip; canonical fixture; tamper, noncanonical, size, version, database, operation, selector, context and limit mismatch; caller-text absence scan. |
| `slice45_canonical_pages` | limit bounds; empty/terminal/repeated cursor; active/history write-cursor order; anonymous exclusion; kind and every Slice 35 eligibility term before limit; exact concatenation with existing `NodeRecord` shape. |
| `slice45_operational_state` | registered format-v1 latest-state point/page; missing key; missing/wrong-kind/unsupported-format collection; replacement; exact payload; point/page agreement; no mutation-log fallback. |
| `slice45_page_races` | concurrent insert, supersede, lifecycle, dependency closure, erasure, attribute/projection change, state replace/excise, collection change, close/reopen; bound result or exact whole-call failure. |
| `slice45_query_plans` | canonical composite index, operational PK, no OFFSET/temp-order/mutation-log scan. |
| `step33_pagination` | two unique indexes, six exact triggers, 14/16/18 manifest branches, one cutover increment, exhaustion rollback, pre-upgrade duplicate-key migration refusal, post-upgrade duplicate-key open refusal, and schema-32-token/raw-state-mutation/upgrade refusal. |
| binding suites | strict request/response/error wire, canonical fixture, Python/TypeScript parity, source-independent packaged smoke. |

Implementation order is codec/types and RED properties; schema/index/trigger
RED; reader-transaction page primitives; operational governance; facade and
bindings; compatibility and race tests; then documentation. Tests remain
frozen during GREEN corrections.

Run focused Rust/schema, fast, heavy, all, applicable all-feature/operator,
fresh-wheel Python, npm/native, locally packed consumers, and Windows
CPU/native routes. The benchmark uses checked-in deterministic generators for
10k and 50k logical nodes (`kind=slice45_doc`, insertion-ordered logical IDs,
256-byte UTF-8 JSON bodies) and the same number of `slice45_state` rows
(fixed-width keys and 256-byte JSON payloads). It records input, runner,
manifest, raw NDJSON, result JSON/Markdown, candidate, toolchain, SQLite, and
environment hashes under `dev/plans/runs/0.8.25-slice-45-pagination/`.

Primary causal cells are:

1. mint one context plus the first canonical page versus the same first page
   using a pre-minted context, with mint validation, snapshot,
   binding/terminal-scan, token-codec, and page-query stages reported
   separately;
2. the exact canonical page SQL without frozen/cursor work versus the same
   `PageV1<NodeRecord>` query with one pre-minted context and no cursor;
3. first page versus continuation, with cursor codec and frozen validation
   stage timings reported separately; and
4. operational-state point read without versus with one pre-minted context.

Empty eligibility/default validity is used in the matched cells; a separate
non-gating eligibility cell proves filters but is not pooled into overhead.
The existing public `read_list_filter` and a complete page walk are reported as
separate operational observations, not causal baselines. Frozen validation
records projection-state and terminal row counts plus parse, authentication,
binding, and query stage times so its current O(number-of-terminal-rows) digest
scan cannot be misattributed to keyset pagination.

Each primary steady cell runs ten independent paired processes per scale and
at least 1,000 operations per process; treatment order alternates. Three
additional process-cold repetitions measure open and first use. Peak RSS uses
five fresh processes per arm and is never inferred from two phases in one
process. A fixed-seed 10,000-draw paired percentile bootstrap over
repetition-level p95 deltas reports the two-sided 95% interval. The
preregistered materiality boundary is a median paired p95 increase of both more
than 10% and more than 0.25 ms in any primary cell, or median peak-RSS increase
of both more than 5% and more than 8 MiB.

Raw repetitions and intervals are retained regardless of outcome. A material
effect—including one attributable to frozen binding validation—requires causal
analysis and explicit reviewed disposition before closure; it cannot weaken
frozen validation, eligibility, or cursor authentication. Release-wide
comparative policy remains in Slice 75. CUDA, CE, GPU allocation, and
live-model routes are N/A because this slice executes SQLite reads only.

The design becomes READY only after an independent review has no unresolved
P1/P2 implementation-shaping finding. Implementation then requires committed
RED/GREEN chronology, independent code review, separate verification, exact
evidence references, and a Slice 45 status record.
