# COMP-03 competitive research: LongMemEval-S answer accuracy

Date: 2026-10-16. Web research only; no repo files were changed.
Legend: **[V]** means I read the number on the primary page or paper. **[S]** means it came only from search snippets or a secondary page, not checked against the primary source. **[I]** means inference.

## 0. Our numbers, reweighted to the full-500 type mix [I, arithmetic]

The full-500 mix is MS 133, TR 133, KU 78, SSU 70, SSA 56, SSP 30, with 30 abstention questions spread across the types.

| Arm | Stratified 60 | Reweighted to the 500 mix | Wilson 95% CI on the stratified 60 |
|---|---|---|---|
| FathomDB FTS (KU 10, MS 5, SSA 10, SSP 8, SSU 10, TR 10) | 53/60 = 0.883 | (78+66.5+56+24+70+133)/500 = **0.855** | [0.78, 0.94] |
| LanceDB FTS (SSP 9, otherwise the same) | 54/60 = 0.900 | (78+66.5+56+27+70+133)/500 = **0.861** | [0.80, 0.95] |
| FathomDB hybrid | 52/60 = 0.867 | not computable (no per-type breakdown supplied); probably about 0.84 | [0.76, 0.93] |

- Reweighting costs about 3 to 4 points, all of it from multi-session at 5/10. MS is 27% of the full benchmark but only 17% of our sample.
- Rough CI on the reweighted figure is about ±8 to 9 pt. That understates the uncertainty, because the 10/10 cells count as zero variance even though a 10/10 Wilson lower bound is only about 0.72.
- Per-type cells of n=10 can't separate systems. The MS 5/10 has a Wilson CI of about [0.24, 0.76].
- The two stores are statistically indistinguishable: a 1-question gap against about 3/60 flips between identical-config runs.

## 1. Published LongMemEval-S results

The "Ingest" column separates raw text (raw turns or sessions stored as-is) from LLM processing at ingest (extracted facts, a graph, or observations).

| System / config | Score | n | Reader | Judge | Ingest | Retrieval unit / k / budget | Source | Self / indep |
|---|---|---|---|---|---|---|---|---|
| LongMemEval paper, full-context GPT-4o (no CoN / CoN) | 0.606 / 0.640 [V] | 500 | gpt-4o | gpt-4o-2024-08-06 (≥97% human agreement) | raw, about 115k tokens | whole history | arxiv.org/html/2410.10813v2 (Fig 3b) | authors |
| LongMemEval paper, Oracle GPT-4o (no CoN / CoN) | 0.870 / 0.924 [V] | 500 | gpt-4o | gpt-4o | evidence sessions only | — | same | authors |
| LongMemEval paper, Llama-3.1-70B full-context / oracle | 0.334 / 0.744 [V] | 500 | L3.1-70B | gpt-4o | raw | — | same | authors |
| LongMemEval paper, RAG (**M** only in the main table) | round K=V: 0.615 (top-5) / 0.670 (top-10); round K=V+fact: 0.657 / 0.720; session K=V+fact: 0.714 / 0.700 [V] | 500, on M | gpt-4o | gpt-4o | raw values; fact-augmented keys | round or session, k=5/10, Stella retriever | same (Table 3) | authors |
| LongMemEval paper, retriever ablation (S-style, recall only) | Round K=V R@10: BM25 0.538, Contriever 0.747, Stella 0.784 [V] | — | — | — | raw | — | same (Table 9) | authors |
| LongMemEval paper, ChatGPT / Coze (97-question small-history subset) | ChatGPT-4o 0.577; Coze-4o 0.330; offline reading 0.918 [V] | 97 | — | — | product memory | — | same (Fig 3a) | authors |
| Zep paper | gpt-4o: 0.712 (full-context 0.602); gpt-4o-mini: 0.638 (full-context 0.554) [V] | 500 | gpt-4o / 4o-mini | gpt-4o | LLM-built temporal knowledge graph (gpt-4o-mini) | 20 edges + nodes, about 1.6k tokens | arxiv.org/html/2501.13956 (Jan 2025) | vendor |
| **Zep, current** | **0.902** (451/500): SSA 96.4, SSU 94.3, SSP 90.0, MS 83.5, KU 93.6, TR 90.2 [V] | 500 | **gpt-5.4** (reasoning=medium) | **gpt-5.4** with CoT grading | LLM graph | 20 edges, 10 nodes, 10 episodes, 5 thread summaries, 5 observations, plus cross-encoder; median 4,408 tokens | getzep.com/research (dateModified 2026-09-29) | vendor |
| Mastra RAG (semantic recall) | topK 2 / 5 / 10 / 20: 0.634 / 0.740 / 0.786 / **0.801** [V] | 500 | gpt-4o | not stated | **raw messages**, chronological | message-level vector, k=2–20 | mastra.ai/research/use-rag-for-agent-memory (2025-07-17) | vendor |
| Mastra Observational Memory | gpt-4o 0.842; gpt-5-mini **0.949**; gemini-3-pro 0.933; gemini-3-flash 0.892 [V] | 500 | as listed | gpt-4o | LLM observer/reflector (gemini-2.5-flash) builds an observation log; no retrieval | about 30k-token context | mastra.ai/research/observational-memory (2026-02-09) | vendor |
| Emergence: Naive RAG | 0.520 [V] | 500 | gpt-4o | not stated | raw turns | turn retrieval | emergence.ai/blog/sota-on-longmemeval-with-rag (Jun 18; year not shown, likely 2025) | vendor |
| **Emergence: Simple** | **0.824**: KU 79.5, MS 73.7, SSA 100, SSP 70, SSU 95.7, TR 81.2 [V] | 500 | gpt-4o-2024-08-06 | not stated | **raw** | **turn matching → whole-session retrieval**, NDCG over turns, cross-encoder rerank, CoT prompt | same | vendor |
| Emergence: Simple Fast / Faster | 0.790 / 0.768 [V] | 500 | gpt-4o (default) | not stated | raw; facts extracted at query time | 42 / 20 turns | same | vendor |
| Emergence: Internal | 0.860 [V] | 500 | not stated | not stated | closed | multi-step rerank | same | vendor |
| Emergence baselines | full-context 4o 0.638; full-context o3 0.760; oracle 4o 0.824; oracle o3 0.920 [V] | 500 | 4o / o3 | not stated | — | — | same | vendor |
| Supermemory | gpt-4o 0.816; gpt-5 0.846; gemini-3-pro 0.852 [V via the Hindsight table and the Supermemory page] | 500 | as listed | gpt-4o | chunking + atomic memories + relations + dates (LLM extraction implied) | k=20, hybrid | supermemory.ai/research | vendor. The page's "gpt-4o 97%" row equals its Recall@20 table, so it is suspect. The 99% post is self-described parody. |
| Hindsight | OSS-20B 0.836; OSS-120B 0.890; Gemini-3 Pro reader **0.914** [V] | 500 | as listed | **GPT-OSS-120B** (non-standard) | LLM fact extraction + entity graph | 4-way recall (semantic, BM25, graph, temporal) + RRF + rerank; 8,192-token budget (per a Zep page [S]) | arxiv 2512.12818 (Dec 2025) | vendor; co-authored with Virginia Tech and the Washington Post |
| Mem0 (managed platform, new algorithm) | 0.934 (April 2026) → **0.944** [V for the headline only] | 500 [S] | **not published** | **not published** | LLM single-pass, ADD-only extraction | about 6.9k tokens, top_200 [S] | mem0.ai/research-3; mem0.ai/blog/mem0-the-token-efficient-memory-algorithm | vendor; config not published |
| Mem0 (older; MemOS harness) | 0.664 [V] | — | not stated (likely gpt-4o-mini) | — | LLM extraction | — | HF MemTensor/MemOS_eval_result | competitor-run |
| MemOS harness, other systems | MemOS 0.778; Memobase 0.724; Zep 0.638; Supermemory 0.584; MIRIX 0.435; MemU 0.384 [V] | — | not stated | — | — | — | same | MemOS vendor |
| EverMemOS | 0.83 (MemReader table); 0.826 (EvalMem); 0.67 to 0.78 with other backbones (MemForest) [S] | 500 | gpt-4.1-mini (EverMind) [S] | — | LLM | — | arxiv 2604.07877, 2609.22231, 2605.23986 | mixed |
| A-Mem | 0.632 (gpt-4o-mini) [S] | — | gpt-4o-mini | — | LLM notes | — | arxiv 2608.12990 (LycheeMemory) | third party |
| LightMem | 0.729 [S] | — | GPT (backbone unconfirmed) | — | LLM | — | arxiv 2608.12990 / 2510.18866 | mixed |
| ContextStream | 0.900 (0.896 single-shot) [V] | 500 | gpt-5.5, self-consistency k=3 | gpt-4o-2024-08-06 | not stated | — | contextstream.io/benchmarks (Jun 2026) | vendor |
| Honcho | 0.926 [S] | — | — | — | — | — | Exabase press release (May 2026) | competitor-reported |
| Exabase M-1 | 0.964 at top-50 [S] | — | — | — | — | — | hpcwire Exabase press release | vendor |
| ByteRover | 0.928 [S] | — | — | Gemini-3 judge (non-standard) | — | — | byterover.dev blog | vendor |
| Agent Zero Memory | 0.956 [S] | — | 8 backbones within 3.4 pt | — | — | — | arxiv 2608.29606 | authors |
| agentmemory (JordanMcCann) | 0.962 (481/500) [S] | 500 | — | — | — | — | github JordanMcCann/agentmemory | unverified |
| MemDelta controlled study | verbatim RAG 0.472 vs full-context 0.498 (gpt-4o-mini); embedding swap alone +6.2 pt; Mem0 +11 pt over MiniLM-RAG but −1.2 pt vs cloud-embedding RAG [V abstract] | 500 | gpt-4o-mini / Gemini / Sonnet | — | raw vs Mem0 | — | arxiv 2606.29914 | **independent** |
| LanceDB hermes-agent-memory | session-search 0.53; vector 0.66; hybrid-rrf 0.61; hybrid-linear 0.61; hybrid + cross-encoder 0.68 [V] | **60 stratified** (10 per type) | **gpt-5.4** | gpt-5.4-mini | raw messages | **top-5 messages** | lancedb.com/blog/semantic-memory-for-hermes-agent-with-lancedb (2026-06-16) | vendor |

### Not found

- **Letta / MemGPT.** No first-party LongMemEval number (Letta's 74.0 is LoCoMo). The Zep paper says MemGPT could not complete the benchmark. A third-party "83.2%" is unsourced [S].
- **LangMem.** No LongMemEval-S result; it appears only on LoCoMo (MemOS paper).
- **MemoryOS.** No LongMemEval accuracy found. EvalMem has diagnostic shares only [S].
- **cognee.** Publishes BEAM results, not LongMemEval.
- **OpenAI memory.** Only the paper's 97-question ChatGPT probe (0.577 / 0.711).
- **TiM.** Nothing found.
- **Databases** (Chroma, Weaviate, Qdrant, pgvector, Elasticsearch, SurrealDB, Milvus, Pinecone). **No end-to-end QA result names any of them.** Only retrieval recall exists:
  - MemPalace raw ChromaDB + MiniLM: R@5 0.966 at session level.
  - agentmemory BM25: R@5 0.862.
  - agentmemory BM25 + vector: R@5 0.952.
- **LongMemEval-M.** The only end-to-end numbers found are the paper's Table 3 RAG results (best about 0.72, gpt-4o). No vendor publishes M.

## 2. Assessment

### (a) Which results are actually comparable

Comparability is judged against our setup: S, raw ingest, a strong reader, a budget of several thousand tokens.

**Closest on store or ingest design (raw text, retrieve, expand):**

- Emergence Simple (0.824, gpt-4o). Turn-level match, then whole-session retrieval with rerank; architecturally almost the same as our round → session expansion.
- Mastra RAG topK 20 (0.801, gpt-4o, raw messages).
- LongMemEval paper round-level RAG (0.67 to 0.72, gpt-4o, but on **M**).
- Emergence Naive RAG (0.52).
- MemDelta verbatim RAG (0.47, gpt-4o-mini).
- **All of them use a weaker reader than gpt-5.4.**

**Closest on reader:**

- Zep 0.902 (gpt-5.4 reader and judge, full 500, about 4.4k tokens) is the single most comparable published number. Its ingest differs (an LLM-built graph).
- LanceDB's own Hermes run (gpt-5.4, the same stratified 60 design, raw messages) differs mainly in evidence budget (top-5 messages, no session expansion) and judge (gpt-5.4-mini).

**Not comparable:**

- Mem0 0.944 (reader and judge unpublished).
- ByteRover (Gemini judge).
- Hindsight (GPT-OSS-120B judge).
- Exabase, Honcho, agentmemory, Agent Zero (unverified).
- Supermemory's "97%" (a recall metric).
- MemPalace (recall metric).

### (b) How much of the spread comes from reader, judge and evidence budget

**Reader model, roughly +8 to +12 pt from gpt-4o to a frontier model at fixed memory** [V for each pair]:

- Emergence full-context: 63.8 → 76.0 (o3).
- Emergence oracle: 82.4 → 92.0 (o3).
- Mastra OM: 84.2 → 94.9 (gpt-5-mini).
- Supermemory: 81.6 → 85.2.
- Hindsight: 83.6 → 91.4.

**Evidence budget, up to about +17 pt at a fixed reader and store:**

- Mastra RAG: 63.4 (k=2) → 80.1 (k=20).
- LongMemEval M round RAG: 0.615 → 0.670 (k=5 → 10).
- LanceDB Hermes top-5 messages: 0.53 to 0.68 with gpt-5.4. Our session-expanded 30k-char context on the same dataset, the same reader and the same 60-question design gives 0.87 to 0.90. [I] That 20 to 35 pt gap is mostly retrieval unit and budget, not store.

**Retriever or embedder:**

- MemDelta: an embedding swap alone moved accuracy 6.2 pt.
- Within COMP-03, three different retrievers land within 2/60 of each other. That is inside noise.

**Judge:**

- The standard gpt-4o judge has about 97 to 98% human agreement (paper Table 6). Preference and abstention are the least reliable judge categories (0.90).
- Judge choice (gpt-5.4, gpt-5.4-mini, OSS-120B, Gemini) is a further few-point confound that no source quantifies. [I]

**Overall [I]:** most of the 0.5 → 0.95 spread across published systems is explained by reader generation plus evidence budget. Store or index choice is second-order once retrieval recall is high. Session-level R@10 is about 0.95 to 0.99 even for BM25 or MiniLM on S.

### (c) Is 0.87 to 0.90 on a stratified 60 with gpt-5.4 competitive?

**Yes, in the main pack. Not leading, and not proven.**

- **Reweighted** to the 500 mix, we are about 0.855 to 0.861 (CI about ±8 to 9 pt).
- **Same reader (gpt-5.4):**
  - Zep reports 0.902 on the full 500. We sit about 4 pt below that point estimate, inside our CI.
  - Zep's MS is 83.5 versus our 5/10. Multi-session is the one category where an aggregation-oriented memory layer clearly helps.
- **Raw-ingest RAG systems with gpt-4o** (Emergence Simple 0.824, Mastra RAG 0.801): we beat them by 4 to 6 pt. [I] But the reader upgrade alone is worth about 8 to 12 pt, so raw retrieve-and-expand on these stores is probably roughly on par with those designs, not better.
- **Oracle:** we are close to Oracle-GPT-4o (0.82 to 0.87) and below Oracle-o3 (0.92), the ceiling for a frontier reader given perfect evidence.
- **Frontier-reader systems:** we are below Mastra OM gpt-5-mini (0.949), Hindsight Gemini-3 (0.914) and Mem0's unspecified 0.944. All of these use LLM processing at ingest, and two use a different judge.
- **Gap versus the best frontier-reader systems:** about 5 to 9 pt at the point estimate. That is within or near our CI, and concentrated in MS (and, per the small cells, SSP).

**Caveats:**

1. With n=60 and roughly 3 of 60 answers flipping between identical runs, FathomDB and LanceDB are statistically tied. No ranking against any vendor is supported at better than ±8 pt.
2. Stratification flatters us by about 3 pt versus the 500 mix.
3. Several factors are confounded, with unknown sign: the gpt-5.4 judge versus the standard gpt-4o judge; the cleaned dataset versus the original `longmemeval_s` used by most published runs; and question-date injection.
4. Our evidence budget (about 30k chars, roughly 7 to 8k tokens) is larger than Zep's 4.4k tokens, similar to Hindsight's 8k, and much smaller than full context.
5. Every competitor number is vendor-self-reported, except MemDelta (independent, but weak readers) and Hindsight's co-authored validation.

## 3. Source catalog

Each entry gives: URL · publisher and date · what is measured · dataset · metrics · method · self-reported or independent. "Opened" means I read the page; "Snippet only" means I saw it only in search results, not verified.

1. <https://arxiv.org/html/2410.10813v2> · Wu et al., LongMemEval (ICLR 2025, 2024-10)
   - Measures: long-context LLMs, RAG designs, ChatGPT, Coze. Dataset: LongMemEval S/M/oracle.
   - Metrics: QA accuracy, Recall/NDCG@k. Method: gpt-4o judge; round/session values; fact keys; time-aware query expansion. Authors. Opened.
2. <https://arxiv.org/html/2501.13956> · Zep (Rasmussen et al., 2025-01)
   - Measures: Zep vs full context. Dataset: LongMemEval-S (and DMR).
   - Metrics: accuracy, latency, context tokens. Method: gpt-4o / 4o-mini reader, gpt-4o judge, LLM graph. Vendor. Opened.
3. <https://www.getzep.com/research/> · Zep (modified 2026-09-29)
   - Measures: Zep. Datasets: LongMemEval (0.902), LoCoMo (0.947).
   - Metrics: accuracy, latency p50/p95, median tokens. Method: gpt-5.4 reader and CoT judge. Vendor. Opened.
4. <https://www.getzep.com/vectorize-hindsight-alternative/> · Zep (2026)
   - Measures: Zep vs Hindsight. Dataset: LongMemEval and LoCoMo.
   - Metrics: accuracy, token budget (Zep 4.4k vs Hindsight 8.2k). Competitor characterization. Opened.
5. <https://arxiv.org/pdf/2512.12818> · Vectorize, Virginia Tech, Washington Post (Hindsight, 2025-12)
   - Measures: Hindsight vs full context, Zep, Supermemory. Datasets: LongMemEval-S (per type), LoCoMo.
   - Metrics: accuracy. Method: GPT-OSS-120B judge; LLM fact extraction; 4-way retrieval. Vendor plus academic. Opened (pdftotext).
6. <https://github.com/vectorize-io/hindsight-benchmarks> · Vectorize
   - Measures: Hindsight vs others. Datasets: LongMemEval, LoCoMo. Metric: accuracy. Vendor. Snippet only.
7. <https://hindsight.vectorize.io/blog/2026/03/23/agent-memory-benchmark> · Vectorize (2026-03-23)
   - Introduces AMB (Agent Memory Benchmark), with single-call and agentic modes. Vendor. Snippet only.
8. <https://mastra.ai/research/observational-memory> · Mastra (2026-02-09)
   - Measures: Mastra OM plus a cited leaderboard. Dataset: LongMemEval-S, 500 questions.
   - Metric: accuracy per type. Method: gpt-4o judge; observer gemini-2.5-flash; readers gpt-4o / gpt-5-mini / gemini-3. Vendor. Opened.
9. <https://mastra.ai/research/use-rag-for-agent-memory> · Mastra (2025-07-17)
   - Measures: Mastra semantic-recall RAG and working memory. Dataset: LongMemEval-S, 500.
   - Metric: accuracy per type, topK sweep. Method: gpt-4o reader, raw messages. Vendor. Opened.
10. <https://cdn.jsdelivr.net/npm/@mastra/longmemeval@1.1.10/README.md> · Mastra
    - Benchmark harness README with S/M descriptions; no scores seen. Snippet only.
11. <https://www.emergence.ai/blog/sota-on-longmemeval-with-rag> · Emergence AI (Jun 18, likely 2025)
    - Measures: EmergenceMem variants, naive RAG, full context, oracle (gpt-4o, o3). Dataset: LongMemEval-S, 500.
    - Metrics: accuracy per type, latency. Judge not stated. Vendor. Opened.
12. <https://www.emergence.ai/blog/emergence-is-the-new-new-state-of-the-art-in-agent-memory> and <https://www.emergence.ai/blog/og2q4h3p2zcmkxjdvj099ogqm21a4a> · Emergence AI
    - Follow-up SOTA posts (86%). Vendor. Snippet only.
13. <https://supermemory.ai/research> and <https://supermemory.ai/research/longmembench.md> · Supermemory (May 2026)
    - Measures: Supermemory vs Zep and full context. Dataset: LongMemEval-S.
    - Metrics: Recall@20 plus judge accuracy (the gpt-4o row is suspect). Method: gpt-4o judge, k=20. Vendor. Opened.
14. <https://blog.supermemory.ai/we-broke-the-frontier-in-agent-memory-introducing-99-sota-memory-system/> · Supermemory
    - "99%" ensemble claim, self-described parody. Vendor. Snippet only.
15. <https://supermemory.ai/blog/longmemeval-benchmark-what-it-tests/> · Supermemory
    - Benchmark explainer; notes the cleaned-data revision issue. Snippet only.
16. <https://mem0.ai/research-3> and <https://mem0.ai/blog/mem0-the-token-efficient-memory-algorithm> · Mem0 (Apr–May 2026; page modified 2026-10-13)
    - Measures: Mem0 platform. Datasets: LongMemEval 94.4, LoCoMo 92.5, BEAM 64.1 / 48.6.
    - Metrics: accuracy, tokens per call. Reader and judge unpublished. Vendor. Opened (headline only).
17. <https://mem0.ai/blog/the-token-efficient-memory-algorithm-now-has-temporal-reasoning> and <https://pypi.org/project/mem0ai/2.0.13/> · Mem0
    - 93.4 → 94.4 history; old 67.8; top_200. Vendor. Snippet only.
18. <https://mem0.ai/blog/ai-memory-benchmarks-in-2026> · Mem0 (2026)
    - Aggregator: Mem0, ByteRover, Zep, EverMemOS on LongMemEval and LoCoMo / BEAM. Vendor. Opened.
19. <https://mem0.ai/blog/state-of-ai-agent-memory-2026> · Mem0
    - Survey of benchmarks. Vendor. Snippet only.
20. <https://huggingface.co/datasets/MemTensor/MemOS_eval_result/blob/refs%2Fpr%2F1/README.md> · MemTensor (MemOS)
    - Measures: MemOS, Mem0, Zep, Memobase, Supermemory, MIRIX, MemU. Dataset: LongMemEval.
    - Metrics: accuracy per type, tokens. Reader not stated. Vendor-run. Opened.
21. <https://arxiv.org/pdf/2507.03724> · MemOS paper (2025-07)
    - Measures: MemOS vs LangMem, Zep, OpenAI, Mem0. Dataset: LoCoMo. Metric: LLM-judge score. Vendor. Snippet only.
22. <https://arxiv.org/pdf/2506.06326> · MemoryOS (2025-06)
    - Measures: MemoryOS, A-Mem. Dataset: LoCoMo. Metrics: F1 / BLEU. Authors. Snippet only.
23. <https://evermind.ai/blogs/a-unified-evaluation-framework-for-ai-memory-systems> · EverMind (2026)
    - Measures: EverOS vs Mem0, MemOS, Zep, MemU. Datasets: LoCoMo, LongMemEval. Method: gpt-4.1-mini. Vendor. Snippet only.
24. <https://evermind.ai/zh/blogs/top-ai-memory-systems-benchmarked-in-2026> · EverMind
    - Cites Zep LongMemEval 90.2 / LoCoMo 94.7. Secondary. Snippet only.
25. <https://arxiv.org/pdf/2604.07877> (MemReader), <https://arxiv.org/pdf/2605.23986> (MemForest), <https://arxiv.org/pdf/2609.22231> (EvalMem) · academic, 2026
    - Measures: EverMemOS, MemOS, Zep, Mem0, MemU and others. Dataset: LongMemEval-S.
    - Metrics: accuracy; EvalMem adds failure-layer diagnostics. Third party. Snippet only.
26. <https://arxiv.org/pdf/2602.15313> · Mnemis (2026-02)
    - Measures: Mnemis vs MemOS, Mem0, Zep, EverMemOS. Datasets: LoCoMo and LongMemEval. Method: gpt-4o-mini / 4.1-mini. Authors. Snippet only.
27. <https://arxiv.org/pdf/2608.12990> · LycheeMemory V2 (2026-08)
    - Measures: A-Mem 63.2, LightMem 72.9, MemOS 77.8. Dataset: LongMemEval-S. Method: gpt-4o-mini. Third party. Snippet only.
28. <https://arxiv.org/pdf/2510.18866> · LightMem (2025-10)
    - Measures: LightMem vs A-Mem and others. Dataset: LongMemEval-S. Metrics: accuracy, tokens, cost. Authors. Snippet only.
29. <https://arxiv.org/pdf/2608.03463> (LeanMem) and <https://arxiv.org/pdf/2605.06132> (MemReranker) · academic 2026
    - LongMemEval / LoCoMo accuracy and construction tokens. Snippet only.
30. <https://arxiv.org/abs/2606.29914> · MemDelta (2026-06)
    - Measures: verbatim RAG, full context, Mem0, self-memory. Dataset: LongMemEval-S, 500.
    - Metrics: accuracy with significance tests. Method: gpt-4o-mini, Gemini, Sonnet; embedding-swap ablation. **Independent.** Opened (abstract).
31. <https://arxiv.org/pdf/2604.23878> · ZenBrain (2026-04)
    - Measures: ZenBrain vs Letta, A-Mem, Mem0. Dataset: LongMemEval (500). Metric: P@5; 11.8% of Letta queries errored. Authors. Snippet only.
32. <https://arxiv.org/pdf/2605.15701> · H-Mem (2026-05)
    - Measures: H-Mem vs Mem0, Zep with shared embedder / reranker / judge. Datasets: LoCoMo, LongMemEval-S. Authors. Snippet only.
33. <https://arxiv.org/pdf/2608.29606> · Agent Zero Memory (2026-08)
    - Claims 95.6% on LongMemEval across 8 backbones. Authors. Snippet only.
34. <https://arxiv.org/pdf/2604.22085> · Memanto (2026-04)
    - Vector-only system, 89.8% on LongMemEval [S]. Authors. Snippet only.
35. <https://arxiv.org/pdf/2608.13334> · RippleMem (2026-08)
    - LongMemEval accuracy for a memory system. Snippet only.
36. <https://arxiv.org/pdf/2603.04814> · "Beyond the Context Window" (2026-03)
    - Measures: Mem0 fact memory (pgvector footnote) vs long context. Metrics: cost and performance. Snippet only.
37. <https://arxiv.org/pdf/2604.01707> · "Memory in the LLM Era" survey (2026-04)
    - Notes that Zep index construction on LongMemEval did not finish in 2 days. Third party. Snippet only.
38. <https://arxiv.org/pdf/2604.21284> · MemPalace critique (2026-04)
    - Measures: MemPalace / raw ChromaDB. Dataset: LongMemEval-S. Metric: recall_any@5 (not QA). Independent. Snippet only.
39. <https://vectorize.io/articles/mempalace-benchmarks> and <https://vectorize.io/articles/mempalace-review> · Vectorize (2026)
    - MemPalace on LongMemEval R@5 and ConvoMem; overfitting critique. Competitor. Snippet only.
40. <https://github.com/rohitg00/agentmemory/blob/main/benchmark/LONGMEMEVAL.md> · rohitg00
    - Measures: BM25, BM25+vector (MiniLM), MemPalace raw. Dataset: LongMemEval-S, 500.
    - Metrics: R@5/10/20, NDCG, MRR; retrieval only, no reader. Self. Opened.
41. <https://cdn.jsdelivr.net/gh/rohitg00/agentmemory@main/benchmark/COMPARISON.md> · rohitg00
    - Cross-system comparison table. Snippet only.
42. <https://github.com/JordanMcCann/agentmemory> · J. McCann
    - Claims 96.2% (481/500) on LongMemEval. Self. Snippet only, unverified.
43. <https://www.lancedb.com/blog/semantic-memory-for-hermes-agent-with-lancedb> · LanceDB (2026-06-16)
    - Measures: Hermes session-search vs LanceDB vector / hybrid / cross-encoder. Dataset: LongMemEval-S, stratified 60.
    - Metrics: accuracy, R@5, MRR@5, latency. Method: gpt-5.4 reader, gpt-5.4-mini judge, top-5 raw messages. Vendor. Opened.
44. <https://docs.lancedb.com/integrations/ai/hermes-agent.md> · LanceDB
    - Summary of 0.66 / 0.53. Vendor. Opened.
45. <https://www.lancedb.com/blog/openclaw-memory-from-zero-to-lancedb-pro> · LanceDB
    - Measures: OpenClaw memory-core 52%, LanceDB vector 76%, LanceDB-pro + cross-encoder 80%. Dataset: LoCoMo. Metric: accuracy. Vendor. Snippet only.
46. <https://www.letta.com/blog/benchmarking-ai-agent-memory> · Letta (2025-08-12)
    - Measures: Letta filesystem 74.0%. Dataset: LoCoMo. Metric: accuracy. Vendor. Snippet only.
47. <https://www.cognee.ai/competitor/mem0> and <https://docs.cognee.ai/reference/benchmarks> · cognee
    - Measures: cognee vs Mem0. Dataset: BEAM 100K / 10M (cognee 67% vs Mem0 48.6% at 10M). Vendor. Snippet only.
48. <https://www.cognee.ai/ai-memory-evals-0825> · cognee (2025-08)
    - Memory evals essay (HotpotQA-style). Snippet only.
49. <https://contextstream.io/benchmarks> · ContextStream (Jun 2026)
    - Measures: ContextStream 90.0, plus cited Zep, ByteRover, Supermemory. Dataset: LongMemEval.
    - Method: gpt-5.5 reader with self-consistency k=3; gpt-4o judge. Vendor. Opened.
50. <https://www.byterover.dev/blog/benchmark_ai_agent_memory_real_production_byterover_top_market_accuracy_longmemeval> · ByteRover
    - Claims 92.8% on LongMemEval-S with a Gemini judge. Vendor. Not opened.
51. <https://www.hpcwire.com/bigdatawire/this-just-in/exabase-achieves-highest-reported-score-on-leading-ai-memory-benchmark-using-a-smaller-cheaper-model/> · Exabase press release (2026-05-26)
    - Claims M-1 96.4% at top-50; lists Mem0 94.8, Honcho 92.6, HydraDB 90.79. Vendor. Snippet only.
52. <https://recallium.ai/benchmarks> · Recallium
    - LongMemEval Hit@10 99.8% (retrieval). Vendor. Snippet only.
53. <https://atlan.com/know/best-ai-agent-memory-frameworks-2026/> and <https://atlan.com/know/mem0-alternatives/> · Atlan (2026)
    - Aggregator; notes 0 of 8 frameworks independently verified. Secondary. Opened (first).
54. <https://dev.to/everest_an/-i-benchmarked-ai-agent-memory-in-2026-and-the-numbers-tell-a-different-story-than-the-marketing-2ae4> · DEV.to
    - Claims Mem0 OSS scores 32.4% on LongMemEval. Independent blog. Snippet only, unverified.
55. <https://academy.studiomeyer.io/concepts/longmemeval> · StudioMeyer
    - Aggregator (Mastra OM 94.87, Hindsight 91.4, Zep 63–71, Mem0 49). Unsourced. Snippet only.
56. <https://blog.getzep.com/gpt-4-1-and-o4-mini-is-openai-overselling-long-context/> · Zep (2025)
    - gpt-4.1 / o4-mini long-context results, likely on LongMemEval. Vendor. Not opened.
57. <https://blog.getzep.com/state-of-the-art-agent-memory/> · Zep (2025)
    - Original 71.2% SOTA post. Vendor. Not opened.
58. <https://penfieldlabs.substack.com/p/we-audited-locomo-64-of-the-answer> · Penfield Labs
    - LoCoMo audit: 6.4% of the answer key is wrong, and the judge accepts up to 63% of intentionally wrong answers. Independent. Snippet only.
59. <https://arxiv.org/pdf/2605.12493> · LongMemEval-V2 (2026-05)
    - New, harder benchmark (AgentRunbook-C 72.5%). Authors. Snippet only.
60. <https://github.com/garrytan/gbrain-evals/blob/main/docs/comparison-systems.md> · gbrain-evals
    - Comparison-systems list. Snippet only.
61. <https://www.memoryplugin.com/wiki/evaluating.html> and <https://www.wizwand.com/dataset/longmemeval> · aggregators
    - Not opened.
