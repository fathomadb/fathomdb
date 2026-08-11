---
title: Performance and hardening initiative
status: ACTIVE
scope: cross-release
last_reviewed: 2026-08-11
---

# FathomDB performance and hardening initiative

## Purpose

FathomDB must be fast, durable, and useful through its Rust, Python, and
TypeScript APIs. This initiative turns that direction into an iterative,
evidence-first program. It measures the real cost and quality of canonical
storage, FTS, vectors, graph traversal, hybrids, and public SDK use before
selecting a remediation. It does not make a support or performance claim from a
synthetic fixture, one host, or a green symbol-presence test.

The initiative is cross-release, with its first implementation and measurement
release assigned to 0.8.24 by HITL (`steward` ledger seq-249). It does not
alter the active 0.8.23 CUDA and Memex ladder, and it does not authorize a tag,
registry publication, ANN, a new default retrieval arm, or a schema change.
The 0.8.24 evidence-integration contract is
[`0.8.24-retrieval-performance-evidence-spec.md`](0.8.24-retrieval-performance-evidence-spec.md).

## Product goal

1. Store canonical text and graph data durably in one local SQLite database.
2. Provide correct, measurable FTS, vector, graph, and hybrid retrieval with
   explicit public controls and truthful cost/readiness reporting.
3. Keep the in-library query path local, CPU-default, deterministic, and
   network-free. CUDA accelerates explicit embedding only; it is not a
   retrieval shortcut or a required runtime dependency.
4. Provide one governed capability contract across the Rust facade, Python
   SDK, and TypeScript SDK, backed by functional rather than symbol-only tests.
5. State support only for corpus sizes, hardware, workloads, and knobs that
   have a reproducible measurement artifact.

## Current state and known limits

| Area | What exists | What is not yet established |
| --- | --- | --- |
| Canonical storage | SQLite/WAL canonical nodes, edges, and operational mutations; writes carry projection cursors. | Sustained canonical-write throughput, batch-size curve, storage amplification, and concurrent-write envelope. |
| Text / FTS | FTS5 node and edge indexes, BM25 search, direct text-only search, projection-aware text search, and tokenizer configuration. | A reproducible text-ingest and FTS latency envelope across corpus scales and public knobs. |
| Vector | `sqlite-vec` two-phase binary KNN plus f32 rerank, projection readiness, pre-KNN metadata filters, and vector-equivalence safeguards. | A fresh 0.8.23+ scale characterization and a supported 100k/1M/2M envelope. Earlier 1M observations are decision input, not a current claim. |
| Graph | Durable edges, bounded neighbors and expansion, and graph participation in hybrid retrieval. | Graph write/traversal scale, fan-out behavior, graph-on/vector-off policy, and whether a graph retrieval arm adds value by query class. |
| Hybrid / rerank | FTS/vector fusion, optional recency and cross-encoder paths, explain output, filters, and read views. | A cost/quality matrix for each combination and an explicit public knob inventory. |
| SDKs | Governed Rust facade plus Python and TypeScript bindings with functional tests. | One executable capability-and-cost parity matrix. The Rust facade also documents that `SearchHit` and `IdSpace` are reachable but not nameable. |

## Measurement rules

- Every result records candidate SHA, clean-worktree assertion, schema version,
  Cargo lock digest, OS/CPU/RAM/disk, SQLite/sqlite-vec version, build features,
  embedder identity/revision/dimension, CUDA state when applicable, and full
  command.
- Report process-cold and warm cases separately. Never call a fresh process
  "cold cache" without proving OS-cache state.
- Record raw samples or a lossless reproducible histogram, not only a mean.
  Report p50, p95, p99, minimum, maximum, mean, error/timeout count, and sample
  count for each valid cell.
- Separate acceptance, canonical commit, synchronous FTS/filter projection,
  asynchronous embedding/vector work, `drain`, and query time. A fast query
  after an unreported deferred projection is not an ingestion result.
- Every corpus carries its manifest/hash, license posture, generator version,
  query/gold revision, and intended claim. Missing, partial, or synthetic gold
  is reported as such, never treated as a pass.
- Compare only identical candidate, corpus, hardware class, and treatment.
  Hardware-dependent performance is an observation, not a portable promise.

## Performance measurement matrix

### Ingestion and storage

Each row runs with one feature profile: canonical-only; canonical plus FTS;
canonical plus vector; canonical plus graph; canonical plus FTS/vector/graph;
and each supported projection-registry combination. The dependency columns are
evidence, not knobs silently held constant.

| Workload | Measures | Required dependencies / dimensions |
| --- | --- | --- |
| Canonical text write | documents/s, MiB/s, per-batch p50/p95/p99, commit latency, rollback/error rate | body-size distribution, batch size, WAL/checkpoint policy, durability mode, concurrent writers |
| FTS-enabled write | canonical cost plus synchronous FTS/index delta and FTS database bytes | tokenizer, field/projection specification, text length and language distribution |
| Vector-enabled write | acceptance-to-ready latency, embedding throughput, queue depth, retry/failure count, vector rows versus accepted rows, `drain` latency | embedder identity/revision/dimension, CPU or explicit CUDA device, model-cache state, batch size, projection workers, vector kind |
| Graph write | node and edge writes/s, body-less versus body-bearing edge cost, dangling-edge report count, graph projection/drain cost | degree distribution, edge body size, relation/kind, embedder availability |
| Fully enabled write | end-to-end accepted-to-searchable latency, each phase's contribution, database/WAL/FTS/vector byte growth | all above dimensions, plus mixed node/edge ratio |
| Lifecycle and delete | transition, purge, source erasure, projection cleanup latency and residual-row invariants | active/deleted ratio, graph degree, vector and FTS coverage |

### Read and search

All read measurements report result count and a correctness witness as well as
latency. The request matrix must cover the public Rust, Python, and TypeScript
surfaces; native crossing overhead is reported separately from engine time.

| Request shape | Measures | Required dimensions |
| --- | --- | --- |
| ID and collection reads | `get`, `get_many`, list/filter, collection/mutation paging latency and throughput | hit/miss ratio, page size, filter selectivity, read view, cursor position |
| Direct FTS | text-only p50/p95/p99, QPS, candidate count, result-prefix stability | query length/type, tokenizer, corpus size, limit, filter selectivity, warm/process-cold |
| Vector stage | query embedding time, binary-candidate time, f32 rerank time, end-to-end latency, recall fidelity | embedder/device, corpus size, candidate K, metadata partition/filter, warm/process-cold |
| Graph traversal | neighbors and expansion latency, nodes/edges visited, fan-out, truncation/cap events, QPS | depth, root count, direction, degree distribution, validity/read view |
| Hybrid retrieval | FTS/vector/graph candidate counts, fusion time, deduplication, branch fallbacks, end-to-end latency | enabled arms, filter, limit, graph depth, read view, corpus/query class |
| Reranked / explained retrieval | candidate-pool and per-pair reranker latency, total latency, explanation overhead | reranker model, pool size, CPU/GPU if ever supported, cache state |
| Load and recovery | p50/p95/p99 under mixed readers/writers, tail amplification, lock/busy errors, open/close time | reader/writer mix, concurrency, WAL size, checkpoint policy, process lifecycle |

The exact public knob inventory is itself a deliverable. Initial candidates are
projection role and tokenizer; vector projection and embedder choice; result
limit; typed filter; read view; graph direction/depth; and any existing
reranker/explain configuration. A control absent from a public SDK is recorded
as absent, not silently invented for the benchmark.

## Retrieval-quality measurements

Quality is not one number. Each metric is stratified by corpus, query class,
retrieval arm, K, and relevant feature configuration.

| Metric | What it proves | Preconditions |
| --- | --- | --- |
| Evidence Recall@K / `found@K` | Every required gold evidence item appears in the retrieved set. Primary retrieval-completeness signal. | Versioned qrels with required/supporting evidence and stable document IDs. |
| Hit rate / Success@K | At least one accepted gold item appears. Useful for single-evidence queries. | Binary qrels. |
| Precision@K | Returned results are relevant rather than merely numerous. | Sufficient positive and negative judgments. |
| MRR | Position of the first relevant result. | Ordered binary/graded qrels. |
| nDCG@K | Ranking quality with graded or required/supporting relevance. | Graded qrels and fixed gain/discount rule. |
| MAP | Overall precision across multiple relevant documents. | Complete enough multi-document judgments; do not use on partial qrels. |
| All-bridges-present@K | Every required bridge for a multi-hop question is present. | Explicit multi-hop qrels. |
| Vector fidelity Recall@10 | Approximate vector stage versus exact-f32 vector top-10. This is an implementation-fidelity metric, not IR relevance. | Same embedder, corpus, and exact-f32 ground truth. |
| Answer exact match / token F1 | End-to-end answer correctness after retrieval. | Identical answerer, prompt, context limit, and scoring rules across systems. |
| Abstention and negative false-positive rates | Whether the system declines unsupported questions and avoids fabricated positive answers. | Labeled negative/unanswerable cases. |
| Per-class deltas | Whether a change helps factoid, temporal, multi-hop, knowledge-update, multi-session, or exploratory queries rather than hiding a regression in an aggregate. | Powered class strata and paired comparison records. |

Report candidate coverage separately from final-answer quality. A retrieval
change cannot claim an answer-quality lift from a different answerer, prompt,
or context budget.

## Gold and corpus inventory

| Asset | Current role | Valid measures | Limits / action |
| --- | --- | --- | --- |
| AC-013 deterministic synthetic vector fixture | Capacity and repeatability fixture at 10k/100k/1M. | Vector/query latency, setup and drain timing, deterministic scale characterization. | No natural-language relevance claim; not a gold set. Reuse the 0.8.23 scale protocol rather than create a competing runner. |
| Real BGE corpus (`data/corpus-data/raw/*.jsonl`) and eu7 harness | Vector-quantization fidelity. | Vector-stage Recall@10 versus exact f32; bootstrap CI; dev-box latency/stress reporting. | Distinct from fused-search relevance and end-user quality. Current clean clones may not contain the gitignored corpus. |
| Composite corpus plus 200 cross-document chains | FTS/vector/graph wiring and cross-document retrieval baseline. | Chain coverage, graph/FTS/vector correctness, evidence retrieval. | Roughly 10k documents; explicitly not the capacity benchmark; license/cache posture must be retained. |
| IR-C human fact-level gold (`all.gold.json`) | Candidate-recall CDF across BM25, dense, oracle union, and fused arms at K 50–1000. | Evidence Recall@K / `found@K`, arm CDF, reranker-depth decision. | Corpus/hash and qrels must be present and pinned; synthetic schema fixture is not a substitute. |
| LongMemEval re-pinned memory gold | Agent-memory comparison. | Per-class evidence recall, end-to-end EM/F1, abstention, paired FathomDB–Mem0–naive-RAG deltas. | Preserve real versus generated-top-up provenance and powered class counts. |
| LoCoMo | Conversation-memory, temporal, multi-session, and knowledge-update evaluation. | Per-class retrieval and answer quality after a pinned adapter/gold audit. | Loader exists; a corpus manifest, licensing review, and qrels/answer protocol must be frozen before it gates anything. |
| AP-News BenchmarkQED plus AutoQ | Long-document and cross-document search/sensemaking, including linked questions. | Evidence/answer metrics by activity/data × global/local/linked bucket. | Useful only once the manifest, question version, and relevance/answer scoring are pinned. |
| MuSiQue | Multi-hop bridge retrieval and answer quality. | All-bridges-present@K, per-hop recall, EM/F1, unanswerable false-positive rate. | Existing historical harness is a reference; each new campaign pins its materialized corpus and answerer. |
| Synthetic IR gold fixture | Loader/schema regression coverage. | Gold-schema validation only. | Explicitly illustrative and unpinned; it can never establish product retrieval quality. |

## Iterative execution ladder

1. **Inventory and contract.** Produce a checked measurement manifest: workloads,
   public API calls, knob availability, corpus/gold hashes, provenance fields,
   and invalid-result rules. Add functional cross-SDK capability tests where a
   public operation has no real-database witness.
2. **Baseline.** Capture canonical-only, FTS, vector, graph, and full-stack
   ingest/read results at the supported scale points. Publish versioned,
   fixture-scoped result artifacts; a poor result is a valid baseline, not a
   failed experiment.
3. **Quality baseline.** Run the arm matrix against every eligible gold asset,
   preserving per-query outcomes and class strata. Establish power before
   comparing configurations or competitors.
4. **Diagnose.** Attribute a miss to one bounded layer: canonical commit,
   FTS, embedding/projection, vec0 scan/rerank, graph traversal, fusion,
   reranking, binding overhead, or corpus/gold inadequacy.
5. **Remediate.** Write a separate design and RED test only for the proven
   limiting layer. For a vector scale miss, run the exact-scan overhead
   attribution before considering a packed kernel; ANN/IVF/HNSW remains a
   separate future decision. For a graph-quality miss, prove a per-class gain
   before making graph fusion a default. For an SDK gap, update all governed
   interfaces and parity tests together.
6. **Re-measure and state support.** Compare only equivalent cells, quantify
   regression/improvement, and update the support envelope, documentation, and
   durable result index. A threshold is promoted to a CI gate only after its
   host variability and reproducibility are demonstrated.

## Hardening rules

- Database integration tests use real SQLite/FTS5/sqlite-vec databases; no
  mocked database substitutes.
- Each behavior change is red → green → refactor and receives an independent
  review before a support claim.
- New indexes/projections preserve the canonical-versus-derived split and use
  forward-only schema discipline.
- Performance failures are not hidden through retries, skipped suites, smaller
  fixtures, disabled arms, or unrecorded environmental changes.
- A result from a dirty worktree, missing gold, failed drain, missing vector
  rows, timeout, or incomparable environment is invalid and retained with its
  reason.

## Decisions and open HITL gates

- **2026-08-11, HITL:** create this iterative performance-and-hardening
  initiative and require ingestion, search, retrieval-quality, and SDK-surface
  measures.
- **2026-08-11, HITL / seq-249:** assign the initiative's first implementation
  and measurement work to 0.8.24, while 0.8.23 remains on its CUDA and Memex
  ladder.
- **2026-08-11, HITL / seq-250:** choose the independent performance runner
  with an EARP adapter. EARP records one-run observed cost; the independent
  runner owns repeated sampling and performance claims from the same resolved
  workload.
- **Pending baseline evidence:** set any new numerical latency, throughput,
  storage, or quality support threshold. The existing 10k gate remains intact;
  100k/1M/2M require current measurements before a new promise.
- **Pending gold audit:** select which eligible real corpora become binding
  gates, with licensing, reproducibility, class coverage, and answerer policy
  recorded per campaign.

## Immediate next action

The 0.8.24 evidence-integration foundation has a partial local observed-cost
prototype and a deliberately non-claiming/non-conforming repeated-runner
prototype. Before either may make a complete cost or repeated-performance
claim, they need the workload-manifest, execution-provenance, typed-invalidity,
statistical, and artifact-durability contracts in
[`0.8.24-retrieval-performance-evidence-spec.md`](0.8.24-retrieval-performance-evidence-spec.md).
The next bounded slice is that conformance work, followed by the measurement
manifest and capability matrix. It inventories public Rust/Python/TypeScript
calls and knobs, reuses the existing scale runner and result-artifact
conventions, and produces no performance-support claim or retrieval rewrite.
