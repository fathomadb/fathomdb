---
title: Long-record embedding — size of the problem and choice of fix
date: 2026-10-18
status: ACTIVE
desc: Measures how much of each consumer corpus lies past bge-small's 512-wordpiece cut. A preregistered, paired position-intervention probe shows that the cut costs the vector arm 0.09 R@10; for hybrid the pooled effect is 0.000, but that is two opposite stratum effects cancelling, and moving past-cut gold into the head gains +0.03 on 4 of 133 questions. Costs six fixes in vectors, embed time, index size, query latency, schema and snippet anchor. Recommends a guard now and one specified overflow-chunking experiment, parked until a stated trigger fires.
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

**Owner decision, 2026-10-19: accepted.** Ship the guard of § 7.1 (vector
coverage per record reported in `orient`, and the scored extent in the
EP-17 snippet). Park overflow chunking (option C, § 7.2) until one of its
triggers fires. The guard is not built yet; where it lands is open (§ 9,
question 2).

**Owner decisions, 2026-10-31.** B2-8 (the guard) is approved: its storage,
a per-vector span table shaped for chunking, ships in 0.8.28's format 35,
and its reporting in 0.8.29 Block 2 (§ 9, question 2 answered). Chunking
stays parked, but the store is built ready for it. **Before any chunking
decision, read `dev/design/overflow-chunking-reevaluation.md`**, which
holds the tripwires, how they are monitored, and the costs that must be
weighed. Some numbers below are stale for format 35 (that file, § 4.4).

Paths are relative to `src/rust/crates/` unless given in full. Every number
below comes from `dev/experiments/long-record-embedding/results/` unless a
source is cited.

- **Setup.** Measurements ran on 2026-10-18 against the `exp/agent-access`
  tree (`4ff9239ef`), built as a CUDA wheel with `test-hooks`, with the RTX
  3090 on `cuda:0`.
- **History.** A first probe (P1) and a first intervention (P2) were
  superseded; § 5.3 says why. The conclusions rest on P2b.

## 1. Summary

- **Size.** The cut depends on the record unit, not the corpus.
  - Turn records are almost never cut: LoCoMo turns 0 %, LongMemEval turns
    16.6 % over the cut, and 1.1 % of gold turns are partly cut.
  - Session and document records are cut most of the time:

    | records | over the cut | bytes past it | gold past it | questions with all gold past it |
    | --- | --- | --- | --- | --- |
    | LongMemEval-S sessions | 94 % | 79 % | 37 % | 28 % |
    | LoCoMo sessions | 81 % | 27 % | 19 % | 17 % |
    | AP-News articles | 77 % | 48 % | 24 % | 18 % |
    | IR-C corpus (all sources) | 25 % | 56 % | 4 % (QAConv) | 4 % |

- **Effect.** Probe P2b moves the gold turns of the same LongMemEval-S
  questions across the cut and pairs each question with itself. Making the
  gold visible to the record's vector raises vector-only R@10 by 0.086
  (95 % CI 0.044–0.127). The pooled hybrid effect is 0.000 (CI −0.016 to
  +0.016).
  - Under the preregistered pooled rule, neither failure is shown.
  - The pooled hybrid zero is a cancellation of opposite stratum effects,
    not evidence of no effect. In the direction a fix acts (past-cut gold
    moved into the head, n = 133), hybrid gains +0.030 (CI +0.008 to
    +0.060). That rests on 4 questions, all gains. In the reverse direction
    (n = 182), hybrid changes by −0.022 (CI −0.044 to −0.005), on 4
    questions, all losses; this is unexplained.
  - Both stratum results are descriptive, not preregistered.
  - No answer-level measurement exists yet.
  - Text-only recall was identical across the paired stores, as the
    design requires.
- **Cost.** Every chunking option multiplies vector rows. The vector arm
  costs about 8 µs per row at p50 and 10.5 µs at p95, per query, on this
  machine. The cheapest chunking option would take a 19k-session store's
  vector side from about 0.2 s to about 0.8 s p50 (1.05 s p95), single
  caller.
- **Recommendation.**
  - Ship a guard now: record how much of each record the vector covers,
    report it in `orient`, and report the scored extent in the EP-17
    snippet.
  - Specify one fix, overflow chunking (option C), with a preregistrable,
    powered experiment (§ 7.2).
  - Do not run it until a trigger fires (§ 7.2): an answer-level loss on
    long records, a change to the fusion weights or the embedder, or the
    exact-scan kernel landing.
- **A separate engine finding.** Fusion sums the RRF terms of distinct nodes
  whose bodies are byte-identical, and returns them as one hit carrying one
  `source_id` (`fathomdb-engine/src/fusion.rs:99-111`). LongMemEval-S has
  640 such groups. See § 9, question 5.

## 2. The problem in code

- **Truncation.** `fathomdb-embedder/src/candle_bge.rs:66` sets
  `MAX_SEQUENCE_TOKENS = 512`, including `[CLS]` and `[SEP]`.
  `candle_bge.rs:558-568` applies it as tokenizer truncation, so 510 content
  wordpieces survive. The projector embeds the raw body.
- **One vector per write.** `_fathomdb_vector_rows.write_cursor` is `UNIQUE`
  (`fathomdb-schema/src/lib.rs:242-246`).
- **Candidates.** The vector arm takes `max(limit, 192)` bit-prefilter
  candidates (`fathomdb-engine/src/search.rs:1537`, `search_types.rs:348`),
  then rescores them in f32.
- **Fusion.** Weighted RRF with k = 30, text 3 : vector 1
  (`fathomdb-engine/src/fusion.rs:10-19`), keyed on the body string
  (`fusion.rs:99-111`).

## 3. Size of the problem

`size_corpora.py` rebuilds each corpus's record bodies exactly as the named
harness does. It tokenizes them with the engine's pinned tokenizer
(`~/.cache/fathomdb/embedders/0b2926f8a9b1/tokenizer.json`, sha256
`d241a60d…`, matching `fathomdb-embedder/src/loader.rs:73`). "Gold past"
means the evidence span starts at or after the end of content wordpiece 510.

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

- **LongMemEval:** the `has_answer` turn's line in its gold session (896
  spans, 479 questions).
- **LoCoMo:** the evidence `dia_id`'s line (2,820 spans, 1,982 questions).
- **AP-News:** BenchmarkQED v2 local and linked claim excerpts, found in the
  article by substring (126 found, 5 not).
- **IR-C:** only QAConv answers are positionable, by first exact
  occurrence, the position most favourable to the vector (1,827 found).
  EnronQA answers are paraphrases, and the QMSum acquire script drops the
  span indices.

**IR-C by source** (rows in `results/sizing.json`):

- QMSum is the outlier. A third of its 600 records are meeting transcripts
  (p90 17,185 wordpieces), and 94.3 % of its bytes lie past the cut.
- Bytes past the cut elsewhere: CNN/DailyMail 39.4 %, Enron 29.3 %,
  EnronQA 18.4 %, QAConv 9.8 %. The other five sources are under 2 %.

**Memex.** No populated Memex store exists on this host:
`~/projects/memex/fathom.db` holds no nodes, and the 60-document extraction
gold is short by construction. The write path shows long records are
routine:

- web captures are kept up to 50,000 characters, about 12k wordpieces
  (`memex/src/memex/tools_web.py:22`);
- meeting transcripts are written as derived retrieval items
  (`memex/src/memex/fathom_facade.py:363-380`);
- Memex embeds the inner `props["body"]` field
  (`memex/src/memex/fathom_store.py:2903-2911`).

The real distribution is open question 1.

**Reading.** When consumers ingest turns, as the MCP evaluation stores and
LoCoMo examples do, almost no gold lies past the cut. When they ingest
sessions, articles or captures, a fifth to a third of it does.

## 4. What is different from the IR-C chunking result

IR-C tried 128/96 windows with max-pool. Dense-only R@10 on exploratory
questions went from 0.073 to 0.163, but hybrid stayed at or below text-only
(`dev/plans/runs/IR-C-ws1-fusion-experiment-full.json`). This study does not
repeat that. Three things differ:

1. **Query shape.** IR-C's exploratory class is QMSum summary questions,
   whose answer spans a whole transcript. Here the target is localized
   evidence, a `has_answer` turn or a claim excerpt, which one window can
   hold.
2. **Position.** IR-C's gold had no positions (`span_locator_queries=0`), so
   it could not separate evidence before the cut from evidence after it.
   § 5 does, by moving the evidence.
   - It also explains why IR-C's whole-record dense arm was so weak on
     exploratory questions: it saw roughly the first 3 % of each long QMSum
     transcript.
3. **Anchor.** A chunk-level hit gives EP-17 a real span, whatever it does to
   recall (§ 7.3).

## 5. Probes

### 5.1 P2b: position intervention (the result)

Preregistered in `PREREG-P2.md` (`413ba06ad`), as amended by `PREREG-P2b.md`
(`e894432c4`), before the run.

**Design.**

- **Stores.** Two stores hold all 19,195 LongMemEval-S sessions, with the
  default embedder on `cuda:0`. Vector coverage is 1.0 on both.
- **Intervention.**
  - Past-cut questions (P, 133): every gold session's `has_answer` turns
    move to the front.
  - Before-cut questions (B, 182): those turns move to the end.
  - Twin sessions with identical bodies move together.
  - Line multisets are unchanged, so `bm25()` is unchanged.
  - 11 questions are excluded because their gold could not be moved across
    the cut.
- **Arms.** `search_text_only(limit=10)`; `search(limit=10)` with shipped
  defaults; and the vector arm's ranking from `explain` on a `limit=100`
  call.
- **Metric.** Session `recall_any@10`, counting a twin of a gold session
  as gold. Per question, `d = recall(gold visible) − recall(gold hidden)`,
  averaged over all 315 questions. 95 % CIs from 10,000 bootstrap
  resamples.

**Result** (`results/probe2b_summary.json`):

| arm | R@10, gold visible | R@10, gold hidden | D | 95 % CI | SE | MDE (80 %) |
| --- | --- | --- | --- | --- | --- | --- |
| text | 0.616 | 0.616 | 0.000 | 0.000 to 0.000 | 0.000 | — |
| vector | 0.432 | 0.346 | **+0.086** | +0.044 to +0.127 | 0.021 | 0.059 |
| hybrid | 0.625 | 0.625 | **0.000** | −0.016 to +0.016 | 0.009 | 0.025 |

- **Validity.** Text agreement between stores is 100 %, which passes.
- **Rule 1: vector failure not shown.** `D(vector) = 0.086` is below the
  0.10 bar, although its CI excludes 0. The cut has a real but moderate
  cost to the vector arm on its own.
- **Rule 2: product failure not shown.** The pooled MDE (0.025) is below
  the 0.03 threshold, so rule 3 does not apply. But the pooled estimator
  averages the two directions, and they cancel (below). So the pooled
  result does not show the absence of an effect.
- **Rule 4 reading.** At shipped fusion weights and session-level records,
  putting the gold inside the record's one vector moves hybrid top-10
  recall on few questions: 8 of 315 changed. The direction that matters
  gains about 0.03 on past-cut questions.
  - This bounds that intervention only. It is not a bound on a chunk
    vector (a different unit), on retuned fusion weights, or on answer
    accuracy.

**By stratum** (descriptive, not preregistered;
`results/probe2b_strata.json`):

| direction | n | D(vector) | 95 % CI | D(hybrid) | 95 % CI | hybrid +1 / −1 questions |
| --- | --- | --- | --- | --- | --- | --- |
| P: past-cut gold moved into the head (what a fix does) | 133 | +0.211 | +0.143 to +0.286 | +0.030 | +0.008 to +0.060 | 4 / 0 |
| B: gold moved out of the head | 182 | −0.005 | −0.055 to +0.044 | −0.022 | −0.044 to −0.005 | 0 / 4 |

- **P.** The vector effect is large, and the hybrid effect sits at rule 2's
  threshold with its CI above 0, on 4 questions. A fix that puts past-cut
  evidence into a vector can therefore be expected to gain on the order of
  0.03 hybrid R@10 on past-cut questions. Those are 28–37 % of
  LongMemEval-S questions, so the whole-corpus gain would be about 0.01.
- **B.** Moving the gold out of the head barely moves the vector arm,
  perhaps because B questions (77 % temporal-reasoning or multi-session) are
  matched through text other than the gold turn. That does not explain why
  hybrid *improves* on 4 questions when the gold is hidden. The rotation
  also changes which text fills the head, which is a candidate cause, but it
  is untested.

**By type** (descriptive): `D(hybrid)` stays within ±0.033 for every
question type.

### 5.2 Latency on the same stores

Single caller, 60 questions after 20 warm-up calls (`results/latency.json`).

The full-body 19,195-session store:

| call | p50 ms | p95 | p99 |
| --- | --- | --- | --- |
| text, limit 10 | 40 | 67 | 71 |
| hybrid, limit 10 | 323 | 392 | 417 |
| hybrid, limit 100 | 546 | 682 | 748 |
| hybrid, limit 10, `explain` | 2,431 | 3,545 | 3,815 |

**Attribution.**

- Four stores of 1,500-character bodies (4,800; 9,600; 19,195; and 38,390
  records) fit hybrid minus text as:
  - p50: 25 ms + 7.96 µs per vector row;
  - p95: 19 ms + 10.5 µs per vector row.
- Residuals are up to 31 ms at p50 and 42 ms at p95. The 4,800 and 9,600
  points are inverted, so below about 10k rows fixed costs and noise
  dominate. The line is set by the two larger stores.
- Full bodies add about 90 ms at 19,195 rows: hybrid minus text is 283 ms
  against 194 ms for short bodies. So hydration of long bodies is a second
  term, and the fit does not model it.
- p99 over 60 calls is one sample, and is not fitted.
- The 38,390-record store was measured while the root disk was nearly full.
  Its text p99 (213 ms) is suspect.

**Comparisons.**

- The 1–2M brief implies about 1.5 µs per row
  (`dev/design/1-2M-chunk-scaling-DECISION-BRIEF.md`). This host, store and
  hydration path are about 5× slower, so the per-row figure here is local.
- The queue design's realistic contention band already puts ok-call p95
  at 0.5–1 s (`dev/experiments/queue-sim/REPORT.md` § 7). That was
  calibrated on a 3.3 MB LoCoMo store with hybrid p50 7 ms. A 19k-session
  store is already 323 ms single-caller.
- `explain` adds about 2 s at this size.

### 5.3 Superseded probes

- **P1** (`PREREG.md`) compared different questions: B with gold before the
  cut, P with gold past it. It reported `E(vector) = +0.137`.
  - The review found this to be a composition effect. Within question type,
    min-n weighted, the effect is +0.017 for the vector and −0.041 for
    hybrid (`results/probe_exploratory.json`). The strata differ in type
    mix and in gold-session count (2.02 against 1.37).
  - P1's hybrid arm also came from a `limit=100` call, not the shipped
    default.
  - P1's raw numbers stay in `results/probe_*` as the record. No
    conclusion rests on them.
- **P2** (`PREREG-P2.md`) failed its own validity check: text agreement was
  85.4 %, against a 98 % bar.
  - Cause: LongMemEval-S holds 640 groups of byte-identical sessions
    (1,371 sessions). Fusion sums equal bodies (§ 2), so twins score about
    double. P2 rotated a gold session but not its twin, which broke the
    merge.
  - `diag_bm25.py` shows raw `bm25()` was identical on both stores.
  - P2b fixes this (`PREREG-P2b.md`).

## 6. Options

**How the table's numbers are derived.**

- **Vectors per record** come from `results/vector_cost.json`.
- **Embed time** scales with vector count. Two full builds on the 3090 took
  11 ms and 16 ms per vector (`results/probe_build.json`,
  `results/probe2_build.json`, the second including writes).
- **Index size** is computed: 384 f32 values (1,536 B) plus a 48 B bit code
  plus metadata, about 1.6 KB per vector.
- **Query cost** uses the § 5.2 fit for the vector side only, single caller,
  at the 19,195-record LongMemEval store. Add text, body hydration and
  contention on top.

| option | vectors per record: LME / AP / IR-C | LME embed on the 3090 | LME vec0 size | vector-side query p50 / p95 | schema and projection | EP-17 anchor |
| --- | --- | --- | --- | --- | --- | --- |
| 0. today | 1 / 1 / 1 | 3.5–5 min | 31 MB | 0.18 / 0.22 s | — | `Start` |
| A. chunk-only, 256/192 windows, max-pool | 12.4 / 5.0 / 3.5 | 43–63 min | 380 MB | 1.9 / 2.5 s | new chunk-span table; replaces the record vector, which changes short-record vectors | chunk span |
| B. record vector plus 256/192 chunks for records over the cut | 13.2 / 5.5 / 3.5 | 47–68 min | 410 MB | 2.0 / 2.7 s | additive chunk table; record vector unchanged | chunk span, else `Start` |
| C. overflow only: record vector (first 510) plus 510-windows over the rest | 5.1 / 2.3 / 1.8 | 18–26 min | 160 MB | 0.80 / 1.05 s | additive chunk table; short records byte-identical | chunk span, else `Start` |
| D. late chunking (long-context model, 256 windows inside 8,192 macro chunks) | 9.6 / 4.1 / 3.0 | not measured: 4.7× today's wordpieces through quadratic attention | 300 MB at d = 384; 2× at d = 768 | 1.5 / 2.0 s | new embedder identity, full re-embed, plus the chunk table | chunk span |
| E. long-context embedder, one vector per record | 1 / 1 / 1 | not measured: same wordpieces as D | 31 MB at d = 384; 2× at d = 768 | 0.18 / 0.22 s, or more at d = 768 | new embedder identity, full re-embed; cut moves to 8,190 (IR-C still has 19 % of wordpieces past it) | `Start` |
| F. ingest-side chunking by the consumer | consumer's choice | consumer's | consumer's | grows with the consumer's row count (LongMemEval turns, 199,499 rows: about 1.6 / 2.1 s, extrapolated 5× past the largest measured store) | none in the engine | the chunk record is the hit; snippets are honest |

Option notes:

- **A, chunk-only.** This is IR-C's mechanism at a different geometry, and
  IR-C measured it flat to negative on hybrid. Dai and Callan found
  best-passage (MaxP) aggregation beats first-passage on Robust04, but
  first-passage was best on ClueWeb09-B description queries ("Deeper Text
  Understanding for IR with Contextual Neural Language Modeling", SIGIR 2019,
  <https://arxiv.org/abs/1905.09217>). Replacing the record vector is not a
  safe default.
- **B, record plus chunks.** This is the parent–child or small-to-big
  pattern: LangChain `ParentDocumentRetriever`
  (<https://reference.langchain.com/python/langchain-classic/retrievers/parent_document_retriever/ParentDocumentRetriever>)
  and LlamaIndex `AutoMergingRetriever`
  (<https://developers.llamaindex.ai/python/framework/integrations/retrievers/auto_merging_retriever/>).
  It is the most expensive option, and its chunks duplicate the head the
  record vector already covers.
- **C, overflow only.**
  - It adds vectors only where the cut hides text, so short-record corpora
    (LoCoMo turns, Memex extraction documents) are untouched, and the eu7
    fidelity gate is not reopened for them.
  - The head window is the record vector already.
  - It is the cheapest option that sees past-cut text.
- **D, late chunking.** "Late Chunking: Contextual Chunk Embeddings Using
  Long-Context Embedding Models" (Günther et al., Jina AI, 2024,
  <https://arxiv.org/abs/2409.04701>) pools chunk vectors after one
  long-context pass.
  - It reports a +1.8 nDCG@10 average over naive chunking on BEIR.
    Naive chunking wins on needle and passkey tasks, where the context is
    noise; LongMemEval `has_answer` evidence is closer to that shape.
  - It needs a long-context model with token outputs. The Apache-2.0
    candidates are `jina-embeddings-v2-small-en` (33M parameters, 512-d,
    8,192 tokens, ALiBi) and `nomic-embed-text-v1.5` (768-d, 8,192 tokens).
    `jina-embeddings-v3` is CC BY-NC 4.0 and ruled out.
- **E, long-context embedder.** This is untested here.
  - IR-C ran nomic-embed-text-v1.5 only under its chunked, 512-token
    geometry. It parked whole-record long-context embedding as "PARKED, not
    refuted" (`IR-C-retrieval-findings.md:35-38`).
  - LongEmbed reports BM25 averaging 81.3 against 58.2 for Jina-V2 and 57.5
    for Nomic-V1 on its long-document retrieval tasks ("LongEmbed: Extending
    Embedding Models for Long Context Retrieval", Zhu et al., 2024,
    <https://arxiv.org/abs/2404.12096>).
  - One vector per long record gives no anchor.
- **F, consumer-side chunking.**
  - This is how the MCP evaluation stores already work: LoCoMo turns, 0 %
    cut. It costs the engine nothing and gives honest snippets.
  - It moves the unit choice to every consumer, and it multiplies rows by
    more than C does. LongMemEval turns are 10.4× the session count.
- **Not costed: contextual retrieval.** Anthropic's contextual embeddings
  plus contextual BM25 cut top-20 retrieval failure by 49 %
  (<https://www.anthropic.com/news/contextual-retrieval>). Generating the
  prefixes costs about $1.02 per million document tokens, through an LLM
  call per chunk at ingest. FathomDB's local, no-egress default does not
  make such calls. A consumer that already runs an LLM at ingest can add
  it on top of C or F.
- **Not costed: late interaction.** ColBERT stores one 128-d vector per token
  (<https://arxiv.org/abs/2004.12832>). That is a different index class.

Every option except E multiplies the rows that the O(N) vector scan reads.
So most of chunking's latency cost is the scan, which option A of the 1–2M
brief (a packed exact-scan kernel) targets.

## 7. Recommendation

### 7.1 Now: a guard, no new vectors

1. **Record the embedded extent.**
   - At projection time, store `embedded_bytes` (the byte offset of the cut)
     and the body length, in a new sidecar table keyed by `write_cursor`.
   - Adding it is an additive `CREATE TABLE`.
   - New rows get the values when they are projected. Existing rows need a
     projection-generation bump, so that the projector recomputes them: a
     derived-data rebuild, not an `INSERT … SELECT` data migration. The
     tokenizer already knows the offset.
2. **Report it per store in `orient`.** Report the share of records over the
   cut and the share of body bytes the vectors do not see, cached with the
   projection state so that `orient` adds no scan.
   - This is the fact an agent can act on: in a store where much text is
     unembedded, prefer `mode: text` for passages deep inside long records.
   - A per-hit hint is not proposed. A vector-only hit on a partly embedded
     record was found by its head, so "search again as text" is not
     actionable on it, and the real failure is records not returned. A
     hint from store-level facts on every search call would also need a
     value the call does not hold (hints-sidecar.md H3).
3. **Report it per hit in the EP-17 sidecar.** `SnippetV1` gains
   `scored_bytes: Option<u32>`, the extent the hit's vector scored. It lives
   in the opt-in snippet sidecar, so there is no per-hit struct growth
   (R-OBS-2). A `Start` snippet can then say whether it covers the whole
   scored region. This changes the `SnippetV1` struct that EP-17 proposes
   (`match-centred-snippets.md` § 7.2), so EP-17 and the snippet design must
   be amended in the same PR that builds it.
4. **No EP-17 anchor change.** `anchor = Start` stays honest for vector-only
   hits (`match-centred-snippets.md` § 3.4).
5. **Consumer guidance.** Document the cut and the § 3 table in the public
   embedder docs. Ingest turns, sections or capture pieces of at most about
   1.5 KB when vector recall past a record's start matters. Note that this
   multiplies rows (§ 6, F).

### 7.2 The one fix to specify: overflow chunking (C), parked

When a trigger below fires, start from
`dev/design/overflow-chunking-reevaluation.md`.

C is the option to keep ready:

- it is the cheapest option that sees past-cut text;
- it leaves short-record corpora untouched;
- it gives EP-17 a real anchor;
- it needs no new model.

**Do not run it now.** The two halves of the evidence point the same way:

- **The expected gain is small.** In P2b, making past-cut gold visible to
  the vector gains about 0.03 hybrid R@10 on past-cut questions (4 of 133
  questions), about 0.01 corpus-wide.
- **The latency gate cannot pass today.** Under the current scan, C's
  vector-side p95 on a LongMemEval-shaped store projects to 1.05 s, against
  0.22 s today.

That pairing is a weak case for the latency cost now. It is also the reason
to keep C specified rather than close it.

**Run it when any trigger fires:**

1. an answer-level evaluation on session- or document-unit stores shows a
   loss that traces to evidence past the cut. No such measurement exists
   yet, and this study did not make one (§ 9, question 3);
2. the fusion weights or the default embedder change;
3. the exact-scan kernel (1–2M brief, E1 then option A) lands, so that the
   latency gate can pass.

**Experiment, ready to preregister:**

- **Arms.** Both use bge-small with CLS pooling as shipped.
  - C0: today.
  - C1: overflow windows of 510 wordpieces, no overlap, fixed a priori.
    There is no tuning split, so the whole sample is held out.
  - An optional D1 (late chunking, `jina-embeddings-v2-small-en`, 256
    windows) only if the owner wants that question closed.
- **Harness.** A Rust test under `fathomdb-engine/tests/`, as IR-C did,
  with `test-hooks`. It is test code, not product code.
  - **Text ranking.** The C0 store's `search_text_only`. The text arm must
    not change, so chunk records never enter the FTS index.
  - **Vector ranking.** The chunk store's vector stage, with
    `set_vector_stage_only_for_test` (`fathomdb-engine/src/search_api.rs:1465`)
    and a raised candidate pool via `set_search_limit_for_test`
    (`search_api.rs:1430`). The harness groups chunks to their parent
    before fusion, keeping each parent's best chunk; 192 ungrouped
    candidates would cover only about 38 distinct records.
  - **Fusion.** The harness reimplements RRF (k = 30, 3 : 1). It must first
    reproduce the engine's hybrid top-10 on C0 for at least 99 % of
    questions, or the run stops.
- **Sample and power.**
  - The power basis is P2b's past-cut stratum, the direction the fix acts
    in: paired SE for hybrid 0.0149 at n = 133.
  - A 0.03 effect at 80 % power, one-sided α = 0.05, needs SE ≤ 0.012,
    which is about 205 paired past-cut questions (133 × (0.0149 / 0.012)²).
  - A chunk vector may change more questions than P2b's intervention did.
    Recheck the discordance on the first 50 questions.
  - Pool all positionable past-cut questions: LongMemEval P and mixed
    (287), AP-News local and linked with a past-cut excerpt, QAConv
    past-cut (about 75), and QMSum specific questions once their upstream
    `relevant_text_span` is re-fetched at the pinned SHA.
  - Guard set: LoCoMo turns and IR-C exact-fact questions.
- **Decision rule. Promote C1 only if all four hold:**
  1. Paired hybrid R@10 gain on the pooled past-cut set ≥ 0.03, one-sided
     95 % lower bound > 0.
  2. Paired LongMemEval answer accuracy gain ≥ 2 points on the same
     questions, lower bound > 0, with the existing judge harness.
  3. No guard corpus loses more than 0.01 hybrid R@10.
  4. Single-caller hybrid p95, measured on the C1 store at LongMemEval
     size, is at most 1.5× C0's.
  - If 1–3 hold and only 4 fails, park C1 behind the kernel and
    re-measure there. Otherwise close C1 and keep the guard.
- **Cost.**
  - Embedding: about 18–26 minutes on the 3090 for LongMemEval, under
    5 minutes for the rest.
  - One answer-judge run.

### 7.3 What a promoted C would change (later, not now)

- **Schema.** A new table
  `_fathomdb_vector_chunks(vec_rowid PRIMARY KEY, write_cursor, chunk_ix,
  byte_start, byte_end)`. Overflow vectors live in `vector_default`, and
  `_fathomdb_vector_rows` keeps the head vector, so `write_cursor UNIQUE`
  stays. Existing long records get overflow vectors through a projection
  rebuild. All of this is additive.
- **Search.** The vector arm groups candidates by `write_cursor` and keeps
  the best chunk, before the candidate cut. The hit carries the winning
  chunk's span.
- **EP-17.** An overflow-window vector hit gets `anchor = Match` with
  `spans = [(byte_start, byte_end)]`. That needs a third anchor source in
  the snippet design's § 7.2, or a new `SnippetAnchor::Chunk`, which is the
  owner's choice. Head-window and short-record vector hits keep `Start`.
- **Identity.** Chunk geometry is vector configuration, not embedder
  identity. `ADR-0.6.0-vector-identity-embedder-owned.md` lists "chunking
  policy (chunk size, overlap, splitter type)" as allowed configuration, so
  no identity change follows. Late chunking (D) would change how text
  becomes a vector, which is identity.
- **Guard.** `scored_bytes` and the `orient` coverage fields then report
  chunk coverage.

## 8. Receipts

`dev/experiments/long-record-embedding/`. `README.md` lists every file,
explains how to rerun, and records each deviation.

- **Preregistrations:** `PREREG.md` (P1), `PREREG-P2.md`, `PREREG-P2b.md`.
- **Code:** `cut.py`, `size_corpora.py`, `vector_cost.py`, `probe.py`,
  `probe2.py`, `probe_exploratory.py`, `latency.py`, `latency_fit.py`, and
  the `diag_*.py` diagnostics.
- **Tests:** `test_cut.py`, `test_probe.py`, `test_probe2.py`.
- **Results:** `results/` — `sizing.json`, `vector_cost.json`,
  `probe2b_summary.json`, `probe2b_per_question.jsonl`, the latency files,
  the superseded P1/P2 results, and the diagnostics.

## 9. Open questions for the owner

1. **Memex's real distribution.** How long are Memex's actual records: web
   captures, transcripts, notes? A read-only length census on a real store
   would settle whether § 3's session rows or turn rows describe Memex.
2. **Guard scope.** Should the `embedded_bytes` sidecar, the `orient`
   fields and `SnippetV1.scored_bytes` go into the agent-access Block 2
   work, or later?
3. **Triggers.** Are § 7.2's three triggers the right ones?
   - Recommended: schedule the answer-level evaluation on session-unit
     stores (trigger 1) now. Retrieval shows a small effect in the
     direction a fix acts (+0.03 on past-cut questions), and no
     answer-level measurement exists.
   - It needs no chunk vectors: compare answers for P2b's past-cut
     questions on the original and the rotated store, with the existing
     judge harness.
4. **Latency.** Hybrid on a 19k-session store is 323 ms p50 single-caller,
   and `explain` adds 2 s. The queue design's service times came from a
   3.3 MB store. Should this be raised against the queue design, and should
   it bring the 1–2M brief's E1 forward?
5. **Duplicate bodies in fusion.** Distinct nodes with byte-identical bodies
   have their RRF terms summed and come back as one hit carrying the first
   node's `source_id` (`fusion.rs:99-111`).
   - In a memory store this double-counts repeated content and hides the
     other copies' provenance.
   - Is that intended, or a defect to file? It sits outside this design.
