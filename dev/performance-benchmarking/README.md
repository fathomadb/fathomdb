# FathomDB performance and competitive benchmarking

This directory is the durable, commit-friendly index for FathomDB performance
and competitive-benchmark evidence. It records the question, comparator,
dataset identity, measurement protocol, result status, and pointers to the
authoritative artifacts. It is not a data lake or a second copy of an
experiment.

## Storage rule

- Store short, reviewable result notes and this index here.
- Point to raw runs, corpus payloads, generated gold, model caches, and large
  reports where they live. Those paths are often gitignored, machine-local, or
  intentionally not committed.
- Never copy experiment payloads into `dev/` merely to make a result note
  self-contained. Record a content digest, run identifier, and stable source
  path instead.
- A missing pointed-to artifact makes the result non-reproducible; retain the
  finding only as historical evidence and say that the artifact is unavailable.
- Performance observations are descriptive unless a predeclared repeated-run
  protocol says otherwise. Do not convert one run into a latency SLO, QPS
  claim, or tail-latency estimate.

`data/corpus-data/` and `experiments/runs/` are the normal local targets. Both
can contain uncommitted material. The canonical historical synthesis remains
[`../experiments-ledger.md`](../experiments-ledger.md), rather than this
directory duplicating it.

## How to add a result

Create one Markdown file per completed campaign, named
`YYYY-MM-DD-<topic>-result.md`, containing only:

1. The decision question and claim class: diagnostic, regression, parity, or
   surpass.
2. The systems and exact versions/configurations compared.
3. Corpus and gold identifiers: source path, digest, query count, and license.
4. Protocol: metric, answerer/judge controls, repetitions, and eligibility
   rules for summary statistics.
5. Result and uncertainty, including invalid or unavailable observations.
6. Pointers to the raw run directory and durable record/manifest files.
7. What the result does not establish.

Use relative links for committed documents. For gitignored or machine-local
artifacts, use a plain, clearly labeled path and digest; do not create broken
Markdown links just to make the path clickable.

## Historical competitive benchmark register

| Comparator | Benchmark question and protocol | Best current local gold | Status / result interpretation |
| --- | --- | --- | --- |
| Mem0-OSS | Is FathomDB near-parity or better on agentic memory under an identical-answerer protocol? The original gate compared per-class recall and answer accuracy. | `data/corpus-data/eval/0.8.3-locomo-memory-gold.json`: 1,443 answered queries (factoid 841, temporal 321, multi-session 281). | The 0.8.3 resolution closed at provisional near-parity. It is not a general or current “better than Mem0” claim. LOCOMO has no knowledge-update class, and its temporal/multi-session evidence is session-level. |
| Graphiti/Zep | Second agentic-memory comparator under the same shared protocol as Mem0. | LOCOMO, with the same limits. | A reporting comparator, not the primary historical decision gate. |
| Naive RAG / BM25 | Retrieval floor on the same documents and answerer conditions. | LOCOMO for agentic memory; IR gold and BEIR for retrieval-only comparisons. | Required baseline; a win over it does not establish parity with a memory or graph system. |
| Microsoft GraphRAG | Is FathomDB near-parity or better on global sensemaking, under repeated, order-swapped pairwise AutoE judgments? | `data/corpus-data/raw/apnews_benchmarkqed/`: 1,397 AP-News articles and 350 AutoQ assets. | The historical result is split: exhaustive map-reduce surpassed GraphRAG, while the cheaper depth-1/product path lost comprehensiveness and diversity. AutoQ/AutoE is a judged protocol, not static answer-key gold. |
| HippoRAG-2 | Multi-hop cross-check using answer F1 and all-bridges retrieval. | `data/corpus-data/raw/musique_dev.jsonl`: 2,417 answerable 2–4-hop questions with supporting-paragraph labels. | Intended as a secondary cross-check, not part of the GraphRAG gate. No completed HippoRAG-2 comparison is recorded. |

The benchmark definitions and historical outcomes are authoritative in
[`../design/0.8.3-mem0-parity.md`](../design/0.8.3-mem0-parity.md),
[`../design/0.8.4-graphrag-sensemaking.md`](../design/0.8.4-graphrag-sensemaking.md),
and [`../experiments-ledger.md`](../experiments-ledger.md).

The next controlled-comparison entry conditions, matching-data rule, and
one-repetition characterization protocol are in
[the 2026-08-11 comparison plans](2026-08-11-competitor-comparison-plans.md).
The first prerequisite-execution outcomes are recorded in
[the 2026-08-11 readiness results](2026-08-11-competitor-baseline-readiness-execution-results.md).

The Mem0-OSS entry condition above is being revised: a parallel-session
FathomDB-vs-Mem0 LOCOMO run (FTS-only default config only) motivated a
FathomDB-self-characterization phase ahead of any further competitive run.
See
[the 2026-08-14 LOCOMO FathomDB capability campaign plan](2026-08-14-locomo-fathomdb-capability-campaign-plan.md) —
it sequences a knob/config sweep against FathomDB's own baseline (feeding
[`../design/retrieval-measures-initiative.md`](../design/retrieval-measures-initiative.md))
before any Mem0 comparison is re-run.

## Additional on-disk gold worth reusing

| Data | Suitable use | Restriction |
| --- | --- | --- |
| `data/corpus-data/eval/ir_gold/all.gold.json` | 4,597-query FathomDB retrieval benchmark: exact-fact, exploratory, and negative queries; compare retrieval systems only. | It has no free-text answers, so it cannot support answer-accuracy claims. |
| `data/corpus-data/raw/beir/{arguana,fiqa,nfcorpus,touche2020}/` | Standard retrieval comparisons with qrels. | Not an agent-memory or graph-system head-to-head by itself. |
| `data/corpus-data/raw/summhay/summhay.jsonl` | Global query-focused summarization with insight and citation gold. | Strong GraphRAG-adjacent follow-up, but not the historical AP-News gate. |
| `data/corpus-data/raw/timelineqa/`, `timeqa/`, and `tot/` | Temporal and personal-memory regression/stress testing. | No historical shared-comparator protocol is registered. |
| `data/corpus-data/external/memex-elps/` | ELPS extraction conformance. | Not an end-to-end retrieval or competitor benchmark. |

LongMemEval was the original primary agent-memory source, but it is streamed or
cached outside this `data/` inventory; do not claim it is locally reproducible
until its actual cache or acquired payload is recorded in the campaign note.

## Performance evidence register

Performance campaigns use the same pointer discipline. A result note must link
the quality artifact to the observed-cost or repeated-performance artifact and
state whether it is a one-run diagnostic or an eligible repeated-run summary.

- Use the raw run's `record.json`, resolved configuration, workload manifest,
  and artifact digests as provenance; never rely on a run name alone.
- Preserve typed unavailable observations and invalid execution cells. They are
  evidence of what was not measured, not zeros or clean passes.
- Keep separate arms separate. Do not pool timings from different candidates,
  configurations, or treatments.
- A fresh-store warm-query treatment is not process-cold unless the campaign
  also supplies the required subprocess and OS-cache evidence.

The current EARP campaign notes and raw artifacts are normally found under
`experiments/runs/`; the implementation planning entry point is
[`../plans/IR-C-test-query-quality-instrumentation-plan.md`](../plans/IR-C-test-query-quality-instrumentation-plan.md).

| Date | Campaign | Status | Durable note |
| --- | --- | --- | --- |
| 2026-08-08 | EARP S6 retrieval repricing before and after Slice 19 | Complete historical diagnostic. The 12-query, one-execution-per-state sample isolates the unindexed write-cursor join; it is not a latency gate or repeated-run summary. | [Result](2026-08-08-earp-s6-retrieval-repricing-result.md) |
| 2026-08-08 | EARP FTS-only characterization over IR-C reuse-tier v2 gold | Complete descriptive characterization: 4,472 scoreable queries plus 125 negatives. It is not a comparator, answer-accuracy, or repeated-run claim. | [Result](2026-08-08-earp-characterization-v2-gold-result.md) |
| 2026-08-08 | EARP paired limit-10-versus-50 calibration | Complete diagnostic loop: the pre-fix instrument found limit-dependent top-K contents, and the post-fix rerun found zero differing pairs. | [Result](2026-08-08-earp-limit-calibration-result.md) |
| 2026-08-11 | EARP retrieval-score versus answerer-judgment abstention | Historical descriptive evidence only. Raw artifacts are unavailable, the judgment run was incomplete, and `retrieved_scores` remains a separate unlanded decision. | [Result](2026-08-11-earp-abstention-result.md) |
| 2026-08-11 | IR-C FTS initial population: 10,506 documents, 4,597 queries, `limit=10` | Quality is complete. The repaired, quality-linked one-repetition artifact has two complete cells and is a descriptive baseline only; it is not a comparator or latency claim. The earlier artifact remains typed-invalid historical evidence. | [Plan](2026-08-11-initial-population-plan.md) · [Result](2026-08-11-initial-population-results.md) |
| 2026-08-14 | LOCOMO FathomDB self-characterization: ingest/query knob grid vs FathomDB's own FTS-only baseline; first M6 (query latency) and M7 (ingest-to-queryable latency) instrumentation for FathomDB. | Planned. Phase A (worktree reconciliation, stable provenance fix, M6/M7 harness, resumable scorer wrapper) not yet started. | [Plan](2026-08-14-locomo-fathomdb-capability-campaign-plan.md) |

## Data and license posture

LOCOMO, AP-News BenchmarkQED, and TimelineQA are EVAL-ONLY and must remain
outside version control. Do not copy their contents, derived verbatim gold, or
generated answers into this directory. Before publishing a result, name the
license and retain only the minimum non-payload metadata needed to reproduce
the run. See [`../corpus-survey/corpus-map.md`](../corpus-survey/corpus-map.md)
for acquisition, license, and suitability details.
