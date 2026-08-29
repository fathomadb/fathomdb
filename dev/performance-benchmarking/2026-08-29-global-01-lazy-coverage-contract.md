# GLOBAL-01 lazy-coverage measurement contract

**Status:** registered; paid execution authorized by coreyt on 2026-08-29 with
a $12 hard cap.

## Decision

Test whether explicit `global_lazy_coverage_v1` improves global
comprehensiveness, diversity, and empowerment over a scaled source-linked
map/reduce control without weakening directness, groundedness, canonical
attribution, lifecycle correctness, latency, or cost beyond the registered
limits. This is an opt-in global-synthesis decision, not a replacement for A0.

## Inputs and split

- Corpus: all 1,397 AP News BenchmarkQED documents in the registered archive,
  at canonical document granularity. The archive SHA-256 is
  `2f70dda22a9f261f285c94f3ac13a8f0df60b69fe3df1b5853b47b372065a66f`.
- Questions: the 49 data-global questions with at least one assertion whose
  stored validation has `is_valid=true` and grounding, relevance, and
  verifiability each at least 4. This yields 637 qualified assertions.
- Assertion file SHA-256:
  `b40c3ff9e18944e8763c888d5a986abe665263a8a52159f110ae889795ab0730`.
- Qualified-manifest SHA-256:
  `bbc03b55617e34b2489aa14efb16495fc80f7f204fe86233c4a77898d122a5ed`.
- Split: rank eligible question IDs by
  `sha256("global-01-split-20260829-v1:" + question_id)`. The first ten whose
  question-text hash is absent from the first-run manifest are development;
  the other 39 are held out. Both qualified first-run overlaps remain held out.
- Development selection SHA-256:
  `f0cc9eaaf7f27520fffc4be9a3dbdfb9d31087b31e78ff72782c48d06379dd27`.
- Held-out selection SHA-256:
  `a00798af1fac0b2982597a1610c9c38d69be01e0f2a6d7fbb187d798ed573b5e`.
- Witness: the first three development questions under that ranking; selection
  SHA-256
  `5dc634d24a8907530e624eba44e2de9721548690a7936b6746e1774a85ef9d91`.

Question text, assertions, answers, source payloads, and databases remain in
the access-controlled external artifact root. Safe receipts retain only counts,
hashes, aggregate metrics, and model usage.

## Arms

Both arms use a fresh FathomDB 0.8.23 database, the strict current `ReadView`,
FTS-only retrieval, full canonical documents, and matched 1,500-token answer
budgets. Every presented source carries canonical `source_id` and content hash.

### Control: `source_mapreduce_c_v1_fts50`

The 15-document first-run control cannot exhaustively map all 1,397 documents
at acceptable cost. The registered full-corpus control therefore retrieves the
top 50 FTS candidates for the original question, maps fixed ordinal batches of
five with a 300-token limit and at most two 30-word claims per batch, and
reduces them under the matched answer budget.
It preserves the first treatment's fixed batching and independent mapping while
making the full-corpus comparison executable. It is a named scaled control, not
an assertion that the first-run implementation was unchanged.

### Treatment: `global_lazy_coverage_v1`

The caller must select this profile explicitly. It performs:

1. four bounded, deduplicated subqueries generated from the global question;
2. FTS depth 50 for the original question and each subquery;
3. reciprocal-rank relevance with `k=60` over the deduplicated candidate union;
4. source-linked subquery-intent grouping by each candidate's best-ranked
   subquery;
5. greedy relevance-plus-novelty selection with weights 0.65 and 0.35,
   respectively, at most six documents per group, at most 24 documents total,
   and a 24,000-token context ceiling;
6. one source-linked claim map per non-empty group, limited to 600 tokens and
   at most four 30-word claims; and
7. coverage-ledger reduction under the matched 1,500-token answer budget.

Claims are query-time-only. Each claim and final answer claim must retain at
least one canonical source ID and content hash. The coverage ledger assigns
every mapped claim exactly one disposition: included, redundant, irrelevant,
conflicting-or-uncertain, or omitted-for-budget. No extracted fact, cluster,
summary, or graph unit is persisted.

## Models and scorer

- Generator: `deepseek-v4-pro`, thinking disabled, temperature 0.0.
- Pairwise judge: `claude-haiku`, temperature 0.7, five repetitions with both
  answer orders. It reports comprehensiveness, diversity, empowerment, and
  directness separately.
- Assertion and groundedness scorer: `claude-haiku`, temperature 0.0, two
  trials. It receives only the answer, qualified assertions, and the canonical
  source excerpts cited by that answer. Each excerpt is selected
  deterministically for claim-term overlap and capped at 1,600 characters. It
  reports passed assertion indices and support for each answer claim.
- Assertion scoring follows BenchmarkQED's assertion-unit design. The stored
  validation threshold is stricter than BenchmarkQED's documented default of
  3. Generated assertions are not regenerated or changed during this run.

Before the witness, run an A/A check over the 16 preserved answers from the
first comparison, bound by answer-manifest SHA-256
`e89ae0edb69230a31557fb212d7748a283f9ecc160d640747cbb207a62335ea1`.
Each metric must return at least 95% ties and no answer-side preference above
5%. Failure invalidates the judge and stops execution.

## Sequence

1. Validate the typed configuration and regenerate the private qualified and
   split manifests. All registered hashes and counts must match.
2. Create a new FathomDB with the experiment bootstrap and run `fathomdb
   doctor` setup and integrity evidence. Ingest all 1,397 canonical documents.
3. Complete zero-spend retrieval, `ReadView`, source/hash attribution,
   supersession, erasure, checkpoint/resume, retry, completeness, and cost-cap
   canaries.
4. Return the safe preflight receipt and projected spend for HITL execution
   authorization.
5. If authorized, run the A/A check and then the three-question development
   witness. The witness validates execution and contracts; it is not a tuning
   set result.
6. If the witness is valid, execute both arms on the 39 held-out questions in
   deterministic alternating arm order.
7. Write external payload artifacts incrementally, then write one safe typed
   receipt and append one experiment-index row.

Every priced cell is persisted before the next call. Resume executes only
missing cells. HTTP 429 honors the full `Retry-After`; 5xx and timeouts use
bounded exponential backoff. Calls are serialized. An incomplete run cannot
emit a comparison verdict.

## Measures and uncertainty

- Pairwise win rate and question-clustered two-sided 95% bootstrap interval for
  comprehensiveness, diversity, empowerment, and directness; 2,000 bootstrap
  draws with seed `20260829`.
- Qualified assertion recall by arm and paired question-level delta.
- Unsupported answer-claim rate by arm and paired question-level delta.
- Deterministic canonical source-link completeness.
- Supersession and erasure canary failures.
- Input/output tokens, USD cost, calls, retries, and completeness.
- Cold and five-repeat steady retrieval p50/p95, plus end-to-end paid p50/p95.

## Acceptance boundary

Accept only if every condition holds on the held-out set:

- FathomDB treatment win rate is at least 0.55 and its question-clustered lower
  95% bound is above 0.50 for comprehensiveness, diversity, and empowerment;
- qualified assertion recall improves by at least 10 percentage points;
- directness win rate is at least 0.45;
- unsupported-claim rate does not increase;
- canonical source-link completeness is 100%;
- supersession and erasure canaries have zero stale or unattributed output; and
- treatment query-token cost and end-to-end p95 are each at most 2x control.

Reject if any condition fails. Do not increase the treatment answer budget,
change the held-out split, or tune a parameter after viewing held-out results.

## Cost and authorization

The preliminary projection is $9.50, based on first-run measured token usage
and 1,376 planned paid cells: 32 A/A cells plus 32 cells for each of three
witness and 39 held-out questions. A cell permits at most three semantic-format
submissions, so the structural ceiling is 4,128 submissions; transport retry
attempts remain separately bounded. The $12.00 hard cap is authoritative and
can stop the run before either count is reached. Neither value authorizes
execution.

HITL authorized execution with a $12 hard cap on 2026-08-29. Stop during
execution on configuration or input
drift, invalid A/A behavior, a failed lifecycle canary, non-resumable state,
incomplete output, exhausted retry budget, or the cost cap.

The A/A gate passed. The initial witness stopped at the first control map after
three identical responses exhausted the exact 300-token output ceiling. Before
any held-out execution, semantic revision `v2-bounded-map` made the registered
claim-count and claim-length bounds explicit. It does not change inputs, split,
models, output-token budgets, scorer, acceptance boundaries, or the cost cap.
The original invalid cells remain in the checkpoint and their cost remains
charged to the same campaign.

Revision `v2-bounded-map` completed five control batches, then another batch
reached the 300-token ceiling despite satisfying the registered semantic
bounds. Revision `v3-compact-source-refs` replaces repeated canonical UUID and
SHA-256 output fields with short prompt-local references. The runner restores
the exact canonical source ID and hash before persisting each claim. This is an
encoding correction only; it changes neither evidence selection nor any
registered measurement boundary.

Revision `v3-compact-source-refs` completed the next batch and removed output
truncation. A later batch returned compact but surplus semantic output.
Revision `v4-cap-surplus-claims` makes the already-registered maximum
deterministic in the caller by retaining only the first two control claims or
first four treatment claims. Invalid source references and claims over 30 words
still fail closed.

## Basis

- [GLOBAL-01 improvement hypothesis](2026-08-29-global-01-improvement-hypothesis.md)
- [GLOBAL-01 first result](2026-08-29-global-01-first-run-result.md)
- [BenchmarkQED datasets](https://microsoft.github.io/benchmark-qed/datasets/)
- [BenchmarkQED AutoQ assertion contract](https://microsoft.github.io/benchmark-qed/cli/autoq/)
- [BenchmarkQED AutoE example](https://microsoft.github.io/benchmark-qed/notebooks/autoe/)
