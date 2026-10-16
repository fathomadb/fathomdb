---
title: Infino's hypotheses about agent retrieval, and how they align with the §4 interface list
date: 2026-10-16
status: research note (non-canonical; see dev/notes/README.md)
purpose: Enumerate the hypotheses Infino states about what agents need from a data layer, summarise the Apache Parquet storage bet behind them, and check each against §4 of dev/notes/llm-agent-query-interfaces.md.
inputs:
  - https://thenewstack.io/infino-agent-retrieval-layer/ (Adrian Bridgwater, 2026-10-07; founder interview)
  - https://infino.ai/ and https://infino.ai/agents/ and https://infino.ai/architecture/
  - https://infino.ai/blog/seed-round/ and https://infino.ai/blog/choosing-a-database-for-ai-agents/
  - https://github.com/infino-ai/infino (README, version 0.11, Apache-2.0)
  - dev/notes/llm-agent-query-interfaces.md (§3 premises, §4 properties, §5 FathomDB mapping)
---

# Infino's hypotheses about agent retrieval, and how they align with the §4 interface list

> Infino is a vendor. Its claims below are reported as claims, with the source
> named; nothing here was independently measured. "§4.x" refers to the
> numbered properties in `dev/notes/llm-agent-query-interfaces.md`.

## 1. What Infino is

Infino is an open-source retrieval library (Rust, with Python, Node and Rust
bindings) founded by OpenSearch and search/ML veterans from Amazon, Google and
LinkedIn, launched publicly on 2026-10-07 with a $7.5M seed. It provides BM25,
vector, hybrid and SQL retrieval over one table whose data and indexes live in
Apache Parquet files on local disk or object storage. SQL planning runs on
Apache DataFusion; retrieval is exposed as table-valued functions
(`bm25_search`, `vector_search`, `hybrid_search`, `token_match`,
`exact_match`, and on the hosted product `graph_walk`) that can be filtered,
joined and aggregated in the same statement. It ships an MCP server
(read-only by default), a CLI, and a hosted "Infino Cloud". Its README says
plainly that it is not an OLTP store: tables are append-only and time-ordered,
updates are delete-plus-insert via tombstones, there is one writer slot per
table, readers pin a snapshot and never block.

## 2. The hypotheses Infino states

Each item is Infino's position, in its words where possible, with the source.

| # | Hypothesis | Source |
| --- | --- | --- |
| H1 | **Agents are a new kind of reader.** An agent "issues many small questions to answer one large one, often at the same time, and every result it receives consumes context and can add cost and latency to subsequent model calls." Data stores were optimised for a human or application that "writes one query, waits, and reads the result." | New Stack (Nwokah) |
| H2 | **The stack is fragmented and the agent is the glue.** Warehouses for SQL, search engines for keywords, vector databases for semantic search, ETL to sync them. "The agent has to call each one and then spend additional model turns combining the results." Fix: collapse the stack into one interface. | New Stack; seed post; homepage |
| H3 | **Existing tools return the wrong shape.** grep "returns matches, and the agent goes on to read whole files"; a vector database "will return a handful of chunks, and then the agent asks again"; a warehouse "returns rows, and the agent combines them itself." The agent "wants a number, not a pile of log lines." | New Stack; homepage |
| H4 | **One question should be one query.** Embed retrieval functions in SQL so "ranked searches, exact counts, joins, filters and aggregates" run together against one snapshot. "The important part isn't SQL itself"; it is that keyword, semantic, filtering, aggregation and grouping "happen together instead of forcing the agent to orchestrate several systems and reason over a pile of intermediate results." | New Stack; README |
| H5 | **Hybrid is mandatory, fused by rank, over one table version.** Keyword and vector "fail in opposite directions": keyword misses paraphrase, vector misses exact identifiers ("a rare token dissolves into its neighborhood"). Scores are not addable, so fuse by rank position (RRF, constant 60, no weights, deterministic). Both sides must read the same table version so they "can never be out of step." | agents page; choosing-a-database blog |
| H6 | **Agents live in retrieval loops; close part of the loop server-side.** Even with fast retrieval, agents spend time to "formulate a query, search, decide whether the results are good enough, try something else, validate the answer, and repeat." Infino integrates "targeted inference models with the retrieval engine so these simpler tasks run faster and more cheaply than with frontier models." | New Stack; seed post |
| H7 | **One copy of the data, in the user's bucket, in an open format.** Store valid Parquet "with the search indexes embedded beside the Parquet footer, so any tool that reads Parquet can read the data with or without Infino." No separate index artifact to build or sync; no lock-in. | New Stack; README; architecture page |
| H8 | **Cost should track query traffic, not standing infrastructure.** Conventional engines keep corpus and vectors on hot block storage in nodes that "stay on." Separate storage from compute, read byte ranges from object storage, cache hot ranges; claim roughly 10x cheaper. | agents page; architecture page; New Stack |
| H9 | **One copy enables central, deterministic governance.** With one copy, "the auth policies an agent runs under are enforced centrally and deterministically rather than across a set of MCP gateways and SaaS connectors": who reads what, which rows and columns, what is logged. | New Stack |
| H10 | **The path from data to query should be short.** No clusters, nodes, shards or headcount "just to query data"; a library with no daemon or lock service; point at Parquet or JSON, ingest, query. "Fewer clients in the agent's tool list." | New Stack; README |
| H11 | **Agent memory has a particular shape.** Append-mostly, rarely deleted, continuously growing, while each query stays small; scoped by user, session, tool or time range; agents "often need counts and aggregates over that state." | choosing-a-database blog |
| H12 | **A retrieval layer is not a system of record.** Transactional, write-heavy state belongs in an OLTP database beside it. | README; choosing-a-database blog |
| H13 | **Agents also ask in natural language, and compact queries save tokens.** NL is "another class of queries" the layer should accept; hybrid search plus SQL "let agents express complex queries compactly," reducing LLM token spend. | seed post; README; homepage |
| H14 | **Raw signals should be inspectable.** BM25 and vector contributions can be examined by joining the separate search relations on `_id`; the `k` retrieval depth is separate from the displayed `LIMIT`. | agents page |

A marketing claim worth isolating rather than treating as a hypothesis: the
homepage's "keyword + semantic + graph = 100% retrieval accuracy". The agents
page's own cited numbers (keyword 40.6, vector 45.0, hybrid 48.4, hybrid plus
rerank 50.0, on BEIR-style benchmarks) contradict it.

## 3. The Apache Parquet bet, briefly

Parquet is a column-oriented file format. A file holds row groups, each
holding one column chunk per column, split into pages that are encoded and
compressed independently. A footer at the end of the file carries the schema,
the byte offsets of every column chunk, per-chunk statistics (min, max, null
count), optional Bloom filters and page indexes, and free-form key-value
metadata. Readers locate data by footer offsets, which has two consequences
Infino relies on:

- **Any byte range the footer does not reference is legal and ignored.**
  Infino writes its BM25 and vector index regions between the row groups and
  the footer. DuckDB, pandas, Spark and DataFusion read the file as an ordinary
  table; Infino's reader additionally knows where the index regions are. This is
  how one file serves as both the data copy and the index (H7).
- **Column pruning and statistics-based skipping are free.** A query reads
  only the column chunks it needs, and per-file or per-row-group min/max, Bloom
  and term summaries let whole files be skipped (H8).

Parquet files are immutable, so Infino borrows the lakehouse pattern: a table
is a manifest listing its files; a write adds files and atomically swaps the
manifest; readers pin a manifest version. That is what makes H5's "same table
version" and H12's append-only posture structural rather than policy. Parquet
has no native text or vector index, so the indexes are Infino's own format
inside the file. The embedded-index trick is a format convention, not a
Parquet feature; an Iceberg or Delta table manager sees ordinary Parquet.

Parquet was not designed for the scattered-row reads that nearest-neighbour
search produces, which is why the vector path is a clustered funnel (cluster
centres, then 1-bit codes, then 2-byte re-scoring) sorted so a query becomes a
short list of byte-range reads. The format bet therefore trades per-row random
access for ownership, analytics interoperability and cold-storage cost.

## 4. Alignment with the §4 list

### 4a. Where Infino and §4 agree

| Infino | §4 property | Note |
| --- | --- | --- |
| H1 (agents are a new reader; results consume context) | §3 premises | Identical diagnosis: context is the scarce resource, each round trip costs, humans scroll and agents pay. This is the shared foundation. |
| H2, H4 (one question, one query; fewer model turns combining results) | §4.4 engine-chosen plan; §4.10 compound requests | Same goal, different mechanism. §4.10 proposed a compound-request envelope; Infino uses SQL's relational algebra as the composition language. Both fold the agent's second and third calls into the first. |
| H3 (wrong output shape; "wants a number") | §4.14 aggregates; §4.2 handles and budget | Strong on aggregates: SQL `COUNT`/`GROUP BY` over search results is exactly §4.14. Partial on §4.2: Infino's remedy for over-fetch is projection and `LIMIT`, not a token budget or a snippet-plus-handle default. |
| H4 exact counts; stats-answered aggregates | §4.9 estimate before execute; §4.3 completeness | A cheap `COUNT` over a search relation gives the agent a saturation and absence signal, and the README says some aggregates are answered from table statistics without a scan. This covers the count half of §4.3 and the estimate half of §4.9. |
| H5 (hybrid, exact tokens, rank fusion, same version) | §4.5 exactness alongside semantics; §4.6 snapshots | Strong. `token_match` and `exact_match` composed with `hybrid_search` in one `WHERE` is §4.5 as written. "Both sides read the same table version" is §4.6 applied inside one query. Deterministic RRF with no weights matches the §3 point that scores are not comparable across arms. |
| H11, H12 (append-mostly, scoped, snapshot-pinned) | §4.6 snapshots; §4.13 change-oriented queries | Time-ordered append tables make "since T" a `WHERE` clause. Pinned manifests give a plan one world. Infino arrives at §4.6 from storage immutability; FathomDB arrives at it from frozen read contexts. |
| H14 (inspect raw signals by joining on `_id`) | §4.4 explain, positive direction; §4.7 handles | Per-hit contributions are available, and `_id` is a handle that joins. This is positive explain done in SQL rather than as a sidecar. |
| H10, H13 (fewer tools in the agent's list; compact queries save tokens) | §4.15 output and schema designed for a per-token reader | Same instinct: a small surface with rich expressiveness beats many narrow tools. Infino's single surface is SQL; §4.15 argued for few verbs with typed options. |
| H4 "SQL as the surface" | §4.16 constrained escape hatch | §4 placed read-only, limited SQL as the long-tail escape hatch behind a small verb set. Infino inverts the ordering and makes SQL the primary interface. See 4c. |

### 4b. Infino hypotheses the §4 list did not cover

- **H6, server-side loop closure.** §4.3 and §4.4 proposed that the engine
  emit signals (counts, saturation, no-match verdict, negative explain) so the
  *agent* can decide whether results are good enough. Infino instead puts
  small inference models inside the retrieval engine to do the judging,
  reformulating and verifying. These are complementary, not competing: a layer
  can emit signals and also offer an opt-in "close the loop for me" mode. The
  trade is determinism and explainability; a hidden model in the read path makes
  §4.4 harder, and FathomDB's explain sidecar and signed frozen contexts are
  built on the opposite instinct. Worth recording as a candidate §4.17 with that
  caveat.
- **H7, the storage format is part of the agent interface.** §4 treated the
  interface as the tool boundary. Infino argues that whether the retrieval
  layer holds *the* copy or *a* copy, and whether other tools can read it,
  changes what an agent is allowed to trust and what the operator must keep in
  sync. FathomDB's single SQLite file with FTS5 and sqlite-vec inside it has the
  same property at the embedded scale point: one file, one copy, open format,
  indexes inside, readable by any SQLite tool. Neither note had named it.
- **H9, governance as a retrieval-layer concern.** §4 said nothing about which
  rows and columns an agent may see or what is logged. For agents this is a
  query-interface property: policy applied at the one read path is enforceable;
  policy spread across tool gateways is not. FathomDB has governed records and
  source-based erasure but no reader-scoped visibility policy. Candidate.
- **H8, cost as an interface constraint.** Orthogonal to the §4 list's
  question, but it shapes which answers are feasible: a token budget (§4.2) and
  an estimate step (§4.9) are cheap to offer when cost tracks bytes read.

### 4c. §4 properties Infino does not address, and one divergence

| §4 property | Infino status | Comment |
| --- | --- | --- |
| §4.1 self-describing catalogue | not addressed | SQL implies `DESCRIBE`-style schema, but nothing published about a data-aware, token-budgeted catalogue (kinds, example values, cardinalities, readiness). Text-to-SQL correctness still hinges on this. |
| §4.2 token budget, snippet-by-default | not addressed | `LIMIT` and projection only. No `max_tokens`, no truncation markers with a handle to fetch the remainder. |
| §4.3 confidence and no-match verdict | counts only | A count says how many matched, not whether the top hit is a confident match or best-of-noise. No calibrated "nothing good matches" signal published. |
| §4.4 negative explain | not addressed | "Why did X not appear" has no SQL analogue short of the agent re-running the arms separately. |
| §4.7 provenance and validity on every result | user-modelled | Columns can carry source and validity, but nothing is first-class. FathomDB carries `source_id`, validity windows and supersession on every hit. |
| §4.8 typed errors with suggestions | not addressed | DataFusion errors are SQL errors. No did-you-mean on identifiers published. |
| §4.11 named working sets | within one statement only | CTEs compose inside a query; nothing persists a result set across tool calls. |
| §4.12 bounded deterministic traversal | unpublished | `graph_walk` exists on the hosted product; no published bounds, caps or all-or-nothing semantics. |
| §4.13 mutation log | by table order | "Since T" works because tables are time-ordered; there is no mutation or boundary-crossing log of the kind `read.mutations` and `read.crossed_boundary_since` provide. |

The one real divergence is **natural language as a first-class query input
(H13)**. §3 argued that NL-in is not the best agent interface because it adds
a second interpretation layer with its own ambiguity, and that agents are
better at emitting structured arguments. Infino offers NL alongside SQL and
Query DSL rather than instead of them, so the disagreement is about emphasis.
The deeper difference is **SQL as primary (H4) versus a small verb set with SQL
as escape hatch (§4.16)**. Infino's bet is defensible for a table-shaped,
operator-owned lake where the schema is small and known. §4's ordering is
defensible for a governed store with typed ids, validity views and signed
frozen contexts, where open SQL would let an agent bypass the semantics the
verbs enforce. The honest synthesis: the §4 verb set should be *expressible* as
relational operations over documented views, so that an escape hatch, if ever
offered, cannot reach rows the verbs would hide.

## 5. Net assessment

Infino and the §4 list share the diagnosis (H1) and most of the composition
and exactness remedies (§4.4, §4.5, §4.6, §4.10, §4.14). Infino adds three
things the §4 list missed: the storage format as part of the interface, central
governance at the one read path, and server-side closure of the retrieval loop.
The §4 list covers a cluster Infino does not: the catalogue, token budgets,
confidence and no-match signals, negative explain, first-class provenance and
validity, actionable errors, and persistent working sets. That cluster is the
"help the agent know when to stop" half of the problem; Infino's cluster is the
"let the agent ask once" half. Both are needed, and the FathomDB mapping in
`dev/notes/llm-agent-query-interfaces.md` §5 already holds the substrate for
the first half.
