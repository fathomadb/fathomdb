# GRAPH-01 bridge-completion measurement contract

**Frozen:** 2026-08-29 before executing `protected_bridge_v1`.

## Decision and claim boundary

Does an explicit, provenance-backed bridge-completion profile improve MuSiQue
supporting-evidence retrieval or answer F1 over the previously accepted fused
baseline enough to justify graph extraction and maintenance?

This is a registered reuse of a previously observed 300-question cohort, not a
pristine held-out claim. It may accept or reject only this mechanism on
MuSiQue-style multi-hop retrieval. It cannot establish a general agent-memory
default.

## Frozen inputs

- Corpus: `data/corpus-data/raw/musique_dev.jsonl`, SHA-256
  `3cff37fd7221506a343a125cf7ca20aab7cd09877e376122da9627e1b935b26f`.
- Question-blind paragraph extractions:
  `data/corpus-data/graph-cache/0.8.2-m1-v1/extractions.json`, SHA-256
  `7ca969520e7c79dab83a3cce0800f8e87369b3aed7b7264577f6959f397373d4`.
- Cohort: the 300 question IDs in the valid prior M1 result
  `dev/plans/runs/0.8.2-m1-verdict-gpt54.json`, SHA-256
  `3f4061119a278045c6c905f5fc5db9705ef733a2b051541042a49012f14f2019`.
- Extraction identity: local Qwen3.6-27B, inherited from the pinned cache.
- One paragraph lacks a cached extraction. The zero-spend preflight discovered
  this before execution. The document remains in both retrieval arms and is
  represented by an empty graph contribution; no mixed-model backfill is
  allowed. The safe receipt reports the missing count.
- FathomDB runtime: 0.8.23. Every execution builds a new database. No old
  graph database is reused as runtime state.

## Arms

The control is `fused_rrf_k60`: RRF of the existing in-harness BM25 and
CLS-pooled BGE passage-dense rankings, with `k=60`, returning ten passages. It
is the frozen M1 comparator and is stronger than a lexical-only control.

The treatment is `protected_bridge_v1`:

1. Rebuild question-namespaced entity and relation projections in a fresh
   FathomDB. Each relation edge carries its exact source paragraph ID.
2. Normalize Unicode letter/number/mark and symbol tokens without transliteration.
   Admit an edge only when both normalized endpoints are non-generic, both
   appear in its paragraph's extracted entity set, both appear verbatim in the
   source paragraph, and neither endpoint has a conflicting extracted type.
   Count and reject malformed extracted relations without inferring missing
   fields; retain their source documents in both arms.
3. Resolve query anchors only by exact normalized entity-phrase occurrence in
   the question. Use the control top five as passage seeds.
4. Enumerate simple paths of one or two edges from query anchors to entities in
   seed passages. A promotable paragraph must be the source of an admitted path
   edge, must not already be in the control top ten, and must be in the control
   top twenty.
5. Rank promotions by distinct query anchors, distinct seed passages, shortest
   path, control rank, then paragraph index. Protect control ranks one through
   eight and promote at most two candidates into the ten-passage budget.

This is explicit path completion, not raw BFS, graph-only retrieval,
lexically-seeded PPR, graph/lexical RRF, or index-key enrichment. Those earlier
mechanisms remain rejected.

## Graph-quality and lifecycle eligibility

Before a quality verdict, report total/admitted/rejected edges, exact source
link completeness, endpoint orphans, normalized duplicate entities,
type-conflict co-mingling, inactive or temporally stale edges, graph database
bytes, and storage amplification over the corpus.

Independently judge a fixed-seed sample of up to 100 admitted edges in batches
of ten. The judge sees the source paragraph and extracted subject, predicate,
and object, but no question, answer, supporting label, or retrieval result.
Batch-local opaque audit IDs prevent the gateway's PII guard from transforming
canonical corpus IDs; map them back locally only after exact ID-set validation.
The graph is eligible only if judged edge precision is at least 0.90 and its
Wilson 95% lower bound is at least 0.80. Source-link completeness must be 1.0;
endpoint orphans, active stale edges, supersession-canary hits, and
erasure-canary hits must all be zero.

## Endpoints and decision rule

Primary retrieval endpoint: paired change in all-supporting-passages-present at
10 on the pooled three- and four-hop questions. Secondary endpoints are mean
supporting-passage recall at 10, the two-hop complete-bridge change, treatment
distinctness, and graph add-on latency.
Questions without any labelled supporting passage are undefined for recall and
complete-bridge: exclude the pair symmetrically, report the count, and fail if
only one arm is undefined. They remain in registered answer evaluation.

Use 2,000 question-level paired bootstrap draws with seed `20260829`. A
retrieval win requires all of:

- pooled three/four-hop complete-bridge delta at least `+0.04` with a 95%
  bootstrap lower bound above zero;
- pooled three/four-hop supporting-recall delta at least zero;
- two-hop complete-bridge delta greater than `-0.02`;
- treatment differs from control on at least 10% of questions;
- graph add-on p95 at most 25 ms and graph storage amplification at most 1.5.

Answer confirmation uses `deepseek-v4-pro`, temperature zero and disabled
thinking, over the pooled three/four-hop questions. Identical contexts share
one answer; changed contexts receive matched control and treatment calls. The
same prompt, output limit, answer normalization, and ten-passage budget apply.
A retrieval win is accepted only when paired answer-F1 delta has a bootstrap
lower bound above `-0.02`. Independently, a material answer win may accept the
treatment when answer-F1 delta is at least `+0.04` with a lower bound above
zero and supporting-recall delta is non-negative.

Every graph-quality, lifecycle, cost, completeness, and operational boundary
must pass in either route. Otherwise reject.

## Execution and cost

- Hard cap: $20, authorized by coreyt on 2026-08-29.
- Edge judge: `claude-haiku`; answerer: `deepseek-v4-pro`.
- All paid cells are checkpointed atomically. Resume executes only missing
  cells. HTTP 429 and provider 5xx responses honor `Retry-After` before bounded
  exponential backoff.
- Stop before a quality verdict on input drift, database/doctor failure,
  graph-quality ineligibility, incomplete paid cells, lifecycle failure, or
  inability to reserve the next request under the cap.

## Outputs

The external artifact root retains the fresh database, safe doctor report,
checkpoint, rankings, and per-question observations. The repository receives
the resolved configuration, typed safe receipt, tests, and a concise result
note. Corpus text, model responses, and secrets are never committed.
