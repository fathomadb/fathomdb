# Embedded competitor benchmark survey

**Researched:** 2026-10-13 by a web-research subagent; primary sources checked
again before filing. **Status:** research and experiment selection, not a run,
receipt, published-reference row, or approved PROGRAM track. Source pages and
repositories can change; pin their releases, commits, inputs, and licenses when
turning a candidate into an experiment file.

## What the published work can give us

Different hardware prevents a defensible speed ratio against a vendor's
published number. We can still match the **task**: corpus and query IDs,
retrieval unit, top-K, gold or exact oracle, index settings, filter semantics,
readiness point, cache state, concurrency, and timing boundary. Published
numbers remain `published_context`. A local competitor run is a separate arm
only after the shared workload and its artifacts are pinned. This follows the
[competitive testing requirements](../../dev/performance-benchmarking/competitive-testing-requirements.md)
and the [competitive runner contract](../configs/competitive/README.md).

| System and deployment | Published/runnable benchmark evidence | Useful matched dimensions | Limit on comparison |
| --- | --- | --- | --- |
| **LanceDB OSS, local in-process** | Its [FTS documentation](https://github.com/lancedb/docs/blob/main/docs/search/full-text-search.mdx) defines an indexed BM25 top-K query and pre/post filtering in Python, TypeScript, and Rust. The vendor's [Hermes memory experiment](https://www.lancedb.com/blog/semantic-memory-for-hermes-agent-with-lancedb) and [runnable plugin repository](https://github.com/lancedb/hermes-agent-memory) use LongMemEval-S, top-5 evidence, FTS/vector/hybrid/reranked variants, retrieval metrics, answer grading, and latency. The [RaBitQ article](https://www.lancedb.com/blog/feature-rabitq-quantization) uses DBpedia 1M and GIST1M with recall@10, throughput, and index-build time on an Intel 12400F, SSD, no-GPU host. [lance-bench](https://github.com/lancedb/lance-bench) stores host/build identity and raw timings for Lance regressions. | Local FTS with the same documents, question IDs, qrels, top-10, materialized IDs, and separately recorded tokenizer/stemming; later precomputed-vector recall versus latency and build-to-ready. | The Hermes article uses a stratified 60-question application sample, top-5, answer models, and a query interval that includes network embedding. It does not disclose enough host/warmup detail for comparison to a FathomDB SDK-call time. The RaBitQ figures are a LanceDB index comparison on different hardware, not a FathomDB result. |
| **SurrealDB, embedded Rust or server** | The official [crud-bench repository](https://github.com/surrealdb/crud-bench) has configuration-defined CRUD, FTS, vector exact/HNSW/DiskANN, seeded holdout queries, recall@K, parameter sweeps, filter predicates/selectivity, build-to-queryable timing, disk, and RSS. Its [published 3.0 discussion](https://surrealdb.com/blog/surrealdb-3-0-benchmarks-a-new-foundation-for-performance) gives a host description and version-to-version results. | Reuse its **protocol ideas**: exact top-K over predicate-matching rows as filtered oracle; report measured selectivity; distinguish build-call return from index actually serving queries; freeze the embedded engine/backend and result projection. | The README mixes implemented and planned checklist items, especially graph paths. Confirm a pinned configuration and adapter before calling a leg runnable. Embedded and server results are different deployment classes. |
| **Chroma, local PersistentClient** | [Local API documentation](https://docs.trychroma.com/reference/python) names `PersistentClient`. Its [single-node performance guide](https://docs.trychroma.com/guides/performance/single-node) describes 1024-dimensional vectors, 100–200-word documents, three metadata fields, EC2 RAM sizes, query/insert tails, batch-32 insertions, and concurrency/collection-size effects. | Precomputed vector plus metadata-filter workload; build-to-ready, disk/RSS, query distribution, and an independently generated exact oracle. | The published EC2 stress numbers are not identified as `PersistentClient` runs. Corpus/query seeds, warmup, index knobs, and accuracy oracle are insufficiently specified for a matched result. `where_document` is not evidence of ranked BM25 parity. |
| **Qdrant server; Qdrant Edge separately** | Qdrant's [server benchmark](https://qdrant.tech/benchmarks/) fixes ANN datasets, matched recall/precision, client/server VM sizes, Docker memory limit, index wait, serial latency versus parallel throughput, and p95. Its [filtered-vector dataset repository](https://github.com/qdrant/ann-filtering-benchmark-datasets) supplies vectors, payloads, queries, conditions, and expected nearest IDs/scores. [Qdrant Edge](https://qdrant.tech/documentation/edge/edge-vs-qdrant-cluster/) is an in-process Python/Rust product with its own index/optimization lifecycle. | The filtered dataset's query and exact-answer format; selectivity and predicate-specific recall; Edge as a possible later local arm. | Server/Docker timings include a client-server boundary and cannot stand in for Edge latency. No directly comparable Edge performance table was found in the reviewed official sources. Unsupported FathomDB predicates must be explicit excluded cells. |
| **DuckDB-VSS; Weaviate Embedded** | [DuckDB-VSS](https://github.com/duckdb/duckdb-vss) is an experimental in-process HNSW extension. [Weaviate Embedded](https://docs.weaviate.io/deploy/installation-guides/embedded) is experimental and starts a local Weaviate instance from client code. | Optional controls for index/filter and deployment-boundary studies. | DuckDB-VSS is a technical extension baseline rather than a primary agent-memory product. Weaviate Embedded has a local server process boundary; neither belongs in the first in-process FTS comparison. |

## Recommended next experiments

1. **First: a real lexical retrieval workload.** Use the already acquired
   BEIR NFCorpus (`data/corpus-data/raw/beir/nfcorpus/`, local and gitignored) test split:
   3,633 documents and 323 test query IDs with qrels are present locally.
   Freeze their hashes, document-to-record mapping, result unit, `top_k=10`,
   scoring definitions (nDCG@10, recall@10, MRR where justified), query order,
   process/cache state, and public-call-to-materialized-IDs timing. Run
   FathomDB 0.8.27 first through a versioned experiment. A local LanceDB FTS
   arm is the first product comparison, using the same qrels and host. Record
   each engine's tokenizer/stemming and effective index state; quality need
   not be identical for the latency cells to be reported separately. Use the
   existing [IR-C FTS baseline](../../dev/performance-benchmarking/tracks/search-01-ir-c-baseline.md)
   as a FathomDB regression reference, not as the BEIR comparator score.
2. **Second: index build, query readiness, and bounded scale.** Adopt
   `crud-bench`'s build-return versus queryable distinction. At declared 10k
   and 17,272-document tiers, capture ingest acknowledgement, searchable
   readiness, reopen/first query, steady query p50/p95/p99, disk, and peak RSS
   in separate cells. Reuse the existing
   [SCALE-02](../../dev/performance-benchmarking/tracks/scale-02-local-first-envelope.md)
   envelope and its qualification rules before adding a larger tier. For
   vectors, use precomputed, pinned vectors and an exact top-K oracle; run a
   capability check first because FathomDB 0.8.27 has no declared common
   vector-only SDK operation. Do not substitute hybrid for vector-only or
   extrapolate a 1M-point published result from 17k local data.
3. **Third: filtered search accuracy and cost.** Borrow the predicate-aware
   exact oracle and observed-selectivity reporting from `crud-bench`. Start
   with one supported categorical equality predicate at roughly 1%, 10%, and
   33% observed selectivity, then expand only to predicate types all selected
   arms actually support. Report filtered recall@10, returned count, latency,
   index/schema settings, and whether filtering happens before or after
   candidate selection. The Qdrant dataset format is a later source of
   precomputed-vector queries and exact results; review its license and
   predicate mapping before acquisition.
4. **Then: shared answer quality.** If retrieval reveals a meaningful
   difference, run a frozen LongMemEval-S or LOCOMO question subset through
   the same answerer, prompt, evidence budget, and judge for each arm. Score
   retrieval and answers separately. The LanceDB Hermes experiment supplies
   useful question-class and top-5 design ideas; its published answer scores
   and embedding-inclusive latency cannot be compared directly with ours.

The first three proposals use the existing latency/capacity and retrieval
scoreboards. Answer remains downstream of a verified retrieval case. A
capability gap is recorded as unsupported or blocked, never as zero recall or
zero time.

## Fields to freeze in a future experiment file

The shared file needs source/version/build digest per arm; deployment mode;
corpus/query/gold hashes and license; record and embedding dimensions; search
mode and index parameters; tokenization or embedding model; exact predicate
semantics and observed selectivity; top-K and result unit; ingestion and
searchability criteria; cold/first-pass/steady state; repetitions, query
ordering, concurrency, timing boundary, result projection; and metric/uncertainty
rules. Preserve CPU, RAM, storage, OS, and runtime information even when the
reference hardware differs. Unknown vendor settings remain **unknown** rather
than inferred defaults. The [SDK experiment runner](../configs/sdk/README.md)
and [competitive experiment file](../configs/competitive/README.md) already
provide the configuration/receipt pattern; this survey adds no benchmark
result and does not authorize downloading a new corpus or executing a
competitor.
