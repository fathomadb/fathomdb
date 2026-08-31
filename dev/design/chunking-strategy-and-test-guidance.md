---
status: PROPOSED
---

# Chunking strategy and test guidance

This document preserves the two assistant responses below verbatim. It is a
research and experiment-design capture, not an approved product decision.

## Verbatim response: competitors, mechanisms, and test implications

Yes—though they chunk for different reasons, so their use is not evidence that our 128/96 passage-max-pool design should win.

| Comparator | Chunking model | Relevance to our result |
|---|---|---|
| Microsoft GraphRAG | Yes. It creates configurable TextUnits; the native defaults are 1,200 tokens with 100-token overlap, and uses them for LLM entity/relationship extraction and provenance. Our planned native comparison pins that geometry. [GraphRAG configuration](https://microsoft.github.io/graphrag/config/yaml/), [our comparison plan](/tmp/fathomdb-performance-experiments-20260815/dev/performance-benchmarking/plans/2026-08-11-graphrag-baseline-readiness.md:20) | Different objective: extraction-quality and graph construction, not solely dense passage retrieval fused with document BM25. |
| HippoRAG-2 | Yes, at its input-passage layer. Its official repository explicitly recommends chunking long passages when OpenIE becomes too complex; those passages are then connected in its graph/PPR retrieval. [HippoRAG repository](https://github.com/OSU-NLP-Group/HippoRAG) | More comparable as a passage-based retrieval system, but its graph propagation is a different retrieval mechanism from our independent passage KNN plus max-pool. |
| Mem0 | Not as a fixed sliding-window retrieval architecture. Its public design extracts semantic memory facts from conversations, embeds those facts, and retrieves with vector, BM25, entity, and temporal signals. [Mem0 evaluation architecture](https://docs.mem0.ai/core-concepts/memory-evaluation) | Its effective “chunk” is an extracted memory/fact, so it is much more semantically normalized than a 128-word window. |

Why our approach was flat or negative is fairly well explained by both the data and research:

1. It improved the weak component, not the product. `128/96` improved dense-only exploratory retrieval (`R@10` 0.073 → 0.163), but the fused result remained below text-only: chunked 3:1 fusion was `0.323/0.532` at `R@10/R@50`, versus text-only `0.327/0.533`. In a lexical-dominant corpus, adding a weak dense ranking can displace better BM25 results. [Full result](/tmp/fathomdb-performance-experiments-20260815/dev/plans/runs/IR-C-ws1-fusion-experiment-full.json:1)

2. Our queries often require document-wide discourse context. The recorded diagnosis is that exploratory questions refer to a discussion spread across a long transcript; independently embedding 128-word windows strips the context needed to represent the relevant passage. The chunked dense arm’s median gold rank was 99, versus BM25’s 26. [Findings](/tmp/fathomdb-performance-experiments-20260815/dev/plans/runs/IR-C-retrieval-findings.md:17)

3. Overlap cannot restore missing global meaning. It duplicates boundary context, but each 128-word window is still encoded independently. The late-chunking literature identifies exactly this failure mode: pre-embedding segmentation loses cross-chunk context; it instead encodes the full text first and pools token embeddings into chunks afterward. [Late Chunking paper](https://arxiv.org/abs/2409.04701)

4. Max-pooling is a precision trade-off. It promotes a document if any passage scores well; with overlapping windows, that can amplify a locally topical fragment even when the document is not the best answer. That is an inference from our max-pool design—not a separately measured causal result—but it is consistent with the fused-path regression.

5. The cost trade-off remains unmeasured here. Chunking multiplies vectors, embedding work, index size, and likely candidate pressure; the IR-C study measured quality, not end-to-end ingest/query p50/p95/p99 for each geometry.

The meaningful untested alternative is therefore not “try 96/72 instead of 128/96,” but a distinct **long-context / late-chunking** experiment: preserve document context during encoding, then produce passage-level retrieval units. That remains parked and has not been recorded in `experiments/`.

## Verbatim response: long-context, late chunking, and retrieval-specific mechanisms

## 1. Long-context vs. late chunking

They are different axes.

```text
Naïve chunking
document → split into windows → encode each window independently → one vector/window

Whole-document long-context embedding
document → encode whole document → pool once → one vector/document

Late chunking
document → encode whole document once → contextual token vectors
         → apply window boundaries → pool each window → one contextual vector/window
```

With naïve `128/96`, a window only attends to its own 128 words. The 32-word overlap repeats boundary text, but it does not let a later window understand context introduced 500 words earlier.

Late chunking runs the transformer over the whole document first. Each token representation can incorporate the rest of the document; only then are token vectors pooled into passage vectors. It preserves local retrieval units while retaining document context. This specifically addresses references, definitions, and discourse whose interpretation is outside the local window. [Late Chunking](https://arxiv.org/abs/2409.04701)

Whole-document embedding is different again: it preserves global context but collapses it into one vector, losing localization. Late chunking is the attempt to have both global contextualization and local retrieval.

It is not a free upgrade:

- It needs a genuinely long-context embedding model and access to token-level outputs. BGE-small’s short context and our independent 128-word calls cannot implement it.
- It may be worse for a pure “needle in irrelevant text” query, where global context is mostly noise; the late-chunking paper reports cases where naïve chunking performs better for that shape. [Late Chunking](https://arxiv.org/abs/2409.04701)
- It changes ingest cost and storage shape, so it needs a separate quality-and-timing experiment—not another stride tweak.

Our measured negative only rules out **naïve, independently encoded sliding windows + document max-pooling** under the tested model, pooling, and fusion setup.

## 2. Chunking mechanisms fit particular retrieval strategies

| Retrieval strategy | Chunking mechanism that can help | Why | Main failure mode |
|---|---|---|---|
| Document-level sparse/BM25 | Keep coherent documents or sections; use structure-aware sections where available | Document-level terms and context remain intact; avoids fragmenting a summary/discourse query | Short passage BM25 can fragment co-occurring terms and create duplicate near-identical hits; length normalization changes ranking behavior |
| Passage sparse/BM25 | Small, section/sentence-aligned passages; limited overlap at boundaries | Strong for exact local facts, quotes, and clauses when the answer is localized | Can lose cross-passage conjunctions; overlap duplicates terms and result slots |
| Single-vector dense retrieval | Shorter semantic passages, preferably section/turn boundaries rather than blind windows | Reduces semantic dilution and keeps inputs inside the encoder’s trained context length | Independent chunks lose antecedents and global discourse; a passage can score well locally but be wrong at document level |
| Long-document dense retrieval | Late chunking or whole-document embeddings | Helps questions whose meaning depends on context elsewhere in a long record | Whole-doc pooling loses locality; late chunking needs long-context models and has higher indexing cost |
| Hybrid sparse + dense | Child passages for dense recall, but fold/group to parent document before fusion; optionally return a parent/sibling context window | Lets dense retrieval find a precise local clue while sparse ranking and the answerer retain coherent context | A weak dense arm can displace stronger lexical hits—the failure observed in our IR-C fused result |
| Cross-encoder reranking | Retrieve compact child passages, then rerank a child plus its parent/siblings | The reranker can inspect evidence at high precision without scoring the full corpus | A child-only candidate can omit prerequisite context; use parent expansion for multi-span/discourse questions |
| Multi-vector / late interaction | Passage-bounded indexing plus token-level interaction within each passage | Avoids collapsing every passage into one vector; ColBERT-style systems retain fine-grained query-token/document-token matches. [ColBERT](https://arxiv.org/abs/2004.12832) | Much larger index and retrieval cost; long documents still require segmentation and parent aggregation |
| Graph extraction / GraphRAG | TextUnits sized for entity/relationship extraction, with provenance back to source chunks | The chunk is the LLM extraction unit, not just a dense-retrieval unit; smaller chunks can improve extraction fidelity | Too-small units fragment relations; overlap can duplicate entities/edges and increase extraction cost. Microsoft explicitly trades chunk fidelity against processing time. [GraphRAG dataflow](https://microsoft.github.io/graphrag/index/default_dataflow/) |
| Memory systems / Mem0 | Semantic extraction into facts, episodes, entities, and time-scoped memories | Granularity is aligned to what the system retrieves: a fact or event, not an arbitrary token range | Distillation can omit wording or provenance; it is not a faithful substitute for raw-evidence retrieval |

The important distinction is **what unit is being optimized**:

- Our experiment optimized a dense **passage** signal, then max-pooled it to a **document** and fused it with document-level BM25.
- GraphRAG optimizes a chunk as an **LLM graph-extraction and provenance** unit.
- HippoRAG treats passages as **graph/PPR-connected retrieval units**.
- Mem0’s closest equivalent is an extracted **memory fact/event**.

That is why “competitors chunk” does not imply “use their window length.” The right design follows query shape and retrieval architecture.

For FathomDB’s observed failure, the most relevant alternatives are:

1. **Parent–child retrieval:** retrieve a small evidence child, but score/fuse/answer with its parent or adjacent siblings. This targets localized facts without losing answer context.

2. **Late chunking:** target long-transcript, cross-window discourse queries specifically. It is a different hypothesis than the failed `64/48`–`256/192` naïve-window sweep.

3. **Semantic/turn/session boundaries:** for LOCOMO, conversation-turn or session units may be a more meaningful granularity than fixed words, especially for temporal and multi-session questions.

Any future experiment should pre-register separate metrics for: child evidence recall, parent/document recall, answer correctness, duplicate-result rate, ingestion/index cost, and query p50/p95/p99. That prevents a dense-only passage gain from being mistaken for a product retrieval gain.
