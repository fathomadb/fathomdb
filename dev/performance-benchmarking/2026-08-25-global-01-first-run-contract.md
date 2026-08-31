# GLOBAL-01 first-run measurement contract

## Decision

Test whether the previously observed global-synthesis split reproduces under a
small native Microsoft GraphRAG witness and a matched FathomDB source-linked
map-reduce treatment. This is calibration evidence, not a personal-memory
default gate.

## Fixed comparison

- Corpus: the first 15 ordered AP News BenchmarkQED documents, bound to the
  preserved native GraphRAG inputs by content hash.
- Questions: eight ordered global questions selected by the registered stride.
- Comparator: Microsoft GraphRAG 3.1.0 global search, level 1, dynamic community
  selection disabled, and the preserved settings and prompts.
- Treatment: FathomDB 0.8.23 canonical source documents with
  `source_mapreduce_c_v1`; no embedder, reranker, or graph projection.
- Shared generator: native `deepseek-v4-pro`, thinking disabled. This supersedes
  the unspent `gpt-5.4` binding after the configured OpenAI account returned
  `insufficient_quota`; both comparison arms use the same generator.
- Structured output: DeepSeek's unsupported JSON-Schema transport hint is
  omitted. The preserved GraphRAG prompts still specify the complete JSON
  shape; the community-report prompt additionally forbids Markdown fences and
  surrounding prose. GraphRAG locally validates every returned object against
  its typed schema; a parse failure or incomplete community-report set
  invalidates the native witness.
- Judge: `claude-haiku`, five order-swapped repetitions, reporting
  comprehensiveness, diversity, empowerment, and directness separately.

The executable values and file digests are frozen in
`experiments/configs/global-01/apnews-15doc-first-run.v1.json`.

## Sequence and validity

1. Run the zero-spend preflight. It must bind every document and question,
   verify the native toolchain and prompt files, create a fresh FathomDB, and
   authenticate to Airlock with `AIRLOCK_VIRTUAL_KEY`.
2. After explicit cost and execution authorization, run two native GraphRAG
   global-search questions as the witness.
3. The witness is valid only if indexing and both queries complete without a
   fallback, each answer is non-empty, and required GraphRAG output tables are
   non-empty. Community reports must cover every generated community exactly.
4. Run the eight-question matched comparison only after a valid witness.

Every priced call is checkpointed before the next cell. On HTTP 429, honor the
whole `Retry-After` interval; on 5xx or timeout without that header, use bounded
exponential backoff. GraphRAG indexing may use at most four concurrent calls;
FathomDB answer generation and judge calls are serialized. Resume retries only
missing cells. An incomplete judge object is checkpointed by usage and response
hash, then retried at most twice with the same rubric and an exact-shape JSON
instruction.

## Cost and stop conditions

Projected total spend remains conservatively bounded at $4.22 using pinned
DeepSeek peak pricing. The hard cap is $6.00. Neither is
authorization. HITL authorized execution under the $6.00 hard cap on
2026-08-29; the typed configuration records that authorization.

The registered read is near-parity only when the clustered two-sided 95%
bootstrap lower bound is at least 0.45 for each headline metric. A lower bound
above 0.50 on every headline metric supports a descriptive win. Otherwise the
result is split; directness remains separate.

Stop without a comparison verdict on corpus/configuration mismatch, an invalid
native witness, an incomplete arm, a retry budget exhaustion, or the cost cap.
Report a split result without post-hoc tuning.
