# EARP abstention characterization result

## Decision question and claim class

Can a raw fused-retrieval-score threshold separate positive from negative
queries, and how does that descriptive signal compare with the answerer
model's own decision to answer from the retrieved context? This is a
diagnostic characterization. It is not a production abstention design or an
answer-quality result.

## Systems and provenance

The score-threshold arm used FathomDB's FTS-only fused retrieval output on the
frozen IR-C reuse-tier campaign. The judgment arm asked the existing EARP
answerer whether the supplied retrieved context supported an answer. It made
no engine or public-API change.

The score-recording experiment was based on
`feat/earp-g8-score-recording-20260809` at `6fc58ec6`. That commit changes the
evaluator's per-query sidecar, schema, and tests to carry per-hit scores. It is
deliberately **not** included in this documentation bundle and remains a
separate product/evaluator decision. The later G-9 measurement and write-up
were not committed before the original experiment worktree disappeared, so no
distinct source commit can be cited for that result.

## Corpus, gold, and protocol

The characterization used `ir-c-reused-v2` over the frozen 10,506-document
corpus: 4,597 queries, including 4,472 retrieval-scoreable positives and 125
negatives. Gold SHA-256 was
`4caabddf7ce55f417e639e3c169fe2035b09c231f36d2f39d293a596373de2bb`;
corpus SHA-256 was
`fe973fcd49fbbda083158f69fe720f17858ab8528e171fa2188eec84131c7d4e`.
The mixed-license corpus and query payloads remain local and are not
redistributed.

G-8 searched a single threshold over the observed top-1 fused scores and
reported balanced accuracy for positive-versus-negative separation. G-9 used
the answerer's judgment under the same retrieved-context shape. Five priced
G-9 attempts encountered service restarts or memory exhaustion before one
attempt reached 4,303 of 4,597 queries (93.6%). The total recorded spend was
$5.0057. This was one incomplete successful execution, not a repeated-run
protocol.

## Result and uncertainty

The positive and negative median top-1 fused scores were both 0.0968. The best
observed single score threshold reached balanced accuracy 0.5472, close to
chance. The answerer-judgment arm reached descriptive balanced accuracy
0.7835 on the recovered labeled outcomes.

No confidence intervals survive with the historical record. The score
threshold was selected and evaluated on the same observed data, while the
judgment result came from an incomplete run after several failed attempts.
Those facts make both values characterization evidence, not unbiased estimates
of deployment performance.

The answer-arm gold has zero free-text `answers` fields. Its originally
declared `answer_accuracy` therefore collapses to an abstention-rate-like
quantity and cannot measure answer correctness. The balanced-accuracy values
above came from relabeling per-query outcomes by the true query class; they do
not repair the missing answer gold.

## Artifact availability

The original G-8/G-9 raw run directories and campaign-3 notes were not present
in the primary checkout when this record was prepared. The surviving evidence
is the source commit `6fc58ec6` for score capture and the contemporaneous
project memory entry `earp-tiered-run-program-20260809-complete.md` outside the
repository. Because neither raw sidecars nor a committed G-9 result record are
available, this result is historical and not presently reproducible.

## Nonclaims

This result does not establish answer accuracy, a calibrated production
threshold, model-independent abstention performance, a public API contract, or
a repeated-run comparison. It motivates treating score capture and any
judgment-based abstention feature as separate future decisions. In particular,
it does not authorize landing the `retrieved_scores` evaluator change.
