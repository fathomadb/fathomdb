# Competitor baseline readiness execution results — 2026-08-11

## Result class

This is a prerequisite-execution record, not a competitor scorecard. No native
comparator completed a valid query/answer run, and no FathomDB comparison arm
was produced.

## Mem0 OSS

The official harness was acquired and pinned at
`mem0ai/memory-benchmarks@4b61c5d31b9c668a12b4f5e78064248a02c82d2b`.
Its requirements, compose, and example-environment SHA-256 digests are,
respectively,
`14cb7961cff56ba10fd9d1cbb18c7842c6a2377565f18ad538d2b439402fd007`,
`aae2317a8c84c1e1106a04a46d3a19f4751a541fd53b427db2a13d3eaff90d6d`,
and `59601fd7d441b855bbfce68b237131691bf2208cb4415f96647d2345fa6672ae`.

LOCOMO integrity is valid. Native execution is blocked before startup:
Docker cannot access `/var/run/docker.sock`, no official `OPENAI_API_KEY` is
available, and LongMemEval has not been acquired or hashed. Status:
`blocked_prerequisite`.

## Microsoft GraphRAG

The exact `graphrag==3.1.0` frozen environment installed successfully in an
isolated `/tmp` venv. A fresh 15-document workspace was materialized from the
validated first-15 corpus witness and native indexing began through Airlock.

The run was intentionally terminated before query completion. During native
indexing, 15 inputs expanded into 1,396 graph-summarization tasks and 121
community-report tasks. No cost ceiling had been declared, so continuing
could make unbounded paid calls. This is `interrupted_cost_guard`, not a failed
GraphRAG result. No comparator answer or FathomDB global-answer arm exists.

## HippoRAG-2

The official source was acquired and detached at
`c617143f01477243992a63b2e2151cc003dd3b21`. Verified digests:

- requirements: `7aa90130c284d576d59a753a98ee791bffb2768ce04c39583231322b182b5c8b`;
- MuSiQue queries: `98ed4e21d3076532f6388d42320fb809599c63a0d8dffca8ece5e41922be6b46`;
- MuSiQue corpus: `73157a03ce3f0b1a5673dd5dc12bb970c24976dbffc688af9eecdd758c97ffcb`;
- supplied gpt-4o-mini OpenIE cache:
  `8540fb7f20bc38ee037e285411b73d9be910e7304f4a4585daee369238040f54`.

The local MuSiQue mismatch is confirmed: its old-M1 file lacks
`question_decomposition` and is not the official shared-corpus representation.
Native execution is blocked before environment creation because Python 3.10
and an official `OPENAI_API_KEY` are absent. Status: `blocked_prerequisite`.

## Required authority before retrying

Provide Docker daemon access and an official OpenAI credential for Mem0;
provide Python 3.10 plus an official OpenAI credential for HippoRAG-2; and
set an explicit GraphRAG dollar ceiling plus a reviewed metering/abort rule.
After those are available, run each native pipeline once and only then build
the matching FathomDB arms over identical inputs and question IDs.
