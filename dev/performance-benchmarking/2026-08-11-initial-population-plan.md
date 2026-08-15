# Initial performance-benchmark population plan — 2026-08-11

## Status

Self-reviewed on 2026-08-11. The corrections from that review are incorporated below.

## Objective

Create the first current, reproducible FathomDB retrieval-quality and linked
performance records from the local IR-C gold corpus. This is a baseline for
future competitor comparisons; it is not a claim of superiority over Mem0,
GraphRAG, HippoRAG, or any other system.

## Scope and constraints

- Candidate: the clean instrumentation checkout at
  `/tmp/fathomdb-0.8.24-instrumentation`, revision
  `bf2d88a0b5df` (`v0.8.22-81-gbf2d88a0`). This is a local candidate, not a
  released 0.8.24 package.
- Corpus: `data/corpus-data/raw/`, pinned by the candidate's
  `tests/corpus/snapshot.json` (10,506 documents).
- Gold: `data/corpus-data/eval/ir_gold/all.gold.json` (4,597 queries;
  SHA-256 `4caabddf7ce55f417e639e3c169fe2035b09c231f36d2f39d293a596373de2bb`).
- Retrieval: FTS-only, `Engine.search_text_only`, explicit `limit=10`, with
  evidence-recall at 5 and 10. This avoids embeddings and answer-model spend.
- Spend: zero dollars. Do not invoke EARP answer arms, external services, or
  competitor implementations in this population.
- Raw outputs belong under the uncommitted `experiments/runs/` tree. This plan
  and its result note only point to those artifacts and their hashes.

## Execution

1. Build the candidate's Python binding into an isolated temporary virtual
   environment. Do not alter the shared project environment and do not use an
   editable install or `maturin develop` from the worktree.
2. Verify the corpus snapshot's every shard digest and document count, the
   gold digest, and the candidate checkout's cleanliness before execution.
3. Invoke `run_characterization` once with a timestamped experiment name and
   `experiments_root=/home/coreyt/projects/fathomdb/experiments`. Preserve the
   generated `record.json`, resolved configuration, workload manifest,
   per-query sidecar, and observed-cost document unchanged.
4. Read the generated workload manifest. Only if it predeclares exactly one
   repetition and the treatments `fresh_store` and `fresh_store_warm_query`,
   invoke `fathomdb-performance characterization` with those exact values.
   This produces a linked descriptive performance run, not inferential timing
   evidence.
5. Validate that both artifact directories are complete and that the
   performance artifact names the quality run as its parent. Record the run
   identifiers, paths, digests, candidate revision, corpus/gold identities,
   results, and typed unavailable fields in one durable result note in this
   directory.

## Acceptance criteria

- The quality run is `complete`, reports 10,506 ingested documents and 4,597
  retrievals, and includes its immutable workload manifest.
- The performance run is complete or records a typed invalid/unavailable
  outcome. A missing performance result is not silently treated as a number.
- The result note labels the performance observations as one-repetition,
  descriptive evidence and does not compare them with historical runs made on
  another candidate or environment.
- No raw corpus, gold payload, database, result sidecar, or timing sample is
  copied into `dev/performance-benchmarking/`.

## Self-review and fixes applied

| Review finding | Correction in this plan |
| --- | --- |
| The example EARP configuration has a placeholder gold digest and cannot safely drive this run. | The runner receives the verified local gold identity directly. |
| The historical S6 results are useful context but predate the present manifest and observed-cost v2 contract. | A fresh quality run creates a new workload manifest before any performance measurement. |
| The normal performance writer allows a single predeclared repetition only as descriptive evidence. | The plan explicitly preserves that limit and forbids significance or regression claims. |
| Shared Python environments can contaminate candidate provenance. | Build and execution use a dedicated temporary virtual environment. |
| Existing competitor targets use different tasks and metrics. | This initial population makes no competitor comparison or superiority statement. |
| The worktree directory name implies a 0.8.24 release but its Git identity is an unreleased post-0.8.22 candidate. | The exact revision and release status are recorded; the results will not be labeled as a registry release. |
| “Committed result note” could incorrectly imply a required Git commit while raw artifacts remain uncommitted. | The plan calls for a durable tracked note and leaves commit ownership to the normal review workflow. |
