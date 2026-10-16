---
title: "Slice 40 Unit D results: COMP-03 retrieval levers"
slice: 40
release: 0.8.28
date: 2026-10-16
status: measured; D3 gate PASS; hybrid verdict ADOPT
---

# Slice 40 Unit D results

Measurements run by the orchestrator on 2026-10-16 against the harness at
`slice/0.8.28-40-comp03` (`0edd7a078`) and wheels built from this branch
(`a3830681d` pre-D3, `5b09afd44` D3), CUDA feature on, 2× RTX 3090. The
comp-lineage result note is
`dev/performance-benchmarking/2026-10-16-comp-03-unit-d-result.md` on the
harness branch; attempt roots under `~/.cache/fathomdb-comp/`.

## D1 retrieval sweep, all 500 LongMemEval-S questions (470 scored)

Session Recall@5 by type, D3 engine; the pre-D3 engine differs by one
preference question at k=5 and one at k=10:

| Type | n | FathomDB FTS | FathomDB hybrid | LanceDB FTS | hybrid − FTS [95% CI] |
| --- | ---: | ---: | ---: | ---: | --- |
| single-session-preference | 30 | 0.733 | 0.900 | 0.733 | +0.167 [+0.033, +0.300] |
| temporal-reasoning | 127 | 0.913 | 0.921 | 0.906 | +0.008 [−0.031, +0.047] |
| multi-session | 121 | 0.975 | 0.975 | 0.967 | +0.000 |
| single-session-user | 64 | 0.984 | 0.969 | 0.969 | −0.016 [−0.047, +0.000] |
| knowledge-update | 72 | 1.000 | 0.986 | 1.000 | −0.014 [−0.042, +0.000] |
| single-session-assistant | 56 | 1.000 | 1.000 | 1.000 | +0.000 |
| Overall | 470 | 0.951 | 0.960 | 0.945 | +0.0085 [−0.009, +0.026] |

Decision rule (requirements REQ40-D3) on both engines: (a) preference
lower bound > 0, (b) no other type's upper bound < 0, (c) overall ≥ 0:
**adopt**. Session MRR@10: FTS 0.911, hybrid 0.929, LanceDB 0.902.
FathomDB FTS − LanceDB FTS overall +0.009 [+0.002, +0.019], ahead or
equal on every type. Query p50: FTS 1.4 ms, hybrid 6.8 ms (includes
embedding the question), LanceDB 4.5 ms; hybrid store build 2.4 s for a
median 247 rounds, of which 2.3 s is embedding.

## D2 answer run v2, 60 questions, k = 10, three arms

| Arm | Correct | Accuracy |
| --- | ---: | ---: |
| FathomDB FTS | 49 | 0.817 |
| FathomDB hybrid | 48 | 0.800 |
| LanceDB FTS | 46 | 0.767 |

Pairwise: FTS − LanceDB +0.050 [0.000, +0.117], three discordant, all won
by FathomDB; hybrid − FTS −0.017 [−0.083, +0.050]; hybrid − LanceDB +0.033
[−0.033, +0.100]. v1 at k = 5 was 42/60 for both FTS arms; the depth
change is the largest mover and is not a paired comparison. Evidence
length p50 is 20,326 characters, so the 20,000-character cap binds at
k = 10. Cost $2.34.

## NFCorpus

| Arm | nDCG@10 | MRR@10 | Recall@10 | Empty lists |
| --- | ---: | ---: | ---: | ---: |
| FathomDB FTS, D3 | 0.3266 | 0.5312 | 0.1563 | 15 |
| FathomDB hybrid, pre-D3 | 0.355 | 0.566 | 0.171 | 0 |
| LanceDB FTS | 0.3265 | 0.5283 | 0.1529 | 15 |

Paired hybrid − FTS +0.0292 [+0.0191, +0.0404], 110 wins, 49 losses, 164
ties. Hybrid is the first FathomDB arm to beat LanceDB on NFCorpus by a
margin. Hybrid latency on this run was measured under GPU and CPU
contention and is not reported.

## D3 gate (REQ40-D6): PASS

| Clause | Result |
| --- | --- |
| NFCorpus paired nDCG@10 delta, CI not below zero | +0.0006 [−0.0000, +0.0016], 2 wins, 1 loss, 320 ties |
| COMP-03 per-type recall unchanged within CI | one preference question down at k=5 (−0.033 [−0.100, 0.000]), one up at k=10; every other type identical |
| 10k TC-5 p95 not higher | body search 13.4 → 9.4 ms p50, projected unfiltered 5.0 → 3.0, c10 14.6 → 11.5, c33 12.7 → 9.4, c01 37.1 → 36.5; p95 lower on every multi-term cell |

D3 (`5b09afd44`) stays in the slice.

## Incidents

- One question, `gpt4_7fce9456`, has two whitespace-only rounds; the
  engine rejects blank bodies with `WriteValidationError`. Both sweeps
  stopped at question 457 and resumed after harness fix `0edd7a078`
  (blank rounds not indexed, ids positional, count recorded).
- The first answer-run attempt used a CPU-built CLI for the doctor step;
  `doctor gpu` failed for the hybrid arm and the failed setup left a store
  root that blocked resume. Rebuilt the CLI with `embed-cuda`, removed the
  root, resumed; one FTS answer was paid twice (about two cents).
- A plain `doc` write under `use_default_embedder=True` embeds nothing
  until a `searchable` projection with `vector=True` is declared. There is
  no production surface to enrol a node kind for vectors otherwise; the
  eval code's `_configure_vector_kind_for_test` is test-hooks only.
  Recorded as a potential below and for the ledger at landing.
