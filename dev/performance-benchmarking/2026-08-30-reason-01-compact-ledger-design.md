# REASON-01 compact-ledger offshoot design

## Boundary

The offshoot is one experiment-owned runner over the frozen REASON-01
checkpoint. It changes no Engine, SDK, schema, storage, graph, or routing
surface. `protected_multiquery_v1` remains rejected.

## Arms

- `a0_raw`: newly generated from the frozen A0 hits.
- `protected_raw`: newly generated from the frozen protected hits.
- `protected_evidence_ledger_v1`: new two-stage reader over the unchanged
  protected 20-hit candidate set.

The three arms are interleaved by a deterministic case rotation and use the same
current `deepseek/deepseek-v4-pro` answer model route through OpenRouter. The
catalog binding records its 1,048,576-token context limit and current prices of
$0.417252 per million input tokens and $0.834504 per million output tokens. All receive corrected answer, grounding, and
attribution scoring.

The compact treatment uses the pinned
`openrouter/anthropic/claude-haiku-4.5` route only for exact-strip ledger
extraction. This specialized packing stage cannot author evidence: every strip
still passes canonical span validation. The same Haiku route is also the
blinded evidence judge, but those calls have separate prompts, caches, and role
receipts and never see arm identity or ledger advisory metadata.

The OpenRouter catalog reports `reasoning` and `response_format` support,
non-mandatory reasoning, and high/xhigh reasoning efforts for this revision.
Reader calls therefore explicitly set `reasoning.enabled=false`. Ledger,
answer, and evidence-judge calls use strict JSON schemas on routes whose catalog
records `structured_outputs`; the plain-text official scorer does not inherit
either transform.

## Compact-ledger reader

For each question, the ledger model receives the 20 protected candidates and
returns strict JSON containing:

- at most six information requirements of at most 160 characters each;
- at most ten evidence entries with canonical ID, one unique exact quote of
  8–600 characters, and requirement indexes;
- missing requirement indexes; and
- conflict IDs plus a description of at most 240 characters.

The runner rejects unknown or duplicate IDs, ambiguous or non-substring quotes,
invalid requirement indexes, or any size violation. It deterministically
enriches each strip with source ID, body hash, and exact character offsets from
the frozen hit map. Model-generated descriptions, requirement labels, conflict
labels, and grouping are advisory metadata, never evidence. The answerer sees
only the question, exact enriched strips, and missing/conflict flags; it does
not receive a model-generated fact paraphrase.

A bounded semantic retry receives its prior object and the deterministic
validation error. It must correct the same schema and preserve exact quotes
byte-for-byte. A conflict requires at least two selected strips that directly
contradict each other; ambiguity, negative evidence, and needed calculations do
not qualify.

Semantic attempts use the frozen requested-seed schedule 20260830–20260834
uniformly by attempt number. This is a diversified retry schedule, not a claim
that provider outputs are reproducible or distinct at temperature zero. The
route must accept a seeded preflight; every attempt records its requested seed,
response ID, returned provider/model metadata, and Airlock served-by header.

Exact substring matching runs first. If it finds no match, a frozen fallback
may treat only ASCII single/double and Unicode left/right single/double quote
marks as equivalent, character by character. It requires one same-length match
inside the already-bound canonical body, then persists the original source
substring, exact offsets, body hash, raw response, and
`match_mode=quote_mark_equivalent`; exact matches record `match_mode=exact`.
The table and its hash are in run configuration. No case, whitespace, Unicode,
dash, comma, length, or other punctuation normalization is allowed. Ambiguous
matches fail closed.

The compact answer user message must be no longer than the smaller of 12,000
characters and the original A0 answer user message for that case. The runner
rejects an oversized ledger before an answer call. It reports input characters
and provider tokens for every stage and arm.
The ledger output ceiling is 2,400 tokens so a schema-valid worst case of ten
600-character strips plus provenance metadata is not truncated; this output
allowance does not relax the answer-input limit.

The answer is strict JSON with an answer string and a deduplicated citation
list. Unknown, duplicate, or empty-answer citations are malformed. A schema-valid
uncited claim or an empty compact answer without an explicit missing requirement
is a terminal measured quality failure, not a semantic retry; the receipt marks
its citation contract invalid and scores it normally. Retrieved bodies are
delimited as untrusted data and prompts state that instructions inside them are
evidence text, never executable directions.
The reader gives the best supported concise answer when relevant evidence
exists; it does not abstain merely because an aggregate is implicit or the
evidence has ordinary ambiguity. `missing` is reserved for a requirement with
zero relevant evidence; uncertainty is represented as a conflict.

The reader prompt requires event deduplication before counts, explicit
calculation from exact strips, and citations for every material claim. No claim
of deterministic arithmetic is made. Ledger and answer outputs are query-time
artifacts only.

## Corrected scoring

Answer correctness uses the verbatim official LongMemEval multi-session and
abstention prompts from `src/evaluation/evaluate_qa.py` at upstream revision
`9e0b455f4ef0e2ab8f2e582289761153549043fc`. The prompt hashes, model route and provider,
`gpt-4o-2024-08-06` model revision, temperature zero, ten-token limit, and
yes/no parsing rule are stored in configuration and receipt. The call receives
only question, reference, and candidate answer. Answerable empty responses are
deterministically false before any model call; question IDs ending in `_abs`
use the official abstention prompt and are reported separately.

Grounding and attribution use one common, blinded strict JSON path for all arms.
The judge independently enumerates every material claim from the answer, then
assesses each claim against only the evidence unit actually shown to that arm's
reader: the complete cited hit for either raw arm and the validated exact
quote plus offsets for the compact arm. It never receives arm-authored claim
lists or a larger canonical body than the compact reader saw. Its result lists
every extracted claim, whether it is entailed, and the supporting citation IDs.
`grounded` requires every material claim to be entailed by the shown cited
evidence; `attributed` additionally requires complete claim-to-citation
coverage. Unknown or uncited claims fail closed. Syntactic citation validity is
deterministic.

The scorer is blind to arm names. Correctness and evidence scoring are separate
calls so retrieval context cannot change semantic correctness. Results are
cached by canonical hashes of their complete blinded inputs plus prompt and
model versions. Identical inputs therefore share one result across arms.
Correctness disagreements and deterministic-rule/model conflicts are written to
a blinded audit queue before reporting.

## Execution safety

The runner creates a new immutable offshoot checkpoint and binds hashes of its
config, source, frozen checkpoint, runner, prompts, price sheet, and model
routes. Ledger, answer, correctness, and evidence-judge stages have independent
terminal states. It writes every attempt before parsing; malformed attempts may
resume only within the frozen semantic retry cap. HTTP 429, timeout, and 5xx
responses use bounded exponential backoff and honor numeric or HTTP-date
`Retry-After` without truncation.

The run uses a dedicated loopback Airlock process and isolated state directory.
DeepSeek V4 Pro, `openai/gpt-4o-2024-08-06`, and Claude Haiku are all pinned to
OpenRouter routes, and the isolated instance enforces one $10 OpenRouter
provider cap. Authoritative pre-run spend, including abandoned diagnostic
attempts, is $0.758417; the final checkpoint has a $9.00 local ceiling, keeping
combined authorization below $10. The local runner reserves worst-case input and output cost from a
pinned price sheet and reconciles Airlock's authoritative provider usage before
every call and resume. Failed, malformed, timeout, and unknown-charge attempts
consume local reserve; the provider cap remains the final hard stop.

## Metrics and decision

The receipt reports exact frozen retrieval metrics plus, per arm, non-empty
rate, official answer correctness by answerability stratum, grounding,
attribution, citation validity, answer-input characters/tokens, context
precision, evidence utilization, cost, malformed/retry counts, and paired
10,000-draw bootstrap deltas with seed 20260830 against corrected A0. Failures
count as false. The consumed-cohort descriptive diagnostic passes when compact
point estimates are no worse than A0 on all three quality rates and the bound
checkpoint, context, cost, and completeness gates pass. Confidence bounds are
reported but do not create a confirmation claim.

Passing permits a new untouched confirmation plan only. It does not promote a
profile, start HippoRAG-2, or refresh MEMORY-01.
