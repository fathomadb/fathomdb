# Graph retrieval and bounded expansion

Choose the graph operation by its seed and result contract. These retrieval
operations traverse stored canonical relationships. Query expansion itself does
not infer entities, resolve entity names, or create semantic edges. A separate
[extractor ingest](extractor-provenance.md) can write extracted nodes and edges.

| Need | Operation | Seeds and output |
| --- | --- | --- |
| Neighbors of a known logical node | `graph.neighbors` | Explicit logical identity, bounded traversal |
| Hybrid ranked hits plus their graph neighborhood | `graph.search_expand` / `graph.searchExpand` | Initial ranked search hits followed by legacy convenience expansion |
| A graph contribution to hybrid ranking | `engine.search` with the graph arm enabled | Legacy text-seeded graph branch contributes to RRF; it is a ranking option |
| Explicit work bounds, degradation and optional exact graph evidence | `graph.expand` | Query-derived lexical seeds or ordered explicit logical seeds, then a separately bounded walk |

## Bounded `graph.expand`

`GraphExpandRequestV1` carries the seed selection, current or frozen read
context, traversal direction, edge kinds and work limits. Query seeds come
from eligible canonical-node **body FTS** in the same reader transaction,
ordered by BM25 then write cursor. The requested ranked limit applies before
duplicate logical IDs are removed, so the returned seed count can be smaller.
No embedder, vector search, cross-encoder, fused-hit search or entity-resolution
step participates in this query-seed path.

Explicit logical seeds preserve caller order. The walk applies its direction,
kind, validity, eligibility and work bounds separately. Read returned work counts
and degradation codes; a bounded result is not a claim that the whole graph was
visited. Node-attribute eligibility is enforced before seed selection and
expansion: an ineligible intermediate node cannot bridge to an otherwise
eligible target. `target_kinds` / `targetKinds` instead selects emitted targets;
an eligible node outside those kinds can still carry the traversal. Superseded
edges and edges whose `t_invalid` is at or before the resolved instant are
excluded; future `t_valid` is not a lower-bound exclusion on this path. Use frozen authority when results and evidence must share the same
resolved view; it remains reproduce-or-fail rather than a permanent reader lease.

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

The optional graph arm inside `engine.search` is another route: its legacy
edge-fact/entity text-seeding and nested fusion are not the bounded
`graph.expand` query-seed
contract. Enabling either route does not guarantee better answer relevance;
measure your corpus and inspect provenance rather than treating traversal depth
as a relevance score. None adds a first-class namespace or a new fused-hit seed
mode to bounded expansion.
