---
title: Long-record embedding — size of the problem and choice of fix
date: 2026-10-18
status: PROPOSED
desc: Measures how much of each consumer corpus lies past bge-small's 512-wordpiece cut, shows with a preregistered probe that the cut hurts the vector arm but not hybrid retrieval today, costs six fixes in vectors, embed time, index size, query latency, schema and snippet anchor, and recommends a guard now plus one gated overflow-chunking experiment.
inputs:
  - dev/plans/prompts/long-record-embedding-prompt.md
  - dev/design/agent-access/match-centred-snippets.md
  - dev/design/mcp-adapter/engine-proposals/EP-17-match-centred-snippets.md
  - dev/design/agent-access/hints-sidecar.md
  - dev/design/agent-access/queue-admission.md
  - dev/experiments/queue-sim/REPORT.md
  - dev/design/chunking-strategy-and-test-guidance.md
  - dev/plans/runs/IR-C-retrieval-findings.md
  - dev/plans/runs/IR-C-ws1-fusion-experiment-full.json
  - dev/design/1-2M-chunk-scaling-DECISION-BRIEF.md
  - dev/adr/ADR-0.6.0-vector-identity-embedder-owned.md
  - dev/experiments/long-record-embedding/
---

# Long-record embedding

Paths are relative to `src/rust/crates/` unless given in full. Every number
below comes from `dev/experiments/long-record-embedding/results/` unless a
source is cited. Measurements ran on 2026-10-18 against the `exp/agent-access`
tree (`4ff9239ef`), built as a CUDA wheel, with the RTX 3090 on `cuda:0`.

## 1. Summary

- **Size.** The problem depends on the record unit, not the corpus. Turn
  records are almost never cut: LoCoMo turns 0 %, LongMemEval turns 16.6 %
  over, and 1.1 % of gold turns are partly cut. Session and document records
  are cut most of the time:

  | records | over the cut | bytes past it | gold past it | questions with all gold past it |
  | --- | --- | --- | --- | --- |
  | LongMemEval-S sessions | 94 % | 79 % | 37 % | 28 % |
  | LoCoMo sessions | 81 % | 27 % | 19 % | 17 % |
  | AP-News articles | 77 % | 48 % | 24 % | 18 % |
  | IR-C corpus (all sources) | 25 % | 56 % | 4 % (QAConv) | 4 % |

- **Probe.** On LongMemEval-S sessions, the cut costs the vector arm
  0.14 R@10 (95 % CI 0.02–0.25) once stratum difficulty is subtracted. It
  costs hybrid retrieval 0.01 (CI −0.05 to +0.07). The preregistered rule
  says: vector failure shown, product failure not shown. Even where the
  vector arm can see the gold, hybrid equals text-only (0.500 vs 0.505), so
  there is little for a fix to win back on this embedder.
- **Cost.** Every chunking option multiplies vector rows, and the vector arm
  costs about 7.8 µs per row per query on this machine. The cheapest chunking
  option takes the vector side of a 19k-session store from about 190 ms to
  about 0.8 s, single caller, before contention.
- **Recommendation.** Ship a guard now: record the embedded span of each
  vector, expose it as a hint, and keep EP-17's `Start` anchor honest. Run one
  gated experiment, overflow chunking (§ 6), whose promotion needs a held-out
  hybrid and answer gain inside the latency budget. Sequence any build after
  the exact-scan kernel work in the 1–2M brief, because chunking's cost is
  mostly the O(N) scan.

## 2. The problem in code

- `fathomdb-embedder/src/candle_bge.rs:66` sets `MAX_SEQUENCE_TOKENS = 512`,
  including `[CLS]` and `[SEP]`. `candle_bge.rs:558-568` applies it as
  tokenizer truncation, so 510 content wordpieces survive.
- `_fathomdb_vector_rows.write_cursor` is `UNIQUE`
  (`fathomdb-schema/src/lib.rs:242-246`): one vector per write.
- The vector arm takes `max(limit, 192)` bit-prefilter candidates
  (`fathomdb-engine/src/search.rs:1537`, `search_types.rs:348`), then rescores
  in f32. Fusion is weighted RRF, k = 30, text 3 : vector 1
  (`fathomdb-engine/src/fusion.rs:10-19`).

## 3. Size of the problem

`size_corpora.py` rebuilds each corpus's record bodies exactly as the named
harness does and tokenizes them with the engine's pinned tokenizer
(`~/.cache/fathomdb/embedders/0b2926f8a9b1/tokenizer.json`, sha256
`d241a60d…`, matching `fathomdb-embedder/src/loader.rs:73`). "Gold past" means
the evidence span starts at or after the end of content wordpiece 510.

| corpus (record unit, harness) | records | wordpieces p50 / p90 / p99 / max | over cut | bytes past cut | gold spans past / straddling | items with all gold past |
| --- | --- | --- | --- | --- | --- | --- |
| Memex extraction gold (doc, `exp_cov_census.py:48`) | 60 | 39 / 51 / 62 / 62 | 0 % | 0 % | 0 % / 0 % | 0 % |
| LongMemEval-S (session, `r2_parity_eval.py:980`) | 19,195 | 2,302 / 3,922 / 4,647 / 16,962 | 94.1 % | 78.5 % | 36.9 % / 2.9 % | 28.0 % |
| LongMemEval-S (turn, `reason_01_preflight.py:190`) | 199,499 | 96 / 593 / 794 / 16,666 | 16.6 % | 10.0 % | 0 % / 1.1 % | 0 % |
| LoCoMo (session, `locomo_loader.py:73`) | 272 | 642 / 976 / 1,276 / 1,379 | 80.9 % | 26.6 % | 19.3 % / 3.2 % | 17.0 % |
| LoCoMo (turn, `fathomdb-mcp/examples/locomo_load.rs:34`) | 5,882 | 43 / 74 / 100 / 130 | 0 % | 0 % | — | — |
| AP-News (article, `apnews_corpus.py:128`) | 1,397 | 785 / 1,546 / 4,059 / 10,698 | 76.9 % | 48.3 % | 23.8 % / 15.1 % | 18.0 % |
| IR-C frozen corpus (doc, `ir_c_fusion_experiment.rs:275`) | 10,506 | 234 / 986 / 11,866 / 36,712 | 24.5 % | 55.5 % | 4.1 % / 0 % | 4.1 % |

Gold positions:

- LongMemEval: the `has_answer` turn's line in its gold session (896 spans,
  479 questions).
- LoCoMo: the evidence `dia_id`'s line (2,820 spans, 1,982 questions).
- AP-News: BenchmarkQED v2 local and linked claim excerpts, found in the
  article by substring (126 found, 5 not).
- IR-C: only QAConv answers are positionable, by first exact occurrence,
  which is the position most favourable to the vector (1,827 found). EnronQA
  answers are paraphrases. QMSum span indices are dropped by the acquire
  script.

IR-C by source: QMSum is the outlier. A third of its 600 records are meeting
transcripts (p90 17,185 wordpieces), and 94.3 % of its bytes lie past the
cut. CNN/DailyMail has 39.4 % of bytes past the cut, Enron 29.3 %, EnronQA
18.4 %, QAConv 9.8 %. The rest are under 2 %.

**Memex.** No populated Memex store exists on this host:
`~/projects/memex/fathom.db` holds no nodes, and the 60-document extraction
gold is short by construction. The write path shows long records are
routine, though:

- web captures are kept up to 50,000 characters
  (`memex/src/memex/tools_web.py:22`, about 12k wordpieces);
- meeting transcripts are written as derived retrieval items
  (`memex/src/memex/fathom_facade.py:363-380`);
- Memex embeds the inner `props["body"]` field
  (`memex/src/memex/fathom_store.py:2903-2911`).

The real Memex distribution is open question 1 in § 9.

**Reading.** Almost no gold lies past the cut when consumers ingest turns,
which the MCP evaluation stores and LoCoMo examples do. A fifth to a third of
gold lies past it when they ingest sessions, articles or captures.

## 4. What is different from the IR-C chunking result

IR-C tried 128/96 windows with max-pool. Dense-only R@10 on exploratory
questions went from 0.073 to 0.163, but hybrid stayed at or below text-only
(`dev/plans/runs/IR-C-ws1-fusion-experiment-full.json`). This study does not
repeat it. Three things differ:

1. **Query shape.** IR-C's exploratory class is QMSum summary questions,
   whose answer spans a whole transcript. Here the target is localized
   evidence: a `has_answer` turn or a claim excerpt that a single window can
   hold. IR-C itself named chunked single vectors as structurally weak for
   discourse questions (`IR-C-retrieval-findings.md`, closeout).
2. **Position.** IR-C's gold had no positions (`span_locator_queries=0`), so
   it could not split "evidence before the cut" from "evidence after".
   Section 5 does. It also shows why IR-C's whole-record dense arm was so
   weak on exploratory questions: it saw roughly the first 3 % of each long
   QMSum transcript.
3. **Anchor.** A chunk-level hit gives EP-17 a real span, whatever it does to
   recall (§ 7.3).

The untested mechanisms the prompt names are costed in § 6: late chunking,
long-context embedders, overflow-only chunking, and record plus chunk
vectors.

## 5. Probe P1: the cut on LongMemEval-S sessions

The preregistration is `dev/experiments/long-record-embedding/PREREG.md`,
committed at `39c4d15e0` before the run.

**Setup.**

- One store with all 19,195 sessions, default embedder on `cuda:0`, vector
  coverage 1.0 (`results/probe_build.json`).
- Global retrieval over the store, scored as session `recall_any@k`.
- Strata: B = every gold span before the cut (192 questions); P = every gold
  span past it (134 questions). The 153 mixed and 21 gold-less questions are
  excluded.
- The vector arm's ranking is read from `explain` `vector_rank`. Every
  measured query had at least 10 vector-ranked hits.

**Results** (`results/probe_summary.json`):

| arm | R@5 B | R@5 P | R@10 B | R@10 P | gap R@10 |
| --- | --- | --- | --- | --- | --- |
| text | 0.385 | 0.343 | 0.505 | 0.463 | 0.043 |
| vector | 0.281 | 0.119 | 0.344 | 0.164 | 0.180 |
| hybrid | 0.406 | 0.321 | 0.500 | 0.448 | 0.052 |

**Effect.** The truncation effect is `E(arm) = gap(arm) − gap(text)`, with
10,000 bootstrap resamples. The text gap is the control: text search sees the
whole body, so its gap is stratum difficulty, not the cut.

- `E(vector)` = +0.137 (CI +0.023 to +0.250): **vector failure shown**
  (rule 1).
- `E(hybrid)` = +0.010 (CI −0.051 to +0.071): **product failure not shown**
  (rule 2).
- Rule 3 applies: the cut is a vector-arm defect that hybrid absorbs today.
  The lean is a guard plus EP-17 anchoring, unless an answer-level loss is
  shown.

**Exploratory cuts** (not preregistered; `results/probe_exploratory.json`):

- `E(hybrid)@5` = +0.043 (CI −0.019 to +0.109). It is suggestive and needs
  the powered run in § 7.
- In stratum B, where the vector sees the gold, hybrid R@10 equals text-only
  (0.500 vs 0.505). The vector finds 13 B questions that text misses, but
  hybrid nets −1 against text. So making P behave like B would win back
  little hybrid recall on this embedder. This bounds the upside of every fix
  in § 6.
- Caveat: the strata differ in question type. B is 42 % temporal-reasoning
  and 30 % multi-session; P is 41 % single-session-user and 17 %
  knowledge-update. The text-gap control removes shared difficulty, but not
  an arm-by-type interaction.

**Latency on the same store**, single caller (`results/latency.json`):

| call | p50 ms | p95 | p99 |
| --- | --- | --- | --- |
| text, limit 10 | 40 | 67 | 71 |
| hybrid, limit 10 | 323 | 392 | 417 |
| hybrid, limit 100 | 546 | 682 | 748 |
| hybrid, limit 10, `explain` | 2,431 | 3,545 | 3,815 |

Attribution by elimination:

- Cutting the bodies to 1,500 characters at the same 19,195 rows takes
  hybrid to 230 ms p50.
- Cutting to 4,800 rows takes it to 95 ms.
- Hybrid minus text is therefore about 43 ms fixed (query embed and setup)
  plus **7.8 µs per vector row**.
- That is about 5× the 1.5 µs per row implied by the 1–2M brief
  (1.5 s at 1M, `dev/design/1-2M-chunk-scaling-DECISION-BRIEF.md`). This
  host, this store and the hydration path all differ, so the figure is local.
- `explain` costs another 2 s and is not usable on a hot path at this size.

Against the queue design: the realistic contention band already puts ok-call
p95 at 0.5–1 s (`dev/experiments/queue-sim/REPORT.md` § 3), and that band was
calibrated on a 3.3 MB LoCoMo store with hybrid p50 7 ms. A 19k-session store
is already 320 ms single-caller.

## 6. Options

The vector multiplier comes from `results/vector_cost.json`. Embed time and
index size are scaled from the probe build: about 9–11 ms per vector on the
3090 at the projection's batch sizes (19,195 vectors in 216 s after writes; 4,800
short records in 42.8 s including writes), and 1.6 KB per vector in vec0 (30.9 MB for 19,195). Query cost uses
the § 5 fit, and its numbers are p50 for the vector side only, single caller,
at the 19,195-record LongMemEval store. Add text and contention on top.

| option | vectors per record: LME / AP / IR-C | LME embed on the 3090 | LME vec0 size | vector-side query p50 | schema and projection | EP-17 anchor |
| --- | --- | --- | --- | --- | --- | --- |
| 0. today | 1 / 1 / 1 | 3.6 min | 31 MB | 0.19 s | — | `Start` |
| A. chunk-only, 256/192 windows, max-pool | 12.4 / 5.0 / 3.5 | ≈ 45 min | 380 MB | ≈ 1.9 s | new chunk-span table; the record vector goes away, which changes short-record vectors | chunk span |
| B. record vector plus 256/192 chunks for records over the cut | 13.2 / 5.5 / 3.5 | ≈ 48 min | 410 MB | ≈ 2.0 s | additive chunk table; record vector unchanged | chunk span, else `Start` |
| C. overflow only: record vector (first 510) plus 510-windows over the rest | 5.1 / 2.3 / 1.8 | ≈ 18 min | 160 MB | ≈ 0.81 s | additive chunk table; short records byte-identical | chunk span, else `Start` |
| D. late chunking (long-context model, 256 windows inside 8,192 macro chunks) | 9.6 / 4.1 / 3.0 | not measured: 4.7× today's wordpieces through quadratic attention | 300 MB at d = 384; 2× at d = 768 | ≈ 1.5 s | new embedder identity, re-embed everything, plus the chunk table | chunk span |
| E. long-context embedder, one vector per record | 1 / 1 / 1 | not measured: same wordpieces as D | 31 MB at d = 384; 2× at d = 768 | 0.19 s, or more at d = 768 | new embedder identity, full re-embed; cut moves to 8,190 (IR-C still has 19 % of wordpieces past it) | `Start` |
| F. ingest-side chunking by the consumer | consumer's choice | consumer's | consumer's | grows with the consumer's row count | none in the engine | the chunk record is the hit; snippets are honest |

Option notes:

- **A, chunk-only.** This is IR-C's mechanism at a different geometry, and
  IR-C measured it flat to negative on hybrid. Dai and Callan found that
  best-passage (MaxP) aggregation beats first-passage on Robust04, but
  first-passage was best on ClueWeb09-B description queries ("Deeper Text
  Understanding for IR with Contextual Neural Language Modeling", SIGIR 2019,
  <https://arxiv.org/abs/1905.09217>). Replacing the record vector is not a
  safe default.
- **B, record plus chunks.** This is the parent–child or small-to-big pattern
  (LangChain `ParentDocumentRetriever`,
  <https://reference.langchain.com/python/langchain-classic/retrievers/parent_document_retriever/ParentDocumentRetriever>;
  LlamaIndex `AutoMergingRetriever`,
  <https://developers.llamaindex.ai/python/framework/integrations/retrievers/auto_merging_retriever/>).
  It is the most expensive option, and its chunks duplicate the head the
  record vector already covers.
- **C, overflow only.** It adds vectors only where the cut hides text, so
  short-record corpora (LoCoMo turns, Memex extraction docs) are untouched,
  and the eu7 fidelity gate is not re-opened for them. The head window is the
  record vector already. It is the cheapest option that sees past-cut text.
- **D, late chunking.** "Late Chunking: Contextual Chunk Embeddings Using
  Long-Context Embedding Models" (Günther et al., Jina AI, 2024,
  <https://arxiv.org/abs/2409.04701>) pools chunk vectors after one
  long-context pass. It reports a +1.8 nDCG@10 average over naive chunking on
  BEIR, and naive chunking winning on needle and passkey tasks where context
  is noise. LongMemEval `has_answer` evidence is closer to the needle shape.
  It needs a long-context model with token outputs. The Apache-2.0
  candidates of bge-small's size are `jina-embeddings-v2-small-en` (33M
  parameters, 512-d, 8,192 tokens, ALiBi) and the larger
  `nomic-embed-text-v1.5` (768-d, 8,192 tokens). `jina-embeddings-v3` is
  CC BY-NC 4.0 and ruled out.
- **E, long-context embedder.** IR-C already tried nomic-embed-text-v1.5:
  worse on exploratory questions, better only on exact-fact questions,
  which text already solves (`IR-C-retrieval-findings.md`). LongEmbed reports
  BM25 at 81.3 average against 58.2 for Jina-V2 and 57.5 for Nomic-V1 on its
  long-retrieval tasks ("LongEmbed: Extending Embedding Models for Long
  Context Retrieval", Zhu et al., 2024, <https://arxiv.org/abs/2404.12096>).
  One vector per long record also gives no anchor.
- **F, consumer-side chunking.** This is how the MCP evaluation stores
  already work (LoCoMo turns, 0 % cut). It costs the engine nothing and gives
  honest snippets. It moves the unit choice, and the record-grouping problem
  (§ 7.1), to every consumer.
- **Not costed: contextual retrieval.** Anthropic's contextual embeddings
  plus contextual BM25 cut top-20 retrieval failure by 49 %, at about $1.02
  per million document tokens to generate the prefixes
  (<https://www.anthropic.com/news/contextual-retrieval>). It needs an LLM
  call per chunk at ingest, which FathomDB's local, no-egress default does
  not make. It is an option for a consumer that already runs an LLM at
  ingest, on top of C or F.
- **Not costed: late interaction.** ColBERT stores one 128-d vector per token
  (<https://arxiv.org/abs/2004.12832>). That is a different index class and
  out of scope.

Every option except E and F multiplies vector rows, and query cost grows with
rows (§ 5). So the latency cost of chunking is mostly the scan, which is what
option A of the 1–2M brief (a packed exact-scan kernel) addresses.

## 7. Recommendation

### 7.1 Now: a guard, no new vectors

1. **Record the embedded span.** At projection time, store how far each
   record's vector reaches: `embedded_bytes`, and the total body length, in a
   new sidecar table keyed by `write_cursor`. It is an additive
   `CREATE TABLE`; existing rows are back-filled by the projector, not by
   `INSERT … SELECT`. The value is the byte offset of the cut, which the
   tokenizer already knows.
2. **Hint.** A hint is derived only from values the call already holds (H3,
   `dev/design/agent-access/hints-sidecar.md` § 2). So the search hit path
   carries a `partly_embedded` flag for records whose body runs past
   `embedded_bytes`.
   - A hit found only by the vector arm, or a no-results hybrid call over a
     store with partly embedded records, gets
     `{"action": "search", "reason": "partly_embedded", "mode": "text"}`.
   - It is closed-vocabulary and data-free, like the B2 rows.
   - It needs an owner decision on the hint vocabulary (§ 9, question 3).
3. **EP-17.** No change is needed. `anchor = Start` is honest for
   vector-only hits, because the start is inside the scored region
   (`match-centred-snippets.md` § 3.4). With `embedded_bytes`, the SDK can
   also say whether a `Start` snippet covers the whole scored region.
4. **Consumer guidance.** Document the cut, and the table in § 3, in the
   public embedder docs: ingest turns, sections or captures in pieces of at
   most about 1.5 KB when vector recall past the start matters.

### 7.2 One experiment: overflow chunking (option C)

Of the options, C is the one worth running. It is the cheapest that sees
past-cut text, it leaves short-record corpora untouched, it gives EP-17 a real
anchor, and it needs no new model. The § 5 bound says a hybrid gain on
LongMemEval will be small. The experiment is there to measure that bound
across corpora, not because a win is expected.

**Arms.** All arms use bge-small, CLS pooling as shipped, and harness-side
pooling. Overflow chunks are written as their own records with
`source_id` = parent, and their vectors go in through the engine's own
projector.

- C0: today.
- C1: overflow windows of 510 wordpieces, no overlap, max-pool to the parent
  before fusion.
- C2: as C1, with 64-wordpiece overlap.
- D1 (optional, only if the owner wants the late-chunking question closed):
  late chunking with `jina-embeddings-v2-small-en` at 256 windows.

**Grouping before the cut.** Chunks must be grouped to their parent before
the 192-candidate cut and before fusion. At 5 chunks per record, 192
ungrouped candidates cover about 38 distinct records, and duplicates would
crowd the fused list. The harness collapses each parent to its best chunk
before RRF.

**Corpora and splits.** Each corpus is split 50/50 by question with seed
20261018. The first half is used to choose between C1 and C2; the second
half is held out.

- LongMemEval-S sessions.
- AP-News articles with v2 local and linked questions.
- IR-C QAConv, CNN/DailyMail and QMSum-specific questions. QMSum needs its
  upstream `relevant_text_span`, re-fetched at the pinned SHA.
- Short-record guards: LoCoMo turns and IR-C exact-fact questions.

**Metrics.**

- Session or document `recall_any@{5,10}` for hybrid, vector and text.
- Answer accuracy on LongMemEval with the existing judge harness.
- Duplicate-parent rate.
- Embed wall time on `cuda:0`.
- Vec0 bytes.
- Single-caller hybrid p50/p95/p99 at each store size.

**Decision rule** (preregister before the held-out run). Promote C1 or C2
only if all of these hold on the held-out half:

1. Hybrid R@10 improves by ≥ 0.03 on at least two of the three long-record
   corpora, with each CI excluding 0.
2. LongMemEval answer accuracy improves by ≥ 2 points, CI excluding 0.
3. No short-record guard corpus loses more than 0.01 hybrid R@10.
4. Single-caller hybrid p95 at the LongMemEval store size stays within
   1.5× today's (392 ms → at most 590 ms).

If 1–3 hold and only 4 fails, park the option behind the exact-scan kernel
(1–2M brief, E1 then option A) and re-measure there. Otherwise close it, and
keep the guard and the consumer-side units.

**Cost of the experiment.** About 18 minutes of embedding on the 3090 for
LongMemEval, under 5 for the rest, and one answer-judge run on the held-out
half. No product code: chunk records go through the public write path, and
pooling is in the harness.

### 7.3 What the promoted design would change (for later, not now)

- **Schema.** A new table
  `_fathomdb_vector_chunks(vec_rowid PRIMARY KEY, write_cursor, chunk_ix,
  byte_start, byte_end)`. The overflow vectors live in `vector_default`;
  `_fathomdb_vector_rows` keeps the head vector, one per write, so
  `write_cursor UNIQUE` stays. Additive only. Existing long records get
  overflow vectors through a projection rebuild, which is derived data, not a
  data migration.
- **Search.** The vector arm groups candidates by `write_cursor` and keeps
  the best chunk, before the candidate cut, and returns the winning chunk's
  span to the hit.
- **EP-17.** A vector hit whose best chunk is an overflow window gets
  `anchor = Match` with `spans = [(byte_start, byte_end)]`. That needs a third
  anchor source in § 7.2 of the snippet design, or a new
  `SnippetAnchor::Chunk`, which is the owner's choice. Head-window and
  short-record vector hits keep `Start`.
- **Identity.** Chunk geometry is a vector-configuration field
  (`ADR-0.6.0-vector-identity-embedder-owned.md`: "chunking policy (chunk
  size, overlap, splitter type)"). It is not embedder identity, so no
  identity change follows. Late chunking (D) would change how text becomes a
  vector, which is identity.
- **Hints.** `partly_embedded` disappears for chunked records.

## 8. Receipts

`dev/experiments/long-record-embedding/`:

- `README.md` — how to rerun.
- `PREREG.md` — the probe's preregistration.
- `cut.py` and `test_cut.py` — the cut arithmetic.
- `size_corpora.py` — `results/sizing.json`.
- `vector_cost.py` — `results/vector_cost.json`.
- `probe.py` and `test_probe.py` — the probe store builds and the
  `results/probe_*.json` files.
- `probe_exploratory.py` — `results/probe_exploratory.json`.
- `latency.py` — the `results/latency_*.json` files, combined in
  `results/latency.json`.

## 9. Open questions for the owner

1. **Memex's real distribution.** How long are Memex's actual records (web
   captures, transcripts, notes)? No populated store is on this host. A
   read-only length census on a real store would settle whether § 3's
   session-level rows or its turn-level rows describe Memex.
2. **Guard scope.** Ship the `embedded_bytes` sidecar and the hint in the
   agent-access Block 2 work, or later?
3. **Hint vocabulary.** Add `partly_embedded` as a reason on the `search`
   action?
4. **Experiment.** Run § 7.2 now, or only after the exact-scan kernel (1–2M
   brief E1)? Include the late-chunking arm D1?
5. **Latency outside this design.** Hybrid on a 19k-session store is 323 ms
   p50 single-caller, and `explain` adds 2 s. Both are outside this design,
   but they bear on the queue design's service-time assumptions, which were
   calibrated on a 3.3 MB store. Should they be raised as a separate finding?
