# HippoRAG-2 baseline readiness plan

> **Program relationship.** This is the parked H0 track in the
> [overall performance benchmarking and experiments program](../PROGRAM.md).
> Its official-runtime, credential, and corpus prerequisites remain explicit
> blockers.

## Objective

Run the official HippoRAG-2 MuSiQue reproduction once before any controlled
FathomDB comparison. Local MuSiQue is a smoke asset only unless it is
reacquired to the official representation.

## Integrity finding

The reported local discrepancy is real. The on-disk 4,834-row file has
SHA-256 beginning `3cff37` and lacks `question_decomposition`; it matches the
old M1 harness. The current acquisition manifests expect a newer file with
SHA-256 beginning `83e0e4` and that field. Although all 1,000 official query
IDs overlap locally, the official set has a shared 11,656-document corpus,
whereas the local file has per-question distractor pools. They are not
interchangeable for a citable HippoRAG result.

## Frozen inputs and knobs

- Pin official HippoRAG commit
  `c617143f01477243992a63b2e2151cc003dd3b21` and hash its requirements.
- Acquire the official MuSiQue query/corpus/reproduction bundle and record
  digests, including its supplied OpenIE cache, before execution.
- Record Python/runtime, GPU, model, endpoint, cache, and runner settings.
  Expected defaults to verify from the checked-out runner are retrieval 200,
  linking 5, QA 5, embedding batch 8, `nvidia/NV-Embed-v2`, and
  `gpt-4o-mini`.

## One-run sequence

1. Make a Python 3.10 isolated environment matching the official requirements
   and validate the official bundle digests.
2. Use the supplied OpenIE cache; record whether cache is read-only or any
   extraction is performed. Start one fresh native index/retrieval/QA pass.
3. Score official supporting-passage retrieval separately from MuSiQue EM/F1,
   stratified by 2/3/4-hop questions. Preserve raw artifacts outside git.
4. Only then add a FathomDB document-retrieval arm over the identical official
   passages, question IDs, aliases, support labels, reader, and budget.

## Stop conditions

Do not use the stale local file as a baseline. The current environment lacks
the official Python/runtime packages and an official OpenAI key; GPU capacity
is available but not sufficient. Missing official corpus/cache/key is a typed
`blocked_prerequisite`, not a failed benchmark.
