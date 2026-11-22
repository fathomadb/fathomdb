---
title: Inspector tool survey and parity assessment
date: 2026-11-18
product: FathomDB Inspector
target_release: 0.1
status: ACTIVE
desc: Official-source GUI survey and current versus planned Inspector task coverage
---

# Inspector tool survey and parity assessment

Current release scope is now [adopted 0.2.0](../plans/0.2.0/scope.md).
The findings and actual/planned comparison below remain the 2026-11-18 research
snapshot; original v0.1 future-view obligations were subsequently carried into
0.2.0, with bounded two-observation comparison added. This is not new runtime
evidence and does not rewrite the dated survey.

## Finding and evidence boundary

ArangoDB is the closest multimodel architecture comparator; Weaviate Explorer
provides direct lexical/vector/hybrid search controls; Neo4j Browser provides
an explicit precedent for linked entity views. Coordinated views are an
established interaction pattern, not a novelty claim. The useful Inspector
question is whether users can investigate one entity across its representations
without losing identity, request context or evidence.

Official documentation, vendor release notes and vendor repositories were
reviewed on 2026-11-18; no competing UI was executed. These are representative
comparators, not an adoption ranking. Current URLs can change. A documented
backend operation is distinguished from a dedicated GUI control. “Not
established” below means not established by the reviewed sources, not proven
absent. Recommendations are research inferences; this note changes no
requirements, acceptance criteria, slice status or release scope.

Accepted follow-up examples and opportunities are captured in the dedicated
[Inspector backlog](../BACKLOG.md). It distinguishes refinements of required
v0.1 work from deferred and unscheduled candidates; the survey remains the
source comparison, not a release commitment.

## Verified GUI workflows

| Tool and actual UI | Established inspection workflow | Boundary relevant to Inspector |
| --- | --- | --- |
| Qdrant Web UI | Local/cloud collection and point inspection; REST console; payload-index management. Graph Exploration navigates semantic similarity. Visualize uses the Distance Matrix API where supported, with fallback for PCA/older servers. Search Quality compares approximate with exact retrieval. [UI](https://qdrant.tech/documentation/web-ui/), [UI releases](https://github.com/qdrant/qdrant-web-ui/releases), [search quality/graph](https://qdrant.tech/blog/qdrant-1.11.x/) | The similarity graph is not stored factual relationships. Dedicated no-code BM25 inspection and a complete text/vector/known-relations person workflow are not established. Backend hybrid support alone does not establish such a UI. |
| Weaviate Cloud Explorer | Browse object UUID/properties/metadata/named vectors and dimensions; Keyword/BM25, Semantic, Hybrid and Aggregate modes; filters; hybrid alpha/property controls; text or raw-vector input; Find similar and Show JSON result actions. [Explorer](https://docs.weaviate.io/cloud/tools/explorer-tool) | Strong direct record-to-search-to-neighbor precedent. Documented for Cloud-hosted collections. Interactive stored-relationship graphs, highlighting and score decomposition are not established here. The old console Query tool was removed in September 2026. |
| Neo4j Browser and Bloom/Explore | Browser graph/table/raw/plan result frames; node/edge properties; expansion/collapse; export. Simultaneous graph/table mode highlights selected entities in both directions. Bloom supports connected-record inspection and native full-text search. [Browser](https://neo4j.com/docs/browser/operations/result-frames/), [Bloom](https://neo4j.com/docs/bloom-user-guide/current/about-bloom/), [full-text UI](https://neo4j.com/docs/bloom-user-guide/current/bloom-tutorial/full-text-search/) | Full-text/vector indexes are explicitly queryable through Cypher; a dedicated vector search builder/space viewer is not established. Current SEARCH syntax is version-sensitive. Local Desktop Bloom is free; some Enterprise functionality requires a license/plugin. [Search index contracts](https://neo4j.com/docs/cypher-manual/current/indexes/search-indexes/) |
| ArangoDB Web UI | Collection/document inspection, filters/download and index administration; interactive graph expansion and node detail; AQL execution/explain, multiple result boxes, saved queries and running/slow query views. Vector index creation has a GUI path. [Collections](https://docs.arango.ai/arangodb/stable/components/web-interface/collections/), [graphs](https://docs.arango.ai/arangodb/stable/components/web-interface/graphs/), [AQL editor](https://docs.arango.ai/arangodb/stable/components/web-interface/queries/), [vector index UI](https://docs.arango.ai/arangodb/3.12/indexes-and-search/indexing/working-with-indexes/vector-indexes/) | One JSON person document can be a graph vertex and indexed for text/vector retrieval. BM25 scores, vector neighbors and traversals are AQL workflows, not established dedicated search inspectors. In documented 3.12, vector support starts at 3.12.4 and requires the startup option. [BM25](https://docs.arango.ai/arangodb/stable/indexes-and-search/arangosearch/ranking/), [vector requirements](https://docs.arango.ai/arangodb/stable/aql/functions/vector/) |
| SurrealDB Surrealist/Studio | Current Studio connects to any SurrealDB instance from web/desktop for SurrealQL, records and schema. Official Surrealist examples show graph-result mode, person/knows relations and hovered record connections. [Studio](https://surrealdb.com/docs/explore/studio), [graph UI examples](https://surrealdb.com/blog/visualising-your-data-with-surrealists-graph-view) | Documents, graph, vectors and BM25/highlighting share an engine, but this does not establish dedicated Studio text/vector inspectors. Current detailed Studio documentation is incomplete; the graph article documents the earlier Surrealist UI. [Engine capabilities](https://surrealdb.com/docs/what-is-surrealdb) |
| MongoDB Compass with Search/Vector Search | Document results; visual/text aggregation builder; Search/Vector Search index creation and readiness; vector queries through Compass; history/favorites; filtered/pipeline JSON/CSV export. [Builder](https://www.mongodb.com/docs/compass/create-agg-pipeline/), [vector indexes](https://www.mongodb.com/docs/compass/indexes/create-vector-search-index/), [queries](https://www.mongodb.com/docs/vector-search/query/aggregation-stages/vector-search-stage/), [history](https://www.mongodb.com/docs/compass/query/recent/), [export](https://www.mongodb.com/docs/compass/import-export/) | Search service availability is separate from the desktop GUI: the reviewed Compass index page specifies supported Atlas or Atlas-CLI local deployments and version prerequisites. Recursive graphLookup returns arrays; an interactive relationship canvas is not established. [Graph lookup](https://www.mongodb.com/docs/manual/reference/operator/aggregation/graphlookup/) |
| Elasticsearch with Kibana Discover/Console/Graph | Discover field/document inspection, filters, result tables, highlighting and query tabs; BM25/vector/hybrid queries through supported languages; Graph explores indexed-term connections. [Discover](https://www.elastic.co/docs/explore-analyze/discover/discover-get-started), [search interfaces](https://www.elastic.co/docs/solutions/search/querying-for-search), [Graph](https://www.elastic.co/docs/explore-analyze/visualize/graph) | Graph derives document-term co-occurrence, not stored person-to-person edges. Defaults sample and suppress weak connections, so individual evidence can be omitted. Playground availability is version-dependent (deprecated 9.4, removed 9.6). [Graph limits](https://www.elastic.co/docs/troubleshoot/kibana/graph), [Playground status](https://www.elastic.co/docs/solutions/elasticsearch-solution-project/playground-query) |
| Milvus with Attu | Schema/data/metadata inspection and filters; vector-search result IDs/similarity scores; Explore renders the query and retrieved entities, opens details and expands additional similarity neighbors. Current Attu also documents import/export, monitoring and administration. [Walkthrough](https://milvus.io/docs/quickstart_with_attu.md), [Attu](https://github.com/zilliztech/attu) | Its graph-like display represents similarity, not asserted relations. Text/analyzer administration does not by itself establish a dedicated BM25 result inspector. Check selected Attu/Milvus compatibility and licensing rather than assuming all historical deployments have current features. |

An advanced comparator is [OpenSearch Search Relevance Workbench](https://docs.opensearch.org/latest/search-plugins/search-relevance/using-search-relevance-workbench/):
search configurations, query sets, judgments, quality metrics and hybrid
optimization, with [side-by-side query results](https://docs.opensearch.org/latest/search-plugins/search-relevance/compare-search-results/)
and [experiment dashboards](https://docs.opensearch.org/latest/search-plugins/search-relevance/explore-experiment-results/).
This is retrieval evaluation tooling, not an entity/relationship inspector or
a universal baseline gate.

ArangoDB offers the closest complete architectural example for the proposed
person task: inspect one person document, query its text/vector indexes in
AQL, then expand its stored relationships in the graph viewer. That is a
combination of documented workflows, not evidence of one integrated no-code
person screen. Weaviate supplies the clearest direct search controls; Neo4j
supplies the strongest verified linked table/graph interaction.

## Inspector implementation versus planned coverage

This assessment uses task-branch baseline `826ac0afc4bf4f85368143f232f927c68541af32`.
The [0.8.29 port record](../plans/0.1/compatibility-0.8.29-port.md) owns executed
verification and limits. The actual [worker Request enum](../../../src/inspector/src/worker.rs)
contains Catalog, Check, Page, Sample, Detail, Chunk, Tree, Associations,
Export and Close; it has no text-search, vector-neighborhood or interactive
graph operations yet. Imported Slice 20 remains historically ACTIVE with
outstanding gates. Architecture and future slice documents are not runtime
passes.

| Stage | Current implementation or planned result | Parity assessment (inference) |
| --- | --- | --- |
| Current record browser | Nodes/edges/current/log browse, filters/scoped counts, document/tree/chunking/exact copy, metadata/attributes/verified direct sources, retained revisions, sampling and policy/lifecycle handling; bounded work/cancellation and conditional narrow erasure export. | A useful record-inspection foundation, substantially short of complete search/vector/graph task coverage. |
| Planned Slice 30 | Indexed text, effective tokenizer/route and available match/score evidence; lexical retrieval and exact origin navigation. | Addresses core lexical inspection; available evidence must not be presented as an invented score explanation. |
| Planned Slice 40 | Stored metric-faithful neighborhoods, named L2 profile/population, bounded candidates, actual Engine hybrid observations and separately cached query models. | Addresses core similarity inspection; distances, candidate scope and query/model origin remain explicit. Current layout has window metadata, but selected producers/admission are head-only. Owner S9-010 qualifies supported single head windows in [0.2.0 scope](../plans/0.2.0/scope.md); within-source multiple-window positive is deferred. Exact spans/revisions and zero/one/many origins still require qualification. |
| Planned Slice 50 | Directed interactive graphs, node/edge content, expansion/collapse, traces, provenance and retained history. | Addresses actual relationship investigation, which similarity/co-occurrence graphs do not establish. |
| Planned Slice 60 | Integrated identity, original requests, observations, qualification and coordinated navigation. | Can establish integrated task coverage only after the interactions and acceptance signals actually pass. |

The [v0.1 plan](../plans/plan-0.1.md), [high-level design](../design/high-level-design.md)
and [requirements](../requirements.md) already require native/coordinated
views, including INS-04/05/09. Successful completion could provide core
inspection coverage across representations. It would not establish full
feature, workflow or administration parity, usability success, or a market
advantage without task-based evaluation.

Existing [ROADMAP](../ROADMAP.md) deferrals remain deferrals: persistent
investigation/saved sessions and general export/share; broader predicates,
field/schema inference and distributions; live attachment to an actively
writing application; historical-format adapters. Remote deployment and other
platforms are not current support promises. Write/admin tooling is outside
the inspection mandate. Relevance tuning, ANN fidelity benchmarks, reduced
2D/3D maps and graph analytics are specialist extensions, not universal core
requirements. ANN fidelity and retrieval relevance measure different things.

## Recommended entity interaction and comparison

The existing design's navigator, document/tree, lexical cards, distance bars,
SVG graph and association/comparison views fit the task. Strengthening concrete
interactions and acceptance evidence is a recommendation, not new approved
scope. A person investigation should keep a canonical ID/revision and read
context visible while coordinating:

- Text index entries, actual matches and lexical scores with route/tokenizer evidence.
- One or more vector representations, source windows/model and actual distances.
- Stored directed person relationships, node/edge content and bounded expansion.
- Verified representation/provenance links, eligibility and readiness facts.

Selection in any view should preserve the original query/context and resolve
or highlight the same verified identity in the others, with return navigation.
Authored-once data can have several physically stored representations; a derived
canonical record can have a different ID. Correlation requires exact verified
links, not matching text. Do not assume every canonical record is a generated
graph projection. Label lexical match, vector similarity, stored edge and source
provenance separately; neither projected 2D proximity nor a similarity link
establishes a known relationship.

This follows Neo4j's documented linked selection and the established
[coordinated multiple views literature (Roberts, 2007)](https://kar.kent.ac.uk/14569/).
A useful optional extension is a per-method result comparison showing membership,
rank and method-specific metric under matched query scope. Unreturned, capped,
unsupported and unknown must remain distinct; raw BM25 and vector distances
must not be treated as comparable scores. That detailed comparison is a
recommendation, not a newly guaranteed v0.1 deliverable.

For ordinary inspection tasks, the recurring baseline is browse/filter, exact
identity/raw content, index/readiness facts, explicit query parameters and
limits, correctly labeled ranks/scores/distances, source-record navigation and
actual relationship expansion. History, saved queries and general exports are
common workflow conveniences but remain subject to Inspector's existing scope.
Advanced tuning and evaluation should follow demonstrated user need.

Finally, inspection controls and enforced read-only access differ. Compass
[documents](https://www.mongodb.com/docs/compass/settings/read-only/) that its
read-only setting hides write UI and recommends a database read role for
enforcement; Qdrant provides [server-enforced read-only keys](https://qdrant.tech/documentation/operations/security/).
Inspector permits qualified Engine startup/lifecycle effects. Its inspection
mandate must not be described as immutable or effect-free attachment.
