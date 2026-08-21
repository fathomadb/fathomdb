# Competitor-comparison plans — 2026-08-11

> **Program relationship.** This document supplies the shared native-comparator
> contract for tracks M0, G0, and H0 in the
> [overall performance benchmarking and experiments program](PROGRAM.md).
> Current priority, blockers, and sequence live there.

## Shared comparison contract

Each comparison must give both arms the identical frozen raw input and
question IDs. Record corpus, gold, package/image, prompt, and configuration
digests; keep any arm-specific derived artifacts separate. A missing digest or
execution provenance makes that cell typed-invalid. Run one isolated fresh
process per arm for the initial characterization only; it is descriptive, not
a winner claim. Do not run a broad repetition campaign until a reviewed plan
declares its statistical rule.

Separate ingestion/index time, retrieval/query time, and answer/judge time.
Freeze and report all knobs extracted from the actual comparator runner, then
use the same answerer, judge, prompt, context budget, and ordering protocol
where the benchmark permits it.

## FathomDB versus Mem0 OSS

Use Mem0's official `memory-benchmarks` harness, separately for LongMemEval
and LOCOMO. First pin its repository commit, Docker images, Mem0 OSS server,
Qdrant image/configuration, model identifiers, and dataset digests, then
reproduce Mem0 without FathomDB. Add FathomDB only at the harness's
ingest/search seam: both systems receive the same raw conversations in the
same order, session/user IDs, questions, and gold. Mem0 keeps its own memory
extraction; FathomDB must not consume Mem0-extracted memory artifacts.

Local starting assets are `data/corpus-data/raw/locomo10.json`,
`data/corpus-data/eval/0.8.3-locomo-memory-gold.json`, and
`dev/plans/runs/0.8.3-locomo-corpus-manifest.json`. They cover 1,443 LOCOMO
questions but no knowledge-update class. LongMemEval is not locally
materialized, so it must be acquired and hashed before a citable run.

Extract: harness backend/top-k/cutoffs, answerer and judge settings/prompts;
Mem0 extraction and embedding models, dimensions, entity/BM25 settings, and
Qdrant configuration; FathomDB search and projection settings. Report the
harness quality metrics by question class plus retrieval@K if exposed.

## FathomDB versus Microsoft GraphRAG

Use BenchmarkQED AP-News global sensemaking. The local raw corpus at
`data/corpus-data/raw/apnews_benchmarkqed/` contains 1,397 articles and has
archive SHA-256
`2f70dda22a9f261f285c94f3ac13a8f0df60b69fe3df1b5853b47b372065a66f`.
First reproduce the retained 15-document GraphRAG 3.1.0 witness described in
`dev/plans/runs/0.8.4-vs-microsoft-graphrag.py`; scale to the full corpus only
after that succeeds.

Then compare GraphRAG native global search with a clearly labelled FathomDB
controlled global-synthesis adapter over the same frozen documents and AutoQ
global question IDs. Pin GraphRAG/package lockfile, YAML, prompts, model and
embedding endpoints/dimensions, chunking, entity types, gleanings, community
settings, cache state, global query token limits, map/reduce settings, and
concurrency. Pin the FathomDB preprocessing, retrieval/coverage selection,
and map/reduce budgets too.

Use blind order-swapped AutoE judgements for comprehensiveness, diversity, and
empowerment, retaining directness and length controls. This is near-exact, not
algorithmically identical: FathomDB does not currently provide GraphRAG's
native Leiden community-report hierarchy.

## FathomDB versus HippoRAG-2

Use HippoRAG-2's released MuSiQue reproduction corpus/evaluator. Pin the
official HippoRAG release/commit, lockfile, corpus/evaluator hashes, OpenIE
and embedding configuration, caches, reader/reranker model, and runner
defaults. First reproduce HippoRAG's own result. For the controlled run both
arms receive exactly the same passages, question IDs, answer aliases, and
support labels. FathomDB receives documents directly; it must not use
HippoRAG's extracted graph/triples except in a separately labelled
shared-extraction ablation.

The local smoke corpus, `data/corpus-data/raw/musique_dev.jsonl`, has 2,417
answerable questions (1,252 two-hop, 760 three-hop, 405 four-hop), but its
current hash conflicts with
`dev/plans/runs/0.8.2-m1-corpus-manifest.json`. Resolve that discrepancy
before any citable local run; use the official reproduction bundle for a
headline comparison.

Extract official retrieval/linking/QA top-k values, iterative reasoning
limits, model/batch settings, and cache/force flags. Report
all-supporting-passages@K and support recall/precision by hop count separately
from official MuSiQue answer EM/F1.

## Entry conditions

No comparison is authorized until its native comparator reproduction passes,
the corpus and gold identities are validated, the operator approves required
credentials/spend and licenses, and the complete knob ledger is committed.
The initial FathomDB-only IR-C timing artifact is not a substitute for any of
these comparison runs.
