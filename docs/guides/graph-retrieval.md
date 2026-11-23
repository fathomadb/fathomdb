# Graph retrieval and bounded expansion

Choose the graph operation by its seed and result contract. These retrieval
operations traverse stored canonical relationships. Query expansion itself does
not infer entities, resolve entity names, or create semantic edges. A separate
[extractor ingest](extractor-provenance.md) can write extracted nodes and edges.

| Need | Operation | Seeds and output |
| --- | --- | --- |
| Neighbors of a known logical node | `graph.neighbors` | Explicit logical identity, bounded traversal |
| Hybrid ranked hits plus their graph neighborhood | `graph.search_expand` / `graph.searchExpand` | Initial ranked search hits followed by legacy convenience expansion |
| A graph contribution to hybrid ranking | `engine.search` with the graph arm enabled | Graph-FTS (default), opt-in fused/reranked top-k, or combined graph branch contributes to RRF; it is a ranking option |
| Explicit work bounds, degradation and optional exact graph evidence | `graph.expand` | Query-derived lexical seeds or ordered explicit logical seeds, then a separately bounded walk with optional inline property predicates |

## Bounded `graph.expand`

`GraphExpandRequestV1` carries the seed selection, current or frozen read
context, traversal direction, edge kinds, optional closed inline property
predicates (`edge_predicates` / `edgePredicates` and `target_predicates` /
`targetPredicates`), and work limits. Query seeds come
from eligible canonical-node **body FTS** in the same reader transaction,
ordered by BM25 then write cursor. The requested ranked limit applies before
duplicate logical IDs are removed, so the returned seed count can be smaller.
No embedder, vector search, cross-encoder, fused-hit search or entity-resolution
step participates in this query-seed path.

Explicit logical seeds preserve caller order. The walk applies its direction,
kind, inline property predicates (implicit `AND`, up to 32 predicates each,
evaluated strictly against inline `properties` with no `body` fallback),
validity, eligibility, and work bounds separately. Read returned work counts
and degradation codes; a bounded result is not a claim that the whole graph was
visited. Node-attribute eligibility is enforced before seed selection and
expansion: an ineligible intermediate node cannot bridge to an otherwise
eligible target. `target_kinds` / `targetKinds` and `target_predicates` /
`targetPredicates` instead select emitted targets;
an eligible node outside those filters can still carry the traversal. Superseded
edges, wrong-kind edges, edges whose `t_valid` is in the future (`t_valid > now`),
and edges whose `t_invalid` is at or before the resolved instant (`t_invalid <= now`)
are excluded in SQL before consuming incident limits or work units. Returned
targets carry `properties` and `origin.terminal_edge_properties` /
`origin.terminalEdgeProperties`. Use frozen authority when results and evidence
must share the same resolved view; it remains reproduce-or-fail rather than a
permanent reader lease.

Optional graph evidence requires frozen context. It identifies the exact target
and winning terminal-edge artifacts, without claiming that every edge on a path
was resolved. See [exact graph evidence](frozen-evidence.md#exact-graph-evidence)
and the [Python](../reference/python-api.md) / [TypeScript](../reference/typescript-api.md)
request carriers.

## Hybrid convenience paths

`search_expand` begins with the ranked hybrid result and then expands its
logical seeds. The ordinary helper uses a second read for expansion and drops
search versions that are no longer active in that snapshot. It is distinct from bounded `graph.expand` and has its own
`search_limit` / `searchLimit` and traversal options. Its frozen counterpart
binds that operation to frozen authority.

The optional graph arm inside `engine.search`, `engine.search_frozen` /
`engine.searchFrozen`, and `engine.search_with_evidence` /
`engine.searchWithEvidence` defaults to `graph_seed="graph_fts"` /
`graphSeed: "graph_fts"` (seeding from edge-body and entity-node FTS without
allocating slots to ineligible endpoints). Setting `graph_seed="fused_top_k"`
or `"both"` (`graphSeed: "fused_top_k" | "both"`) opt-in seeds up to
`graph_seed_k` / `graphSeedK` (`1..=100`, default `10`) canonical-node hits
(`logical_id IS NOT NULL`) from the two-arm fused or cross-encoder-reranked
ranking, enabling traversal over body-less property-graph edges while filtering
edges by optional `graph_edge_kinds` / `graphEdgeKinds` and
`graph_edge_predicates` / `graphEdgePredicates`. When explanation or evidence
is requested, `PerHitExplain` reports `graph_seed_source` / `graphSeedSource`
(`"fused_top_k"`, `"edge_fts"`, or `"entity_fts"`) and `graph_hop_depth` /
`graphHopDepth`, and `EvidenceGraphOriginV1` reports `seed_source` /
`seedSource`. Enabling either route does not guarantee better answer relevance;
measure your corpus and inspect provenance rather than treating traversal depth
as a relevance score. Bounded `graph.expand` remains a separate explicit-or-FTS
seed contract.
