# AC-013: recommended changes (2026-10-30)

This note proposes changes for the owner to approve; nothing here is built.
It is written against `dev/0.8.29` and covers todos TC-1b1f69e6,
TC-68169770 and TC-b23a5ead.

## Evidence

Every number below comes from the `results.json` on these branches. All of
them are pushed and none is merged.

| Branch | Head | What it measured |
| --- | --- | --- |
| `exp/0.8.29-ac013-vector-reads` | `cadcff392` | Rerank re-reads; the CS64, CS256, CHUNKED and F32T prototypes |
| `exp/0.8.29-ac013-seed-connections` | `8666ee72a` | The seed connections and the worker cache sweep |
| `exp/0.8.29-ac013-scale-drain` | `aa30085fc` | The drain backlog and the armlimit prototype |
| `exp/0.8.29-ac013-scale-search` | `07095a16d` | Search at n=100000, the eligibility cache, deferred content, and CS64 at 300k |

## Findings

1. **The gate regression.** vec0 reads and inserts walk the overflow chain
   of a 3 MB chunk blob, once per rerank candidate and once per insert. A
   2 MB per-connection cache cannot hold that chain.
   - CS64 (`chunk_size=64`) gives p50 62–64 ms and a 10k seed of 1.1 s.
   - Results are identical, apart from one Hamming tie at the 192-candidate
     cutoff at 10k.
   - The vector stage stays below today's layout up to 300k: 76 ms against
     135 ms. By extrapolation the two cross near 1.2M rows, because the
     bit-KNN costs 1.4× more per row under CS64.
2. **Drain is O(N²).** Two pending-work queries, the dispatcher scan and the
   writer's idle probe, sort the whole backlog on every run.
   - armlimit puts an `ORDER BY write_cursor LIMIT 256` on each `UNION ALL`
     arm. That makes the drain O(N).
   - The CS64 held drain falls from 33 s to 5.7 s at 100k, and from 303 s to
     about 19 s at 300k. The stores come out identical.
3. **Search at 100k: what is left after CS64.**
   - **The eligibility scans.** Two `EXISTS` scans read the whole store on
     every search, whatever the query.
   - **Text content reads.** The text arm reads the content of every match.
   - **The fixture.** Its queries match 100% of the store, against 29–34% on
     LongMemEval text.
   - **Fixes on LME at 100k.** Both prototypes are exact:

     | Configuration | Hybrid p50 (ms) |
     | --- | --- |
     | CS64 | 283 |
     | + eligibility cache | 149 |
     | + deferred content as well | 54 |
4. **The vector arm is off in any store that has superseded a node.**
   - Node supersession keeps the old `_fathomdb_vector_rows` row; edges
     prune theirs in `write_commit.rs`.
   - The Slice 35 scan, `vector_arm_requires_fallback`, then declines the
     whole vector arm.
   - With 10 supersessions in LME 10k, 500 of 500 hybrid searches returned
     `SoftFallback(Vector)`.

## Recommended changes

### On `dev/0.8.28`, to unblock 0.8.28

| # | Change | Why | Cost |
| --- | --- | --- | --- |
| R1 | CS64 on new vector partitions. Opening a store with the old layout requires a rebuild (no backward compatibility, owner 2026-10-30) | Passes the gate (about 63 ms); seed 3.9 → 1.1 s | Successor ADR to ADR-0.7.0-vector-binary-quant; the shape sentinel; first run the eu7 recall check with the real embedder |
| R2 | armlimit in `scheduler_scan_sql` and `candidate_page_sql` | Makes drain O(N); this was the 1M "did not drain in 3 h" case | Two SQL builders, no schema change; a query-plan test and a node/edge ordering test |
| R3 | Drop the 4 MB worker `cache_size` | CS64 fixes seed time without it, so ADR-0.8.28 stays as written | none |

### For 0.8.29

| # | Change | Gain (LME 100k) | Cost |
| --- | --- | --- | --- |
| R4 | Cache the eligibility answers per read-visibility generation | 283 → 149 ms, exact | Amends ADR-0.8.25-eligibility-and-frozen-reads. Measure write-then-search first; use maintained counters if the cache misses too often |
| R5 | Deferred-content text arm, plus `search_index` `rowid = write_cursor` for new stores | 149 → 54 ms with R4, exact | Slice 71 design amendment; new route-witness names; every `search_index` insert path |
| R6 | Hoist the per-row `sqlite_master` read in the worker commit (TC-0be48ca9) | About 15 page requests per vector | Small |
| R7 | Add a scale gate next to AC-013, which stays at n ≤ 10k: hybrid search at n=100000 on a Zipf-vocabulary corpus that matches about 30% of the store, plus a drain-linearity check | Catches findings 2 and 3 | `AGENT_LONG` only; the generator is committed and contains no LME text |
| R8 | A rerank that does not multiply chunks, such as F32T, before stores reach about 1M rows | Keeps CS64's gain past about 1.2M | Schema bump, erasure upkeep, +70% disk |

## Owner decision

**D1, the vector arm and superseded nodes (finding 4).**

- **Recommended:** prune the node's vector row when the node is superseded,
  as edges already do. This restores hybrid retrieval for stores that
  re-ingest by `logical_id`.
- **The trade-off:** searches with `include_superseded` lose vector hits on
  old versions. Edges already accept that.
- **The alternative:** make vector eligibility native, a larger Slice 35
  redesign.

D1 is a retrieval-quality defect, not a latency one. It has been shipped
since 0.8.25 Slice 35.

## Not measured

- MCP at 1 in flight at 100k. It needs a 100k store embedded with bge, which
  takes about 5 hours on CPU, or a CUDA build.
- The vector stage at 1M, which went over the 10 GB disk budget.
- A write-then-search workload for R4.
- Scoring every OR match, which is still about 340 ms on the synthetic 100k
  fixture. Removing it needs a top-k bm25 bound or a query-term policy;
  both change behaviour.
