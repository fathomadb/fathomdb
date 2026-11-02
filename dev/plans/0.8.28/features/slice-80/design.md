---
title: FathomDB 0.8.28 Slice 80 — search view design
status: COMPLETE; Astra reviewed and independently verified
target_release: 0.8.28
---

# Search views and historical projections

Use the existing ReadView contract: supersession and inactive state are
independent relaxations, while validity, content filters and source-dependency
eligibility continue to apply. Remove search admission rejection. Generate
canonical predicates from the view at all search retrieval sites, including
FTS ranking/streaming, vector hydration, property lookup and graph expansion.
Preserve intentional ownerless-FTS behavior and deterministic version selection
where an operation resolves a logical ID. Search history may return distinct
record versions; do not collapse them merely because their logical IDs match.
Native vec0 lifecycle IN predicates already implement the four combinations.

Retain edge vectors on both supersession keys and update their mapped rows'
lifecycle metadata inside the canonical write transaction. Do not fabricate
an `up_to_date` terminal for a superseded body-bearing edge. Extend existing
edge projection membership to include superseded versions consistently in
pending scans, commit-time membership, readiness aggregates/fast paths and
rebuild replay. Nodes already retain historical membership. Preserve existing
edge expiry and dependency eligibility rules; bodyless edges do not owe vectors.
In-flight commits derive current lifecycle metadata from canonical state.

At boot, reconcile historical edge rows whose earlier supersession created a
skip terminal with no physical vector tuple. Clear only false successful
terminals, rewind below the lowest pending historical cursor, and let the
ordinary bounded worker project them. Do not treat partial sidecar/vector/span
state as a legitimate skip, or clear genuine failed terminals. Rebuild uses
the same historical membership and reconstructs searchable historical FTS.
Lexical restoration advances frozen-read visibility once in the same transaction,
even when vector tuples need no repair; unchanged reopen does not advance it.
Validation failure rolls back both lexical repair and visibility. This repairs
derived projections, with no canonical migration or new tables.

Search must not infer complete historical vectors from a stale terminal or
watermark. Use existing physical completion and soft-fallback reporting; when
a requested historical/temporal view admits records outside the scheduler's
current membership, conservatively fall back to text rather than silently
present a partial dense history as complete. Source-dependency restrictions
remain fail-closed. Default search retains its current native fast path.

Tests use deterministic embedders and real stores. Exercise each flag alone
and together across text and dense search, old/current bodies, node inactive
states, filter/validity combinations, graph/property paths and frozen/paged
entry points. Exercise both edge supersession keys before drain, after drain
and with a paused in-flight embed; verify retained spans and metadata, reopen
of old skipped terminals, rebuild, failures and unavailable runtime fallback.
Retain stale-body defaults and bounded scheduler/physical readiness checks.
Update Rust/Python/TypeScript interface and binding documentation together.

## Astra review refinements

Relaxed hybrid searches check completeness independently of query terms in the
same reader snapshot: every view-visible, vector-committable record must have
its successful terminal and matching physical tuple. Pending, failed or missing
history disables the whole vector arm and reports the existing vector soft
fallback, including when the runtime is absent or no lexical hit exists.
Direct text search does not report an unused vector fallback. Keep this probe
off the default hot path. Status/explain must not mistake legacy intentional
skips for complete history or turn a documented fallback into an unrelated
corruption failure. Reconciliation runs for edge-only stores too, independently
of a node vector declaration. Partial physical tuples remain corruption.

Attribute projections intentionally contain current active rows only. Preserve
that invariant. For relaxed attribute filters derive values from canonical body
using the same validated path and scalar conversion as `extract_scalar_attribute`;
apply predicates before candidate caps. Strict queries retain existing indexes.

## Historical property FTS lifecycle and ranking

The reader pool remains `query_only=ON`. Each relaxed property query creates a
separate private `:memory:` SQLite connection with `temp_store=MEMORY`; it never
creates TEMP tables on a pooled reader, writes a durable index, or shares a
corpus/cache with another query. The tokenizer is the existing property FTS
`porter unicode61 remove_diacritics 2` tokenizer.

| Search operation/view | Private property corpus |
| --- | --- |
| Property, default or validity-only override | None; existing durable property index |
| Property, `include_superseded` | One per call |
| Property, `include_inactive` (including alone) | One per call |
| Property, both existence flags | One per call |
| Body text, hybrid/dense, graph, canonical reads | None |

The main reader transaction is pinned by registry lookup before extraction and
held through population, ranking, hydration and projection-cursor capture.
The private corpus clones **all** durable property rows, across declarations,
without narrowing its statistical baseline by requested property, content
filter, validity or view. It then adds absent `(write_cursor, attribute_name)`
scalar rows for every FTS declaration from historical/inactive canonical nodes
admitted by the requested existence flags and the existing dependency-closure
read predicate at the same frozen instant. This predicate preserves source
validity and closure barriers. The owner's own validity is an output gate,
not a corpus-addition gate. Hard-erased or barred history cannot add statistics.

A single private transaction and prepared insert populate scalar value, name,
canonical cursor and eligibility only; full bodies are not copied. Eligibility
is computed under the full node view and filter in the same main snapshot.
MATCH ranks the whole corpus, with requested name and eligibility restricting
output before LIMIT. At most LIMIT winning canonical rows are hydrated from
that same main snapshot. If no historical scalar rows are added, BM25 scores
and order exactly match the strict property baseline, even with multiple
attributes or validity/content filters. Admitted history can change corpus
statistics, including when its added values do not match the query.

Statements, private transaction and connection have lexical RAII ownership.
On success or any error the corpus drops before the pooled worker sends its
response, and the main read snapshot releases when the call returns. There is
no retained transient corpus on the reader. Dropping a response receiver does
not cancel an already running synchronous task; close joins active readers.
There is no new deadline or memory cap. Up to eight workers can hold separate
corpora concurrently. The test-only, engine-owned rendezvous pauses after
population and can inject an error to verify all-worker reuse and drop paths.

Concurrent supersession/erasure can leave an in-flight query returning its
coherent pre-write snapshot; a materialized result may be delivered after an
erasure finishes. Erasure does not revoke returned data or scrub process memory
or swap. Longer snapshots pin WAL and can increase checkpoint-BUSY
`ErasureIncomplete` results; retry after the reader finishes. Private memory
storage prevents SQLite corpus/sorter files, not operating-system swap.

Let B be durable property rows, P the number of FTS declarations and N the
canonical node count. CPU work is O(B + P*N) in the worst case, plus eligibility
and filter work, because canonical nodes are scanned separately per declaration
even when no historical scalar is added. Memory holds the baseline and added
scalar bytes plus eligibility/deduplication sets, regardless of LIMIT. Holding a worker
and snapshot longer also increases queue latency and delays close. The bounded
ignored qualification test measures two sizes and four simultaneous calls;
RSS samples describe the whole process and allocator reuse, not a memory bound.
The default property path has no corpus-copy cost. Historical dense validation
adds opt-in scans. Retained vectors/FTS increase storage and drain/rebuild work;
boot repair inventories edge lexical cursors once and scans historical edges,
O(E + H) for the inventory and outer loop rather than an unindexed per-edge
FTS lookup; per-history source and physical validation SQL adds further work.

## Evidence authority and ranking compatibility

Authenticated existence flags relax the returned artifact and graph-origin
edge lifecycle only. Evidence source revisions remain active, nonsuperseded,
access-authorized and subject to existing validity/dependency barriers. Issuance
and resolution enforce the same source authority. Historical self-sourced
artifacts therefore remain unavailable when their source is itself historical;
derived historical artifacts with an authorized current source can resolve.
The existing evidence out-of-window refusal remains.

Ranking algorithms, fusion and reranking are unchanged. Retaining/restoring
consolidated historical edge FTS enlarges the persistent edge BM25 corpus and
can change strict edge/graph scores or order, although default eligibility is
unchanged. Rebuild uses the same retained corpus. Strict property score parity
above is narrower than a promise of unchanged ranking across repaired stores.

Graph traversal continues to operate on logical entities. Hydration resolves
the newest eligible revision deterministically by descending write cursor,
matching point-read semantics; body/dense search may independently retrieve
older versions. Avoid duplicate historical endpoint joins consuming graph
fanout limits. Preserve existing body-based fusion/deduplication: historical
search is ranked retrieval, not a complete enumeration of every revision.

Vector attribute reshapes and late declarations derive values for every mapped
historical/inactive vector from its canonical body and registry source, rather
than the current-only EAV cache. Preserve vector IDs and spans; test late
attribute declaration and source changes across reopen.

Rebuild replays FTS for body-bearing canonical edges, including expired
history, only while their source owner remains projection-eligible. It never
resurrects projections removed by source closure. Dense scheduling/completion separately retain their expiry predicate.
A past valid-as-of historical view must retain lexical fallback after rebuild.
