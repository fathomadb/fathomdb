# REASON-01 retrieval-to-answer diagnosis

## Status

This is a post-hoc diagnosis of the rejected REASON-01 treatment. It does not
revive `protected_multiquery_v1`, tune against the consumed 109-question
held-out cohort, or authorize another paid run.

## Conclusion

Keep the protected candidate-generation mechanism as a research ingredient,
but do not pass its 20 raw chunks directly to the answerer. The next bounded
hypothesis should retrieve broadly, select and pack a small set of exact
evidence strips, reason over an explicit evidence ledger, and verify every
answer claim against its citations.

The observed loss is primarily a retrieval-to-reader interface failure plus a
confounded answer scorer. It is not evidence that the additional answer-bearing
turns were harmful by themselves.

## Post-hoc observations

The analysis uses the frozen REASON-01 checkpoint and the official
LongMemEval `has_answer` turn labels. Abstention cases have no answer-bearing
turn and are excluded from turn-recall calculations.

| Observation | Result |
| --- | ---: |
| Answer-labelled cases | 102 of 109 |
| Exact answer-turn recall, A0 | 0.6696 |
| Exact answer-turn recall, protected | 0.7569 |
| Cases with higher exact answer-turn recall | 20 |
| Cases with lower exact answer-turn recall | 0 |
| Cases with all answer turns present, A0 | 49 |
| Cases with all answer turns present, protected | 60 |
| Protected appended chunks | 1,090 |
| Appended exact answer-bearing turns | 27 (2.48%) |
| Appended chunks from a gold session | 434 (39.82%) |
| Appended assistant-role chunks | 781 (71.65%) |
| Mean appended chunk length | 1,540 characters |
| Exact-recall gains that still produced an empty protected answer | 18 of 20 |
| Measured correctness losses caused by protected abstention | 7 of 9 |
| Empty answers marked correct by the judge | 13 cells |
| Identical paired answers with different correctness labels | 5 cases |

The protected arm retained the complete A0 prefix. It therefore did not remove
evidence that A0 could use. It added roughly ten long chunks per question, most
of which were verbose assistant material and few of which were exact labelled
answer turns. Protected answers fell from 35 non-empty responses to 24.

The original session-level result remains directionally valid, and exact
turn-level recall independently confirms the retrieval improvement. The reader
failed to convert it into answers.

## Measurement correction before another answer run

REASON-01's answer judge received the question, reference, complete retrieved
context, candidate answer, and citations. That makes `answer_correct` depend on
the retrieval arm and conflates correctness with grounding. It also produced
impossible or unstable cells, including correct labels for empty answers to
answerable questions and different labels for identical answers.

The next scorer contract should:

1. score answer correctness from only question, reference, and candidate answer,
   using the official LongMemEval prompt and a pinned scorer;
2. score answerable and abstention cases separately, with an empty answer always
   incorrect for an answerable case;
3. score grounding per atomic answer claim against only the cited evidence;
4. score citation entailment and completeness separately from syntactic citation
   validity;
5. report official turn-level recall, context precision, and answer quality
   together; and
6. send blinded paired disagreements and rule/model conflicts to a small human
   audit rather than allowing one context-sensitive judge call to decide them.

The existing answer outputs can be re-scored under this contract without
rerunning retrieval or answer generation. That re-score is a measurement repair,
not a retroactive promotion decision.

## Proposed bounded treatment

Working name: `protected_evidence_ledger_v1`. It remains experimental and
caller-selected for explicit relationship intent.

### 1. Preserve broad candidate retrieval

- Reuse the unchanged A0 plus protected multi-query candidate generation.
- Treat A0 hits as protected candidates, not ten context items that must all be
  emitted.
- Keep the candidate pool separate from the reader context limit.

### 2. Select a compact evidence set

- Split candidates into exact, source-linked turns or extractive sentence spans.
- Preserve canonical source ID, body hash, span location, role, timestamp, and
  the already-applied `ReadView` for every strip.
- Select a set that jointly covers the question's bounded information
  requirements. Score relevance and requirement coverage, then penalize
  redundancy, near-duplicate event descriptions, verbosity, and contradiction.
- Start development with at most 8–10 strips and no more reader tokens than A0.
  The final limit must be selected on development data, not the consumed
  held-out outcomes.
- Order strips by the reasoning requirements or chronology, not by retrieval
  branch round-robin.

This is set selection, not another top-k ranking pass. Multi-hop questions need
complementary evidence, while the REASON-01 failure shows that simply appending
individually plausible passages is unsafe.

### 3. Build an ephemeral evidence ledger

Before answering, the reader emits bounded atomic entries:

- fact or event;
- supporting source IDs;
- which question requirement it satisfies;
- same-event or possible-duplicate group;
- conflict or missing-information flag; and
- arithmetic operand when applicable.

The ledger is query-time only. It is not stored as canonical memory and cannot
outlive source erasure or supersession. This avoids repeating EXTRACT-01's failed
value-changing consolidation while retaining TRACE-01 attribution and
TEMPORAL-01 read-view semantics.

### 4. Reason, cite, and verify

- Generate the answer from the ledger rather than from the raw candidate pool.
- Require a citation for every material atomic claim.
- Deduplicate mentions of the same event before counts or sums. The observed
  three-project error is a concrete example of treating two descriptions of one
  project as separate projects.
- Use deterministic arithmetic over ledger operands when the answer is a count,
  sum, duration, or percentage.
- Run one claim-level entailment check against cited strips. Remove unsupported
  claims or perform one bounded repair; never start an unconstrained answer loop.
- Permit abstention only when the ledger identifies a required information slot
  as missing or contradictory. Record the missing slot in the safe trace.

The first treatment should retain the same answer model and reasoning setting
to isolate the packing and verification changes. Enabling a reasoning mode or
changing model class is a second-stage factor only if compact evidence still
fails to synthesize present facts.

## Why this approach is supported

- The official [LongMemEval repository](https://github.com/xiaowu0162/LongMemEval)
  provides turn-level answer labels and recommends a `con` reading method that
  extracts useful information before reasoning. The
  [LongMemEval paper](https://arxiv.org/abs/2410.10813) reports session
  decomposition and query/index expansion as useful memory-design choices.
- [Lost in the Middle](https://aclanthology.org/2024.tacl-1.9/) shows that longer
  context and evidence position can reduce question-answering performance even
  for long-context models.
- [RECOMP](https://openreview.net/forum?id=mlJLVigNHp) shows that selective,
  compact augmentation can reduce the reader's burden while retaining task
  performance.
- [Corrective RAG](https://arxiv.org/abs/2401.15884) uses retrieval evaluation
  and decompose-then-recompose filtering to remove irrelevant knowledge before
  generation.
- [Chain-of-Note](https://arxiv.org/abs/2311.09210) explicitly evaluates
  retrieved passages before answering and improves robustness to noisy context.
- [SetR](https://aclanthology.org/2025.acl-long.861/) treats multi-hop retrieval
  as collective set selection against identified information requirements,
  rather than independent passage ranking.
- [ALCE](https://aclanthology.org/2023.emnlp-main.398/) separates correctness
  from citation quality and evaluates whether citations actually support
  generated claims.
- [RAGChecker](https://arxiv.org/abs/2408.08067) likewise diagnoses retrieval
  and generation separately using claim-level context precision, utilization,
  noise sensitivity, and faithfulness.

These papers support the mechanism, not a FathomDB performance claim.

## Development and validation sequence

1. Re-score the preserved REASON-01 answers with the corrected, blinded answer
   contract and audit all paired disagreements. Do not change the rejected
   decision from this repair alone.
2. Implement offline diagnostics for official turn recall, context precision,
   evidence utilization, noise sensitivity, and conditional abstention.
3. Develop the compact selector and evidence-ledger reader only on the already
   consumed development material. Freeze the candidate-retrieval recipe.
4. Require a three-arm development ablation: A0 raw context, protected raw
   context, and protected compact ledger. This must demonstrate that compacting,
   not a model swap, restores answer and attribution quality.
5. Pre-register one compact configuration on a new untouched multi-hop cohort.
   The consumed 109 LongMemEval cases may remain diagnostic evidence but cannot
   serve as confirmation. MuSiQue is already within the PROGRAM corpus policy;
   a separate agent-memory confirmation set is needed before a shipped routing
   or SDK claim.
6. Only after confirmation may PROGRAM consider a new caller-side relationship
   profile or refresh MEMORY-01. No Engine, schema, storage, graph, or default
   routing change is justified yet.

## Rejected shortcuts

- Do not promote the 20-item raw protected context.
- Do not revive `deep_compact_v1`; it previously lost thirteen grounded answers.
- Do not persist LLM-extracted facts or summaries before native consolidation,
  supersession, provenance, and erasure pass.
- Do not make graph retrieval the default; GRAPH-01 did not improve answers.
- Do not hide the reader failure by switching to a stronger model in the first
  ablation.
- Do not tune thresholds or pack size against the consumed 109-question cohort.
