---
title: FathomDB 0.8.28 Slice 50 — design
status: PLANNED (phase 1, 2026-10-31; awaiting Fable design review and owner rulings D50-1, D50-2)
target_release: 0.8.28
observed_on: 2026-10-31
---

# Slice 50 design (`DRAIN-SCAN-BOUND`)

Unit S (§ 1) bounds the two pending-work scans. Unit D (§ 2) makes
`drain` prompt on a dense-disabled store. § 3 covers the test hooks, § 4
the interactions with other slices, and § 5 the commit sequence. Line
numbers refer to the slice base `544bf74f8`. Requirements and evidence
are in [requirements.md](requirements.md).

## 0. What exists today and what is new

| Piece | Today | This slice |
| --- | --- | --- |
| Scheduler scan SQL | inline in `next_pending_projection_jobs` (`projection_worker.rs` 1033–1062) | moved into `scheduler_scan_sql()` (text unchanged), then each arm bounded |
| Candidate-page SQL | inline in `pending_projection_candidate_page` (898–916) | moved into `candidate_page_sql()` (text unchanged), then each arm bounded |
| Edge-arm fragment | `pending_edge_projection_from_where` (882) | unchanged |
| Page loops, in-flight skip, `dense_member_kind_at`, below-watermark rewind | 927–1134 | unchanged |
| `write_cursor` indexes | `canonical_nodes_write_cursor_idx`, `canonical_edges_write_cursor_idx` (`fathomdb-schema` lib.rs 955–956) | unchanged; the arms already use them |
| `drain` | `runtime_lifecycle.rs` 57–95 | new dense-disabled branch (§ 2) |
| Internal settles | `drain_for_non_embedding_mutation` (102), `freeze_projection_for_erasure` (`erasure.rs` 550) | changed only if D50-2 approves (§ 2.4) |
| Scan test hook | none on this branch. The experiment branch's `projection_scan_query_plans_for_test` and page tracer are not here | new `projection_scan_page_for_test` (§ 3) |
| Scaling probe | none on this branch (`exp_ac013_seed_drain.rs` lives on the experiment branch) | new `slice50_drain_scaling.rs` and driver (test-plan § 4) |

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
  compound member. Its index walk stops at 256 rows, and the edge arm's
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

One scan call costs O(page plus rejected rows). Rejected rows are pending
rows that the arm predicates pass but the Rust eligibility check drops,
for example superseded nodes, which the node arm does not filter. That is
the same set the shipped loop pages over, so the total work over a drain
is O(N + rejected), not O(N²).

`pending_embedding_work` still pages over every pending member, because
it counts them. Each page now costs one page instead of the whole
backlog. No memory is added, and the temp B-tree shrinks to two pages.

## 2. Unit D: dense-disabled drain

### 2.1 The rule

A session is **dense-disabled** when it holds a runtime that the
equivalence guard refused:

```rust
self.runtime_embedder.is_some() && self.dense_disabled.load(Acquire)
```

This is `!usable_dense_runtime()` minus the absent case.

- `dense_disabled` is set once at open (`open.rs` 874, from the probe)
  and never stored again.
- The projection runtime is then built without an embedder (`open.rs`
  927–928), so no dispatcher scan returns jobs and no worker can embed.
- The engine holds the database lock, so no other process can embed the
  rows either.
- Waiting can therefore never clear the pending rows in this session.

### 2.2 The change in `drain`

`drain` gains one branch after the absent-runtime branch (63–79), before
`wait_for_idle` (81):

```rust
if self.runtime_embedder.is_some() && self.dense_disabled.load(Ordering::Acquire) {
    // A refused runtime is never handed to the scheduler, so no worker of
    // this session can embed a pending row: waiting cannot clear one.
    let pending = {
        let guard = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = guard.as_ref().ok_or(EngineError::Closing)?;
        connection_has_pending_projection_work(connection).map_err(|_| EngineError::Storage)?
    };
    if !self.projection_runtime.wait_for_workers_idle(timeout_ms) {
        return Err(EngineError::Scheduler);
    }
    return if pending { /* D50-1 */ } else { Ok(()) };
}
```

- The probe is the same predicate `wait_for_idle` and readiness use, so
  the branch cannot disagree with them about what is pending.
  `connection_has_pending_projection_work` stops at the first eligible
  row. Unit S makes that O(page).
- `wait_for_workers_idle` returns at once on such a session. It is kept
  so the branch never returns while a worker could still be committing.
- The absent-runtime branch and the usable-runtime path are unchanged
  (REQ50-D2).

**Callers that see the change.**

- Public `drain` in Rust, Python and TypeScript. The bindings call
  `Engine::drain` and map its error, so they need no change unless
  D50-1 picks C.
- `fathomdb-mcp` (on `dev/0.8.29`) calls `drain` on idle close, store
  switch and shutdown. It logs a failed drain and closes anyway
  (`registry.rs` 845–853 there), so its close no longer stalls.
- The internal settles (§ 2.4).
- Readiness, status and the pending counters do not call `drain`, so
  they are unchanged (REQ50-D4).

### 2.3 Owner ruling D50-1: what `drain` returns

On a dense-disabled store with pending vector rows, `drain` returns
promptly under every option. The open question is what it returns.

| Option | Result | Public change | Existing records |
| --- | --- | --- | --- |
| **A** (recommended) | `Err(EngineError::Scheduler)` at once | latency only; the result is what callers get today after `timeout_ms` | `slice30_embedding_readiness.rs` 207 still passes; the interface wording "a bounded `drain` may return `EngineError::Scheduler`" stays true and gains "immediately" |
| B | `Ok(())` at once ("not drainable" rows are excluded) | result changes from `Err` to `Ok` | contradicts the Slice 30 test at 207 (a test change needs the owner) and the interface's "retains ordinary bounded behavior"; `Ok` would no longer mean the rows are done, even though readiness says `Deferred` |
| C | a new typed error (for example `EmbedderRefused`) at once | new public variant in Rust, Python, TypeScript and the error docs | needs an interface-doc and errors-doc change; widest blast radius |

**Recommendation: A.**

- It is the smallest change that ends the stall.
- It keeps "public results identical" in the strict sense: same `Err`,
  without the wait.
- It keeps the Slice 30 contract that refused-runtime work is
  operational deferral, not success.
- The todo's "treat pending vector rows as not drainable" reads as "do
  not wait for them". It does not say "report success".

B would make `drain`'s `Ok` mean "nothing more will happen" rather than
"done", and C needs binding work for little gain over A plus
`read_embedding_readiness`. Under A, the MCP close path keeps logging
one warning per close of such a store, now without the stall.

### 2.4 Owner ruling D50-2: the internal settles

Today, on a refused store with pending vector rows, the callers below
wait 30 s and fail with `Err(Scheduler)`. That was measured for
`transition`. Under D50-1 option A they fail at once instead. Under
option B they would proceed. For an absent runtime they already proceed,
because no worker can commit.

| Caller | Today (refused, pending) | Proposed if approved |
| --- | --- | --- |
| `transition` (`record_lifecycle.rs` 214), `configure_projections` (`projection_registry.rs` 1543), lifecycle actuation (`actuation.rs` 1988): all via `drain_for_non_embedding_mutation` | 30 s, then `Err(Scheduler)`; the mutation fails | treat refused as absent: `wait_for_workers_idle`, then proceed |
| erasure: `purge` and `erase_source` / `excise_source` (`freeze_projection_for_erasure`, `erasure.rs` 550, called at 667 and 994) | 30 s, then `Err(Scheduler)`; the erasure fails | the `no_configured_embedder` path: freeze, `wait_for_workers_idle`, then the erasure transaction removes the rows |
| operator rebuild (`run_rebuild`, `projection_rebuild.rs` 70) | 30 s, then `Err(Scheduler)` | unchanged result, now prompt (option A); a vector rebuild with a refused runtime cannot re-embed, so refusing is the safer outcome |

The change for the first two rows is a check of `self.dense_disabled`
next to the existing `EmbedderRequired` handling. It does not inspect
which `Scheduler` error `drain` returned.

**Recommendation: approve, in this slice.** The cause is the same as
todo 289, a deferral treated as a wait. The fix sits next to it, and
erasure that cannot run on a refused store is a reliability gap. Each
caller family gets its own RED (AC50-11). If the owner declines, Unit D
stops at § 2.2, and a todo records the internal callers.

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
  boundaries) runs on the H-GREEN commit, because Slice 42 met a
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

## 5. Commit sequence

Each RED fails at its own sha for the stated reason, with "RED
evidence:" in its body. The RED shas are audited in `git archive`
exports.

| # | Kind | What | Fails at RED because |
| --- | --- | --- | --- |
| 1 | H-RED | test calling `projection_scan_page_for_test` | compile error: no such method (test infrastructure) |
| 2 | H-GREEN | SQL moved into `scheduler_scan_sql()` and `candidate_page_sql()` (text unchanged); the hook | — |
| 3 | S-RED | AC50-01 plan test, AC50-02 cost test; AC50-03 property test and AC50-05 ordering cases as guards (see test-plan § 3.3) | plan: no bounded arm co-routine; cost: ratio ≈ 7–8 > 1.5 |
| 4 | S-GREEN | per-arm `ORDER BY write_cursor LIMIT` in both builders; the plan-requirement comment | — |
| 5 | tool | scaling probe, driver, analysis and its unit tests (test-plan § 4) | (tool; not a behaviour change) |
| 6 | D-RED | AC50-09 | `drain(5_000)` takes about 5 s |
| 7 | D-GREEN | § 2.2 with the D50-1 result | — |
| 8–9 | D2-RED/GREEN × caller family | AC50-11 (only if D50-2 approves) | each caller takes 30 s, then `Err(Scheduler)` |
| 10 | docs | interface-doc refused-drain passages, CHANGELOG, status, release state | — |
