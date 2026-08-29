# GLOBAL-01 performance-gap analysis and improvement hypothesis

**Status:** proposed and falsifiable; not implemented or accepted as a product
profile.

## Purpose

Capture the post-run analysis of why the first GLOBAL-01 FathomDB treatment
lost comprehensiveness, diversity, and empowerment, and define one bounded
hypothesis for improving those measures without weakening the accepted LOCOMO
memory default, temporal validity, lifecycle attribution, directness, or cost.

## What GLOBAL-01 measured

The first run compared native Microsoft GraphRAG global search with
`source_mapreduce_c_v1` on the same 15 AP News documents, eight global
questions, shared `deepseek-v4-pro` generator, and independent Claude Haiku
judge.

The result was split:

| Metric | FathomDB win rate | Clustered 95% interval |
| --- | ---: | ---: |
| Comprehensiveness | 0.3750 | 0.0000–0.7500 |
| Diversity | 0.3750 | 0.0000–0.7500 |
| Empowerment | 0.3875 | 0.0375–0.7500 |
| Directness | 0.9688 | 0.9062–1.0000 |

This was a synthesis comparison, not a FathomDB retrieval benchmark. The
FathomDB arm did not call `Engine.search`. It read all 15 witness documents in
three fixed ordinal batches of five, allowed 300 output tokens per map batch,
discarded `NONE` maps, and reduced the remaining points with a 1,500-token
limit. Therefore the measured deficit is evidence organization and global
synthesis under that contract. It does not demonstrate a failure of FTS,
vector retrieval, reranking, or fused search.

References:

- [first-run result](2026-08-29-global-01-first-run-result.md)
- [first-run contract](2026-08-25-global-01-first-run-contract.md)
- `experiments/global_01_live.py`, lines 519–556
- [registered receipt](../../experiments/runs/global-01-native-comparison-20260829T1613Z-40685e82/record.json)
- private checkpoint under
  `data/performance-benchmarking/global-01/runs/global-01-first-run-deepseek-20260829-b/checkpoint.json`

## Measured failure modes

### Evidence collapse during mapping

The clearest example was
`global-01-3a66a0cd444b`, the mental-health-intervention question. Two fixed
document batches returned no useful evidence; the remaining batch contributed
one narrow health-care-sharing-ministry point. The FathomDB reducer consequently
produced a 271-character answer. GraphRAG produced a 5,264-character thematic
answer covering regulatory, political, institutional, workforce, funding, and
access barriers.

This is a failure of fixed ordinal partitioning plus independent map rejection.
Evidence that is weak within one arbitrary five-document batch cannot be joined
with related evidence from another batch before the `NONE` decision.

### Coverage loss during reduction

Other losing questions had substantial mapped evidence, but the reducer still
produced a narrower set of themes. The prompt asked for a comprehensive answer
but supplied no coverage ledger, no novelty objective, no per-theme quota, and
no explicit accounting for claims omitted during reduction.

### Output-budget and judge confounding

FathomDB median answer length was 680 characters; GraphRAG median length was
5,505.5 characters. GraphRAG emitted three 75-character no-answer responses,
which are the questions on which FathomDB performed best, while its five
substantive responses were much longer. Coverage-oriented pairwise judging may
reward length even when marginal claims add little value.

The next comparison must use matched answer budgets and assertion recall. It
must not declare success merely by generating longer text.

### Canonical attribution was lost at the final boundary

Map outputs contained ordinal source labels, but final FathomDB answers did not
retain canonical source citations. GraphRAG cited derived community reports,
not canonical records. The run did not score factual correctness,
groundedness, unsupported-claim rate, or canonical attribution.

Any new treatment must preserve canonical source identity and content hash from
candidate selection through the final claim. Derived cluster or summary IDs are
not sufficient.

### Sample uncertainty

Eight questions produced wide question-clustered intervals. The result names a
directional failure mode and supports a new preregistered treatment. It does not
support a general quality-ranking claim.

## Constraints from earlier FathomDB evidence

### Keep A0 as the general memory default

ANSWER-01 found that `hybrid_ce_alpha_10_pool_20` improved groundedness and
attribution by 6.25 percentage points each, but reduced temporal correctness by
18.18 points and did not improve overall answer accuracy. MEMORY-01 found A0
ahead of Mem0 overall by 7.99 points and ahead on temporal questions by 35.83
points, while Mem0 led multi-hop by 8.16 points.

The global treatment therefore must be explicitly selected. It must not replace
A0 for ordinary agent-memory queries.

References:

- [ANSWER-01 track](tracks/answer-01-shortlist-scoring.md)
- [ANSWER-01 receipt](../../experiments/runs/answer-01-shortlist-live-20260822T1234Z-8a050808/record.json)
- [MEMORY-01 result](2026-08-24-memory-01-result.md)

### Bounded query expansion and deeper candidates are supported ingredients

The preserved multi-hop retrieval observations provide a useful mechanism
check, without proving the global hypothesis:

- On 282 LOCOMO multi-hop questions, complete registered-evidence coverage rose
  from 36 for FTS-10 to 63 for FTS-20 and 94 for FTS-50. Mean evidence recall
  rose from 0.3118 to 0.4434 and 0.5688 respectively.
- The protected multi-query shadow reached 65 complete-evidence questions and
  0.4612 mean recall, but mean latency was 22.40 ms versus 1.33 ms for the FTS
  depth observations.
- On 24 held-out LongMemEval multi-hop cases, A0 covered every gold session for
  19 cases. Protected multi-query and deep-compact selection each covered all
  gold sessions for 22 cases; mean evidence recall rose from 0.9396 to 0.9688.

These results support bounded decomposition, deeper candidate collection, and
compact context selection as ingredients. They do not justify enabling
multi-query globally or persisting a semantic graph.

References:

- `data/performance-benchmarking/locomo-multihop/runs/locomo-multihop-retrieval-20260825-02-turn/retrieval-observations.v1.json`
- `data/performance-benchmarking/locomo-multihop/runs/longmemeval-multihop-heldout-20260825-02/heldout-observations.v1.json`
- `data/performance-benchmarking/locomo-multihop/runs/protected-multiquery-equivalence-20260825-02/equivalence-observations.v1.json`

### Apply temporal validity before organization

TEMPORAL-01 verified `ReadView(valid_as_of=...)` exactly for all eight synthetic
TRACE probes. Any grouping, clustering, summarization, or claim extraction must
operate on the already filtered read view. Filtering after summaries or
clusters are built could mix current and superseded facts.

Reference: [TEMPORAL-01 TRACE validity result](2026-08-23-temporal-01-trace-validity-result.md).

### Derived units must obey TRACE-01

Every cluster, summary, or query-time claim must retain canonical source
identity and content hash. Supersession and erasure canaries must prove that no
stale or orphan derived unit remains searchable.

Reference: [TRACE-01 track](tracks/trace-01-projection-lifecycle-integrity.md).

### Do not base this treatment on unconsolidated extracted memory

EXTRACT-01 improved raw answer accuracy by only one of 78 questions and failed
value-changing consolidation: the competing Boston-to-Austin fact remained
active and `supersession_applied` was false. The next global treatment must not
promote free-standing extracted facts to canonical current memory.

Query-time claims may be ephemeral and source-linked. Persisted extracted facts
are out of scope until native consolidation passes its own lifecycle gate.

Reference: [EXTRACT-01 result](2026-08-23-extract-01-implementation.md).

## Falsifiable hypothesis

An explicit caller-selected profile, `global_lazy_coverage_v1`, will improve
comprehensiveness, diversity, and empowerment over `source_mapreduce_c_v1`
without materially reducing directness or increasing unsupported claims if it:

1. applies the requested `ReadView` before any derived organization;
2. decomposes the global question into three to five bounded subqueries;
3. retrieves deeper candidates for each subquery using the accepted FathomDB
   lexical/hybrid mechanisms;
4. groups candidates into source-linked lexical or concept-co-occurrence
   clusters;
5. selects across clusters with a relevance-plus-novelty objective;
6. extracts query-time claims from related source groups while retaining
   canonical source IDs and hashes; and
7. reduces through an explicit coverage ledger that preserves citations,
   exceptions, disagreements, and uncertainty under a fixed answer budget.

This is a lifecycle-safe adaptation of the useful parts of LazyGraphRAG. It is
not a proposal to adopt persistent GraphRAG indexing or to replace A0.

## Proposed profile contract

### Routing

- Caller selection is explicit: `profile=global_lazy_coverage_v1`.
- No automatic promotion from ordinary A0 based only on query length or model
  judgment.
- A later routing experiment may test an explicit global-intent classifier, but
  it is not part of the first treatment.

### Retrieval and organization

- Input: canonical records visible in the requested `ReadView`.
- Decomposition: three to five subqueries, deduplicated and bounded.
- Candidate depth: preregister one depth, initially informed by the FTS-50
  observations; do not tune per question.
- Grouping: source-linked lexical/concept co-occurrence only for the first
  treatment. Do not require a persistent entity graph.
- Selection: relevance plus novelty, with a per-cluster cap and a fixed total
  context budget.
- Claims: ephemeral, query-time only, each with canonical source ID and content
  hash.

### Coverage-ledger reduction

For every selected claim, record one of:

- included in the final answer;
- redundant with an included claim;
- irrelevant after cross-source review;
- conflicting or uncertain and disclosed; or
- omitted due to the fixed answer budget.

The reducer must not silently discard an entire relevant cluster. Final claims
must carry canonical citations. The response-length limit is matched to the
control arm.

## Evaluation design

### Dataset

Use the 49 qualified AP News data-global questions from
`generated_questions_v2/data_global_questions_assertions.json`. The file has 50
questions and 856 assertions; applying the stored
`grounding >= 4`, `relevance >= 4`, and `verifiability >= 4` threshold yields
637 qualified assertions across 49 questions.

Use a fixed held-out split established before treatment execution. Do not tune
on the eight first-run questions.

### Arms

- Control: current `source_mapreduce_c_v1`.
- Treatment: `global_lazy_coverage_v1`.
- Optional reference, not a tuning oracle: native GraphRAG global search.

Use the same generator, judge, source corpus, answer budget, order swapping,
repetitions, and cost accounting across treatment and control.

### Measures

- pairwise comprehensiveness, diversity, empowerment, and directness;
- qualified assertion recall;
- unsupported-claim rate;
- canonical source-link completeness;
- supersession and erasure canaries;
- input/output tokens and USD cost;
- cold and steady p50/p95 latency.

Run an A/A judge validation before the comparison. Human-review generated
assertions or a preregistered sample before treating them as gold.

## Acceptance and rejection boundary

Accept the hypothesis only if all of the following hold on the held-out set:

- pairwise win rate is at least 0.55 for each of comprehensiveness, diversity,
  and empowerment;
- the question-clustered lower confidence bound is above 0.50 for each of those
  three measures;
- qualified assertion recall improves by at least 10 percentage points;
- directness win rate is at least 0.45;
- unsupported-claim rate does not regress;
- canonical source-link completeness is 100%;
- supersession and erasure canaries produce zero stale or unattributed output;
- query token cost and p95 latency are no more than 2× the control.

Reject the hypothesis if any quality or lifecycle threshold fails. Do not
compensate for a failure by increasing the treatment answer budget.

## Main risks

- **Verbosity bias:** pairwise judges may reward longer answers. Mitigate with
  matched budgets and assertion scoring.
- **Query drift:** decomposition can introduce off-topic subqueries. Require
  bounded count, source support, and drift reporting.
- **Novelty over-pruning:** diversity selection can suppress corroborating
  evidence. Preserve corroboration counts in the coverage ledger.
- **Namesake/co-occurrence errors:** lexical clusters can mingle unrelated
  entities. Keep clusters source-linked and claims ephemeral.
- **Stale summaries:** building before temporal filtering can blend superseded
  facts. Apply `ReadView` first and invalidate cached derivatives by source
  version.
- **Product regression:** a benchmark gain may cost too much latency or
  complexity. Keep the route explicit and enforce the 2× limits.

## Research basis

- Microsoft GraphRAG attributes global-question gains to corpus organization
  through community summaries followed by map-reduce:
  [From Local to Global](https://www.microsoft.com/en-us/research/publication/from-local-to-global-a-graph-rag-approach-to-query-focused-summarization/).
- Microsoft LazyGraphRAG motivates query decomposition, iterative breadth and
  best-first selection, grouped claim extraction, and ranked reduction without
  requiring eagerly generated persistent summaries:
  [LazyGraphRAG](https://www.microsoft.com/en-us/research/blog/lazygraphrag-setting-a-new-standard-for-quality-and-cost/).
- RAPTOR supports hierarchical clustering and recursive summaries for holistic
  and multi-step questions:
  [ICLR 2024 paper](https://proceedings.iclr.cc/paper_files/paper/2024/file/8a2acd174940dbca361a6398a4f9df91-Paper-Conference.pdf).
- BenchmarkQED documents assertion scoring, counterbalanced pairwise scoring,
  A/A validation, and human review of generated assertions:
  [BenchmarkQED](https://microsoft.github.io/benchmark-qed/).
- Maximal Marginal Relevance supplies the relevance-versus-novelty selection
  primitive:
  [Carbonell and Goldstein](https://kilthub.cmu.edu/articles/journal_contribution/The_Use_of_MMR_and_Diversity-Based_Reranking_for_Reodering_Documents_and_Producing_Summaries/6610811/1).

## Next action

Do not implement from this note alone. First write a dated measurement contract
that freezes the qualified-question manifest, held-out split, exact treatment
parameters, matched answer budget, scorer, A/A result, repetitions, uncertainty
calculation, cost cap, and the acceptance boundary above. Then perform a
zero-spend input and environment preflight before requesting execution approval.
