---
title: FathomDB 0.8.28 Slice 50 — design
status: Astra design approved; Unit S implemented; qualification in progress
target_release: 0.8.28
observed_on: 2026-11-01
---

# Slice 50 design (`DRAIN-SCAN-BOUND`)

The current scope is scan Unit S. The [plan](plan.md) enumerates changes
since the draft. The provider contract is already implemented; historical
D50-1/D50-2 designs remain in git history and are not selected. Requirements
are in [requirements.md](requirements.md).

The production change extracts the two existing SQL statements into private
builders and bounds each compound arm. The edge fragment and every caller's
eligibility, pagination, in-flight and watermark logic stay unchanged.

## 1. Unit S: bounded scans

### 1.1 The SQL today

Scheduler scan (`next_pending_projection_jobs`; `?1` page cursor, `?2`
edge validity instant):

```sql
SELECT pending.write_cursor, pending.kind, pending.body, current.generation_id FROM (
    SELECT canonical_nodes.write_cursor AS write_cursor,
           canonical_nodes.kind AS kind,
           canonical_nodes.body AS body
    FROM canonical_nodes
    JOIN _fathomdb_vector_kinds
      ON _fathomdb_vector_kinds.kind = canonical_nodes.kind
    LEFT JOIN _fathomdb_projection_terminal
      ON _fathomdb_projection_terminal.write_cursor = canonical_nodes.write_cursor
    WHERE canonical_nodes.write_cursor > ?1
      AND _fathomdb_projection_terminal.write_cursor IS NULL

    UNION ALL

    SELECT ce.write_cursor AS write_cursor,
           'edge_fact' AS kind,
           ce.body AS body
    {pending_edge_projection_from_where(2)}
      AND ce.write_cursor > ?1
) AS pending
CROSS JOIN _fathomdb_projection_generation_current AS current
WHERE current.singleton=1
ORDER BY pending.write_cursor
LIMIT 256
```

Candidate page (`pending_projection_candidate_page`; `?1` after, `?2`
through or `NULL`, `?3` edge validity instant):

```sql
SELECT pending.write_cursor,pending.kind FROM (
  SELECT n.write_cursor,n.kind
  FROM canonical_nodes n
  JOIN _fathomdb_vector_kinds vk ON vk.kind=n.kind
  LEFT JOIN _fathomdb_projection_terminal pt ON pt.write_cursor=n.write_cursor
  WHERE n.write_cursor>?1 AND (?2 IS NULL OR n.write_cursor<=?2)
    AND pt.write_cursor IS NULL
  UNION ALL
  SELECT ce.write_cursor,'edge_fact'
  {pending_edge_projection_from_where(3)}
    AND ce.write_cursor>?1 AND (?2 IS NULL OR ce.write_cursor<=?2)
) AS pending ORDER BY pending.write_cursor LIMIT 256
```

`pending_edge_projection_from_where(i)` expands to `FROM canonical_edges
ce JOIN _fathomdb_vector_kinds ON kind = 'edge_fact' LEFT JOIN
_fathomdb_projection_terminal pt …`. Its `WHERE` holds `body IS NOT NULL`,
`superseded_at IS NULL`, the `t_invalid` and read-eligibility predicates
(`edge_validity_sql`), and `pt.write_cursor IS NULL`.

Who runs them:

- **Scheduler scan.** The dispatcher, through `next_pending_projection_jobs`.
  It runs about once per worker batch, and its page loop runs until it
  has `max_jobs` eligible jobs.
- **Candidate page.** Through `pending_projection_members_in_range` and
  `first_pending_projection_member_in_range`, it serves:
  - `connection_has_pending_projection_work`: `drain`'s probe, run on
    every idle wake of `wait_for_idle`, plus `derive_dense_readiness`;
  - `pending_embedding_work`: readiness counts;
  - `lowest_pending_projection_member_at_or_below`: the
    below-watermark check;
  - `database_has_pending_projection_work`: the open-time wake.

### 1.2 The SQL after

Each arm is wrapped in its own ordered, limited subquery, using the same
`PROJECTION_CANDIDATE_PAGE` constant as the outer `LIMIT`. SQLite does
not allow `ORDER BY` or `LIMIT` on a bare compound member, so each arm
becomes `SELECT * FROM (<arm> ORDER BY <arm's write_cursor> LIMIT 256)`:

```sql
SELECT pending.write_cursor, pending.kind, pending.body, current.generation_id FROM (
    SELECT * FROM (
        SELECT canonical_nodes.write_cursor AS write_cursor, … same node arm …
        ORDER BY canonical_nodes.write_cursor LIMIT 256)
    UNION ALL
    SELECT * FROM (
        SELECT ce.write_cursor AS write_cursor, … same edge arm …
        ORDER BY ce.write_cursor LIMIT 256)
) AS pending
CROSS JOIN _fathomdb_projection_generation_current AS current
WHERE current.singleton=1
ORDER BY pending.write_cursor
LIMIT 256
```

The candidate page changes the same way, with `ORDER BY n.write_cursor`
and `ORDER BY ce.write_cursor`. The compound's column names still come
from its left-most member (`write_cursor`, `kind`, `body`), so the outer
references do not change. This is the prototype's shape
(`855d29155`, `union_arm`). The production change has no env knob and
no `_for` variant. The text is built once per call and goes through
`prepare_cached`, as today.

A comment at the builder states the plan requirement (REQ50-S6). Each arm
must stream from its `write_cursor` index and stop after one page. The
outer sort then sees at most two pages. Removing the per-arm bound makes
the outer sort consume the whole backlog.

### 1.3 Plans on SQLite 3.54.1

Scratch probe, 2026-10-31: a `git archive` export of `544bf74f8` with a
throwaway unit test, nothing committed. It ran `EXPLAIN QUERY PLAN` on a
store with 1,000 pending `doc` nodes and a body edge every seventh row.
The read-eligibility subqueries are omitted below.

Today (scheduler scan; the candidate page has the same shape):

```text
2|0|CO-ROUTINE pending
3|2|COMPOUND QUERY
4|3|LEFT-MOST SUBQUERY
9|4|SEARCH canonical_nodes USING INDEX canonical_nodes_write_cursor_idx (write_cursor>?)
15|4|SEARCH _fathomdb_vector_kinds USING COVERING INDEX sqlite_autoindex__fathomdb_vector_kinds_1 (kind=?)
19|4|SEARCH _fathomdb_projection_terminal USING INTEGER PRIMARY KEY (rowid=?) LEFT-JOIN
34|3|UNION ALL
39|34|SEARCH _fathomdb_vector_kinds USING COVERING INDEX sqlite_autoindex__fathomdb_vector_kinds_1 (kind=?)
43|34|SEARCH ce USING INDEX canonical_edges_write_cursor_idx (write_cursor>?)
213|34|SEARCH pt USING INTEGER PRIMARY KEY (rowid=?) LEFT-JOIN
232|0|SCAN pending
235|0|SEARCH current USING INTEGER PRIMARY KEY (rowid=?)
250|0|USE TEMP B-TREE FOR ORDER BY
```

After:

```text
2|0|CO-ROUTINE pending
3|2|COMPOUND QUERY
4|3|LEFT-MOST SUBQUERY
6|4|CO-ROUTINE (subquery-1)
13|6|SEARCH canonical_nodes USING INDEX canonical_nodes_write_cursor_idx (write_cursor>?)
19|6|SEARCH _fathomdb_vector_kinds USING COVERING INDEX sqlite_autoindex__fathomdb_vector_kinds_1 (kind=?)
23|6|SEARCH _fathomdb_projection_terminal USING INTEGER PRIMARY KEY (rowid=?) LEFT-JOIN
40|4|SCAN (subquery-1)
48|3|UNION ALL
50|48|CO-ROUTINE (subquery-6)
57|50|SEARCH _fathomdb_vector_kinds USING COVERING INDEX sqlite_autoindex__fathomdb_vector_kinds_1 (kind=?)
61|50|SEARCH ce USING INDEX canonical_edges_write_cursor_idx (write_cursor>?)
231|50|SEARCH pt USING INTEGER PRIMARY KEY (rowid=?) LEFT-JOIN
248|48|SCAN (subquery-6)
260|0|SCAN pending
263|0|SEARCH current USING INTEGER PRIMARY KEY (rowid=?)
278|0|USE TEMP B-TREE FOR ORDER BY
```

What the plans show:

- **The arms already stream in index order today.** Neither arm has its
  own sort, and neither plan has a sort below the top level.
- **The defect is the missing bound.** The top-level temp B-tree pulls
  every row out of `CO-ROUTINE pending` before `LIMIT` applies.
- **After the fix.** Each arm is a `CO-ROUTINE (subquery-N)` under its
  compound member. Its index walk stops at 256 matching rows, and the edge arm's
  one-row `_fathomdb_vector_kinds` outer loop keeps `ce` in index order.
  The top-level sort sees at most 512 rows.
- **What a plan test can check** (AC50-01):
  - each compound member is a bounded co-routine;
  - no `USE TEMP B-TREE` has a parent other than 0, because an arm-level
    sort would read the whole arm before its `LIMIT`;
  - each arm reads its `write_cursor` index with a range
    (`write_cursor>?`) and there is no `SCAN canonical_nodes` or
    `SCAN ce`.

  `EXPLAIN QUERY PLAN` does not print `LIMIT`. The cost test (AC50-02)
  is therefore the behavioural check, and the plan test pins the shape.

Cost in the same probe (VM steps for one page from cursor 0, debug
build):

| Store | Scheduler today | Scheduler after | Candidate today | Candidate after |
| --- | --- | --- | --- | --- |
| 1,000 nodes, 143 edges | 38,110 | 20,825 | 34,736 | 18,540 |
| 8,000 nodes, 1,143 edges | 287,110 | 28,394 | 267,736 | 25,770 |
| 8,000 nodes, no edges | 218,334 | 11,048 | 201,310 | 9,256 |

After the fix the cost still rises from 1,000 to 8,000 nodes, because at
1,000 the edge arm has only 143 rows (fewer than a page). AC50-02
therefore compares stores where both arms hold at least a page at the
small size.

### 1.4 Ordering and batch contents are preserved

Let `A` be the node-arm rows and `B` the edge-arm rows that satisfy the
arm predicates for a given `(after, through, effective_at)`. Let `top(X)`
be the first 256 rows of `X` by `write_cursor`. Today a call returns
`top(A ∪ B)`. After the fix it returns `top(top(A) ∪ top(B))`.

- Take `x` in `top(A ∪ B)` with `x` in `A`. Fewer than 256 rows of
  `A ∪ B` precede `x`, so fewer than 256 rows of `A` precede it, and
  `x` is in `top(A)`. The same holds for `B`. So
  `top(A ∪ B) ⊆ top(A) ∪ top(B) ⊆ A ∪ B`.
- The top 256 of a subset that contains the top 256 of the whole set is
  that same set. Both forms sort by the same key, so the rows and their
  order are equal.
- **Precondition: `write_cursor` is unique across nodes and edges.**
  - The engine's allocator gives every row its own cursor, from
    `load_next_cursor` over every cursor-bearing table, and every row of
    a batch gets a separate cursor.
  - `dense_member_kind_at` treats a node and an edge sharing a cursor as
    corruption.
  - On a store that broke the invariant, the shipped query's order among
    tied rows at a page edge is also unspecified. "Identical" is defined
    on valid stores. The property test generates unique cursors.
- **The rest of the query is unaffected.** The generation `CROSS JOIN`
  sits outside the arms and yields one row or none, so it multiplies the
  same 256-row prefix by the same singleton. The `through` bound and the
  validity instant are in the arm `WHERE`, so they apply before each
  arm's `LIMIT`.
- **The callers see the same thing.** The callers' page loops advance
  `after_cursor` to the last cursor of each page and stop on a short
  page. Identical pages give identical loop behaviour, identical jobs in
  identical order, and identical readiness answers. The scheduler's
  in-flight skip and `dense_member_kind_at` filter run in Rust after the
  page and are unchanged.

The scratch probe checked this on 496 page positions across the three
stores, with and without a `through` bound: every page was identical.

### 1.5 Cost after the fix

One scan call costs O(page plus rejected rows). SQL predicates may reject arbitrarily many index entries before collecting
a page; Rust eligibility checks may reject further returned candidates.
The bound limits accepted rows entering the outer sort to 512. It removes
the full-backlog sort on every page, but does not assert a bound on rejected
index entries or prove linear whole-drain cost for every eligibility layout.

`pending_embedding_work` still pages over every pending member, because
it counts them. Each page bounds accepted arm output while still visiting index entries
rejected by its predicates. No memory is added, and the temp B-tree shrinks to two pages.

## 3. Test hooks

One new hook on `Engine`, gated by `test-hooks`, `#[doc(hidden)]`. It is
the pattern of `projection_generation_status_query_plans_for_test` and
`slice45_page_query_plans_for_test`.

```rust
#[cfg(feature = "test-hooks")]
pub enum ProjectionScanForTest { Scheduler, CandidatePage }

#[cfg(feature = "test-hooks")]
pub struct ProjectionScanPageForTest {
    pub sql: String,               // the production SQL text
    pub rows: Vec<(u64, String)>,  // (write_cursor, kind) in returned order
    pub vm_steps: u64,             // SQLITE_STMTSTATUS_VM_STEP for this one call
    pub plan: Vec<String>,         // EXPLAIN QUERY PLAN as "id|parent|detail"
}

pub fn projection_scan_page_for_test(
    &self,
    scan: ProjectionScanForTest,
    after_cursor: u64,
    through_cursor: Option<u64>, // CandidatePage only
    effective_at: i64,
) -> Result<ProjectionScanPageForTest, EngineError>;
```

- It runs `scheduler_scan_sql()` or `candidate_page_sql()`, the same
  functions production calls. It uses a fresh, uncached statement on the
  engine connection, so `vm_steps` counts exactly one execution.
- `effective_at` is a parameter, so a test compares two forms at one
  instant.
- It exposes no job bodies, only `(cursor, kind)`.
- Placement: the hook lives in `projection_runtime.rs`, beside
  `set_projection_scheduler_frozen_for_test`. The SQL builders stay
  `pub(crate)` in `projection_worker.rs`. The lint verb (module
  boundaries) runs after the extraction, because Slice 42 met a
  module-boundary classification failure for a new test module.

The property test (AC50-03) writes rows on a second raw connection to
an engine-created store while the dispatcher is frozen. It then reads
pages through the hook and through the reference SQL, which is the
shipped text copied verbatim into the test file, with the edge fragment
expanded. That is the "bounded scan against the shipped scan on the same
state" check that the master plan's quality section names.

## 4. Interactions

- **Slice 60 (format 35).** It changes the vec0 `chunk_size` and makes
  `search_index` rowid equal `write_cursor`. Neither query reads
  `vector_default` or `search_index`. This slice changes no schema and no
  rowid, so it is independent of Slice 60 in both landing orders. Slice
  60 can rerun this slice's scaling tool to get the CS64 numbers (the
  prototype's 5.7 s at 100k).
- **Slice 70 (eligibility in vec0).** It adds vec0 metadata columns
  written in `projection_commit.rs` and kept current at lifecycle
  writes, and it removes the search-time eligibility scan. The pending
  scans' predicates and the Rust `dense_member_kind_at` check are
  untouched by Slice 70's plan. If Slice 70 changes
  `pending_edge_projection_from_where`, the per-arm wrapper still
  applies, because it wraps whatever the arm is.
- **Slices 80 and 90.** No overlap.
- **Readiness invariants** (R-20-DR, Slice 20 fix-1). The scheduler and
  the probe still share `pending_edge_projection_from_where`. The
  wrapper is added to both builders the same way, and AC50-03 covers
  both.

## 5. Implementation and review

Extract unchanged SQL and add the test-only hook first. Baseline-equivalence
checks establish the oracle; then plan and VM-step tests must fail before
per-arm bounds are added. The test reference is a frozen copy of baseline SQL,
not a rewritten expected query derived from the new implementation. One
behavioral RED/GREEN pair covers the same bound in both scan consumers.

Astra reviews this design before implementation and the final diff after
implementation. A separate verification agent checks the final result and
runs the planned tests. The scaling probe uses the existing public test APIs
on baseline `630af0bfc` and head, without a production runtime toggle.
