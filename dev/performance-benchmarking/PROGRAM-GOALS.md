# Performance benchmarking and experiment program goals

The central hypothesis should be: FathomDB becomes world-class not by choosing one “best” chunk size or retrieval algorithm, but by making an agent’s durable memory a provenance-preserving canonical record with multiple, selectively materialized retrieval projections.

The chunking result is a warning against treating passage-vector retrieval as universally beneficial: independently embedded `128/96` windows improved dense-only retrieval, yet the fused result remained slightly below text-only retrieval. For conversational, discourse-heavy personal memory, a weak dense arm can displace a strong lexical result. The current L0 plan sensibly starts with a smaller, more revealing question: whether turns, sessions, source-aware neighbors, hybrid retrieval, and cross-encoder reranking earn their latency and quality costs on LOCOMO. [Chunking findings](/tmp/fathomdb-performance-experiments-20260815/dev/design/chunking-strategy-and-test-guidance.md) · [L0 campaign](/tmp/fathomdb-performance-experiments-20260815/dev/performance-benchmarking/2026-08-14-locomo-fathomdb-capability-campaign-plan.md)

## Product hypothesis

Keep one canonical memory record—raw text/payload, stable identity, source provenance, timestamps, lifecycle/supersession state, access scope—and derive projections only when they serve a named retrieval job.

That yields a useful split:

| Need | Primary representation | Retrieval |
|---|---|---|
| Exact facts, names, quotes, recent messages | Canonical text; turn/session metadata | FTS/BM25, temporal and source filters |
| Semantic paraphrase and fuzzy recall | Child passage or turn vectors | Dense retrieval, then parent expansion |
| Cross-session personal memory | Session/episode summaries plus linked source turns | Hybrid lexical+dense, recency weighting, rerank |
| Temporal questions | Time-scoped facts/events and source records | Temporal filtering plus lexical/hybrid retrieval |
| Multi-hop “who/why/relationship” questions | Extracted entities/claims/edges with provenance | Lexical seeds → bounded graph expansion → rerank |
| Global synthesis | Documents/sessions plus summaries or graph communities | Coverage-oriented retrieval and map-reduce only when justified |

The crucial property is reversibility and attribution: an extracted fact, vector child, graph edge, summary, or embedding should point back to an erasable canonical source record. Derived memories must not become an untraceable second database. That aligns with FathomDB’s lifecycle, source-erasure, and projection-registry direction—not merely better search. [Rust interface](/tmp/fathomdb-performance-experiments-20260815/dev/interfaces/rust.md)

## Multiple approaches, as needed

| Approach | Opportunity | Costs / failure modes | When it earns activation |
|---|---|---|---|
| Parent–child retrieval | Dense search finds a precise child; return/rerank its parent and adjacent turns so the answerer has context | More records, duplicate candidates, aggregation policy; parent expansion can inflate latency | Local-fact and conversational questions where child evidence recall rises without degrading parent recall or p95 |
| Semantic turn/session boundaries | Natural fit for dialogue, temporal state, speaker attribution, and provenance | A turn can be too small; a whole session can dilute a local answer; segmentation quality becomes a dependency | LOCOMO factoid, temporal, and multi-session classes should decide turn vs session rather than a universal default |
| Late chunking | Contextual passage vectors may address cross-window references and long discourse without losing localization | Needs a long-context model and token-level outputs; materially different ingest/storage cost; can harm needle-in-haystack cases | A separately pre-registered long-transcript experiment, not another stride sweep |
| FTS/BM25 | Fast, explainable, strong on exact language, local-first, and presently a proven baseline | Synonyms/paraphrases and semantically implicit recalls can fail | Always retain as a first-class arm and baseline; never assume dense should replace it |
| Dense vectors | Paraphrase and semantic recall | Embedder dependence, index/ingest cost, possible hybrid displacement, current scan scaling limits | Enable only after the configuration clears a retrieval and latency eligibility rule against FTS |
| Cross-encoder rerank | Can turn broad candidate recall into high precision; prior internal work suggests precision is a meaningful lever | GPU/CPU cost, candidate-pool sensitivity, potential tail-latency penalty | Apply to a bounded candidate set after recall is established, with a fast profile that can decline it |
| Graph projection | Supports explicit relationships, traversal, provenance, and some multi-hop retrieval | Extraction cost/error, stale or co-mingled entities, graph traversal does not automatically beat BM25 | Build from high-confidence, versioned facts; seed from lexical/hybrid hits rather than graph-only search |
| Extracted semantic memory | Compact facts, preferences, episodes, procedures; supports update/merge/supersession | Loss of wording/context, hallucinated extraction, difficult deletion and conflict handling | Treat as a derived, auditable projection with confidence and source links—not a replacement for raw memory |

I would avoid an opaque system that autonomously chooses all of these without visibility. Offer a stable default—likely FTS plus bounded source-aware context—and let an agent request a profile or state its intent: `exact`, `semantic`, `timeline`, `relationship`, `global`, or `fast`. FathomDB can then expose the selected projections, candidates, expansion, and rerank contribution through its explain path. Over time, a learned or rules-based router can recommend profiles, but should be evaluated as its own component and retain an override/fallback.

## Gold-data portfolio

LOCOMO should be the primary personal-memory anchor, but not the only judge:

- **LOCOMO:** turn/session choice, temporal recall, multi-session recall, factoid recall, answer quality. It has no knowledge-update category.
- **LongMemEval:** retain for knowledge-update behavior, while acknowledging the current power constraint.
- **IR-C and BEIR:** retrieval fidelity, exact/exploratory/negative queries, lexical-vs-dense behavior—retrieval only, not answer correctness.
- **MuSiQue / HippoRAG-style protocol:** multi-hop supporting-evidence recall and answer F1.
- **TimelineQA, TimeQA, ToT:** temporal ordering, changing beliefs/preferences, and time-scoped validity.
- **AP-News/AutoE and SummHay:** global sensemaking, coverage, citation quality, and the point at which a graph or map-reduce treatment has earned its high cost.
- **ELPS:** extraction/projection conformance only; not a retrieval-quality proxy.

This directly follows the program’s separation of retrieval quality, answer/task quality, cost, and fidelity/scale—and prevents a local dense-recall gain from being mistaken for a personal-memory product win. [Program](/tmp/fathomdb-performance-experiments-20260815/dev/performance-benchmarking/PROGRAM.md) · [Available gold data](/tmp/fathomdb-performance-experiments-20260815/dev/performance-benchmarking/README.md)

The near-term priority is therefore not to build late chunking or a full graph system now. Complete L0 with provenance, turn/session treatments, neighbor expansion, hybrid and bounded rerank; characterize each class and cost cell; then use the failures to commission one next mechanism. The evidence already supports that FTS is a serious baseline, reranking may be a precision lever, and graph or late-chunking capabilities need query-shape-specific proof rather than architectural faith.
