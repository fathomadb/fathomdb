# Mem0 OSS baseline readiness plan

> **Program relationship.** This is readiness work for track M0 in the
> [overall performance benchmarking and experiments program](../PROGRAM.md).
> Its prerequisites remain blockers until the portfolio board records them as
> resolved.

## Objective

Run Mem0's official `memory-benchmarks` OSS + Qdrant pipeline once before
adding a FathomDB adapter. This is native reproduction, not a reuse of the
historical Chroma/local-airlock substitute.

## Frozen inputs and contract

- Clone `mem0ai/memory-benchmarks`, then record its checked-out commit,
  requirements digest, compose digest, and resolved container image digests.
- LOCOMO is ready: raw SHA-256
  `79fa87e90f04081343b8c8debecb80a9a6842b76a7aa537dc9fdf651ea698ff`,
  normalized corpus digest
  `e9999b551ac67e899f0008c9ec446cecce937ce12f01f2f08cefa9f690fc4c7c`,
  272 sessions and 1,443 eligible questions.
- LongMemEval is absent. Acquire only through the pinned official harness,
  then record every downloaded asset's path, digest, license, IDs, and count
  before evaluating it.
- Freeze the official defaults as actually observed in the pinned checkout:
  OSS backend, extraction `gpt-4o-mini`, embedding `text-embedding-3-small`,
  answerer/judge `gpt-4o`, top-k 200, and cutoffs 10/20/50/200.

## One-run sequence

1. Create an isolated environment; install the pinned requirements.
2. Start the official Mem0 + Qdrant compose topology under a unique project
   name; record health and resolved configuration.
3. Run LOCOMO once, then LongMemEval once with `--all-questions`; preserve raw
   output outside git and commit only a pointer manifest/result digests.
4. Only after native reproduction completes, implement the FathomDB arm at the
   official ingest/search seam. Both arms must receive identical ordered raw
   conversations, IDs, questions, answerer, judge, prompts, and budget.

## Stop conditions

Do not start a citable run until Docker daemon access, an official
`OPENAI_API_KEY`, LongMemEval acquisition, and a spend ceiling are available.
Current environment has Docker CLI but no daemon access, no official key, and
no LongMemEval payload. Report that as `blocked_prerequisite`, never as a
baseline result.
