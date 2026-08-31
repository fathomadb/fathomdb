# EXTRACT-01 implementation and result

## Fixed comparison

- Corpus: the 78 `knowledge-update` cases in LongMemEval-S cleaned.
- Baseline: canonical chat turns searched with the accepted A0 FTS profile.
- Treatment: the same canonical turns plus question-blind extracted entities
  and fact edges ingested through FathomDB's native `fathomdb.extract.v1`
  provider seam.
- Extraction and answering: pinned `gemini-3.1-flash-lite`, reasoning disabled,
  with an 8.5-second minimum call interval.
- Evaluation: the official LongMemEval knowledge-update rubric with pinned
  `claude-sonnet`, reasoning disabled.
- Budget: hard stop before $20; every paid response is charged and
  checkpointed outside the repository.

The question, reference answer, `has_answer`, and answer-session labels never
enter the extraction prompt. The LongMemEval oracle is evaluation-only.
Extraction precision remains unscored because the corpus has no human atomic-
fact extraction gold.

## Implemented controls

- One fresh, doctor-attested FathomDB 0.8.23 database per question and one for
  the lifecycle cell.
- Transactional A0 FTS writes without `drain()`: in 0.8.23, `drain()` attempts
  pending dense work and correctly requires an embedder, while this profile is
  intentionally FTS-only.
- External checkpoint after every model response and completed case; resume
  executes only missing cells.
- HTTP retry for timeouts, 429s, and 5xx responses, plus two bounded semantic
  retries for malformed model output. An explicit Airlock protection-circuit
  cooldown stops immediately so internal retries cannot extend that cooldown.
- Deterministic single-document ELPS repair: canonical source attribution,
  safe entity/relation names, bounded confidence, canonical citation mapping,
  malformed-edge removal, and calendar-real ISO timestamps only.
- A chunk that still cannot produce valid ELPS after semantic retries is
  counted as an empty extraction outcome. Cost, network, and budget failures
  still stop the run.
- Safe repository receipts contain aggregates and hashes only. Corpus text,
  retrieved context, answers, and model output remain in mode-0600 external
  artifacts.
- Storage uses committed logical SQLite page allocation rather than sampling
  only the main file while WAL changes are live.

## Initial smoke

The original Luna smoke exhausted its 4,096-token completion budget without a
valid JSON body. It stopped after one call and has a separate blocked receipt.
An OpenAI nano attempt then checkpointed 48 cases before the provider reported
that no credits remained. Those cells were not mixed into the result. A fresh,
uniform run used Gemini 3.1 Flash-Lite after a real-format extraction preflight
and Claude Sonnet after a real-format judge preflight. The primary retained the
24-fact document cap. Total observed task spend remained below $5, including
the abandoned attempt and preflights.

## Result

The complete receipt is
[`extract-01-knowledge-update-20260823T2236Z-59e805cb`](../../experiments/runs/extract-01-knowledge-update-20260823T2236Z-59e805cb/record.json).

- Raw A0 answer accuracy: 64/78, or 82.05%.
- Raw plus native ELPS answer accuracy: 65/78, or 83.33%.
- Paired outcomes: 2 treatment wins, 1 treatment loss, 63 both correct, and 12
  both wrong. The +1/78 delta is descriptive, not a general quality claim.
- Evidence recall at 10: 77/78 raw and 78/78 treatment.
- Source-link completeness: 100%; 2,096 extracted rows; one of 390 extraction
  documents exhausted semantic retries and was counted as empty.
- The 994 extracted edge outputs all carried bounded confidence, but confidence
  was saturated (mean 0.997, median 1.0) and remains uncalibrated without human
  atomic-fact gold.
- Mean native treatment ingest: 25.07 ms. Physical logical-page storage
  amplification was 1.0 at this per-case scale because extracted rows fit in
  the pages already allocated by the raw corpus.
- Lifecycle: conflict detected, post-erasure absence passed, and zero active
  orphan edges, but value-changing Boston-to-Austin facts remained competing;
  `supersession_applied` was false.
- Primary receipt cost: $2.7983, within the fixed $20 cap.

Within the fixed knowledge-update claim, the acceptance rule therefore yields
`do_not_adopt_unconsolidated_extraction`. Keep raw A0 memory as the product
default for this query shape. The result does not establish preferences,
episodes, or general long-term-memory quality. Reopen EXTRACT-01 only after
native value-changing consolidation is implemented or a new, separately
preregistered comparison is authorized.
