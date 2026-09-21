# Competitive benchmark results and references

**Updated:** 2026-09-21

**Scope:** captured FathomDB comparisons, required retrieval baselines, and
candidate native-system comparisons

This file is separate from release gauntlet results because most competitive
measurements use different corpora, answerers, judges, and timing boundaries.
It records what has actually been measured without turning historical,
incomplete, or differently scoped work into a current product claim.

## Status vocabulary

- **Complete:** both registered arms completed under the same comparison
  contract.
- **Complete historical:** useful retained evidence, but not a current-release
  measurement.
- **Incomplete:** execution stopped or failed its completeness rule; report
  only as diagnostic evidence.
- **Blocked/not run:** no competitive score exists.
- **Candidate:** a useful comparison that still needs a pinned native system,
  matched input, and runnable contract.

## Captured answer and task quality

These are answer- or task-quality results. They are not retrieval recall and
they are intentionally separate from latency.

### Native Mem0 OSS on LOCOMO

The current complete comparison used all 1,540 eligible LOCOMO category 1–4
questions, a top-10 cutoff, and identical evidence-aware `gpt-4o-mini`
answerer and judge settings. The native harness was the patched commit
`41d1e633c78dd9102f466b0255971a664f833fd3`, based on upstream
`mem0ai/memory-benchmarks@4b61c5d31b9c668a12b4f5e78064248a02c82d2b`.
Mem0 used `gpt-4o-mini` extraction and `text-embedding-3-small` embeddings.
The FathomDB A0 arm was built from source commit
`cb244cb032477c00d4c0a254a40e6bfbba1868bc`.

| LOCOMO category | Questions | Mem0 OSS | FathomDB A0 | FathomDB minus Mem0 |
| --- | ---: | ---: | ---: | ---: |
| Overall | 1,540 | 67.21% | 75.19% | +7.99 pp |
| Multi-hop | 282 | 82.62% | 74.47% | -8.16 pp |
| Temporal | 321 | 16.20% | 52.02% | +35.83 pp |
| Open-domain | 96 | 77.08% | 83.33% | +6.25 pp |
| Single-hop | 841 | 80.38% | 83.35% | +2.97 pp |

The one-sided 95% paired-bootstrap lower bound for the overall difference was
`+5.78 pp`, using 10,000 question-level resamples and seed `20260814`.
FathomDB passed the registered overall near-parity rule, but Mem0 retained a
material multi-hop advantage. This result supports neither class-wide
dominance nor a claim about a later Mem0 release.

Sources:
[MEMORY-01 result](../2026-08-24-memory-01-result.md),
[execution controls](../2026-08-24-memory-01-execution-controls.md), and
[paired receipt](../../../experiments/runs/fathomdb-vs-mem0-locomo-comparison-20260824T2140Z-01e702be/record.json).

### Older Mem0 experiment

The 0.8.3-era experiment is retained because it used an older local Mem0 2.x
adapter and motivated the reranking work, but it is not a valid second point
on a version trend. The exact installed Mem0 package version was not retained,
the tuned arm completed only 354 of 606 cells, and the run stopped at 58.42%
completeness because of a provider usage limit.

| Scope | Mem0 OSS | FathomDB baseline | FathomDB tuned CE arm | Interpretation |
| --- | ---: | ---: | ---: | --- |
| 354 answered cells | 0.323 | 0.137 | 0.525 | Incomplete subset; not citable as parity or surpass |
| Estimated full 606 | 0.323 | 0.137 | about 0.33–0.35 | Diagnostic estimate only; marginal parity direction |

The answered subset overstates the tuned arm because missing cells were
disproportionately retrieval-failure cases. Do not compare these values
directly with the later 1,540-question native LOCOMO result.

Sources:
[0.8.3 resolution verdict](../../plans/runs/0.8.3-resolution-verdict.json) and
[experiment ledger](../../experiments-ledger.md).

### Microsoft GraphRAG 3.1.0 on AP-News BenchmarkQED

The strongest historical 0.8.4 comparison used 200 AP-News documents and 200
AutoQ questions against full-strength GraphRAG 3.1.0 at community level 1.
All synthesis arms used `gemini-3.5-flash` with reasoning disabled; the
cross-family judge was `claude-haiku`, with five order-swapped repetitions.
Win rate above 0.5 favors FathomDB.

| FathomDB arm versus GraphRAG | Comprehensiveness | Diversity | Empowerment | Interpretation |
| --- | ---: | ---: | ---: | --- |
| C: exhaustive map-reduce QFS | 0.723 (0.663–0.780) | 0.614 (0.563–0.666) | 0.719 (0.667–0.773) | Directional FathomDB win on all three; expensive read-everything arm |
| D2: depth-1 coverage index | 0.413 (0.348–0.473) | 0.446 (0.392–0.502) | 0.599 (0.544–0.653) | Loses coverage-oriented metrics; empowerment win is length-confounded |

The frozen decision rule remained `NOT_REACHED` because the
question-clustered minimum detectable effect was slightly above its registered
0.05 boundary and the 200-question corpus was exhausted. The substantive
result is split: exhaustive map-reduce beat GraphRAG, while the intended cheap
coverage path did not.

Source: [0.8.4 gating result](../../plans/runs/0.8.4-gating-rerun-RESULT.md).

A later native first-run comparison used 15 documents, eight global
questions, five order-swapped repetitions, `deepseek-v4-pro` for both answer
arms, and an independent `claude-haiku` judge. This is a separate protocol and
must not be pooled with the 200-document result.

| Metric | FathomDB win rate | Question-clustered 95% interval |
| --- | ---: | ---: |
| Comprehensiveness | 0.3750 | 0.0000–0.7500 |
| Diversity | 0.3750 | 0.0000–0.7500 |
| Empowerment | 0.3875 | 0.0375–0.7500 |
| Directness | 0.9688 | 0.9062–1.0000 |

This later run again found GraphRAG directionally stronger on global coverage
and FathomDB substantially more direct. The small question count gives wide
intervals and does not support a general winner claim.

Source: [GLOBAL-01 first-run result](../2026-08-29-global-01-first-run-result.md).

## Captured retrieval baselines

These rows measure retrieval rather than generated-answer quality. Naive RAG
and BM25 are required floors, not competitive memory systems.

The powered 0.8.3 LME-plus-LOCOMO retrieval comparison reported strict
Recall@10 as follows:

| Class | Questions | FathomDB FTS | Naive BM25 | FathomDB minus BM25 |
| --- | ---: | ---: | ---: | ---: |
| Factoid | 997 | 0.919 | 0.930 | -0.011 |
| Knowledge update | 150 | 0.553 | 0.720 | -0.167 |
| Multi-session | 419 | 0.415 | 0.408 | +0.007 |
| Temporal | 471 | 0.726 | 0.781 | -0.055 |

The FathomDB arm was FTS-only. It tied BM25 directionally on factoid and
multi-session, trailed it on temporal and knowledge-update, and does not
establish a graph-system comparison.

Source: [0.8.3 resolution report](../../plans/runs/0.8.3-report.md).

### FathomDB 0.8.26 on MuSiQue historical-300

GRAPH-RETRIEVAL-01 now provides the FathomDB side of a future HippoRAG-2-style
MuSiQue comparison. A CUDA-only qualification reproduced all 300 retained
historical top-10 rankings and froze top-20 fused BM25+dense seeds. The scored
stage then compared those seeds with depth-1 and depth-2 native graph
expansion.

| Retrieval metric | Fused control | Native depth 1 | Native depth 2 |
| --- | ---: | ---: | ---: |
| Complete support@5 | 0.443333 | 0.443333 | 0.443333 |
| Complete support@10 | 0.610000 | 0.610000 | 0.610000 |
| Complete support@20 | 0.916667 | 0.916667 | 0.916667 |
| Support recall@10 | 0.770000 | 0.770000 | 0.770000 |
| nDCG@10 | 0.675379 | 0.675379 | 0.675379 |
| First-support MRR | 0.756240 | 0.756240 | 0.756240 |

Both graph treatments had a `0.000000` candidate-change rate and `1.000000`
no-op rate. This is a completed FathomDB retrieval result, but not a completed
competitive comparison: it uses the retained historical-300 cohort, does not
yet reproduce the official HippoRAG-2 arm, and does not include answer EM/F1.
It therefore establishes neither parity nor superiority over HippoRAG-2.

Sources:
[0.8.26 graph result](GRAPH-BENCHMARKS-SMOKE-0.8.26.md),
[qualified seed manifest](results/graph-retrieval-01-seed-manifest.v1.json), and
[historical run receipt](results/graph-retrieval-01-historical-300-0.8.26.record.json).

## Timing observations

Timing is reported separately because the captured native Mem0 and FathomDB
measurements do not share one latency boundary.

| System and boundary | Captured timing | Interpretation |
| --- | --- | --- |
| Native Mem0 official-harness search | mean 5,081.34 ms over 1,540 questions | Includes the native service/harness path; only a mean was retained |
| FathomDB official-seam request | mean 690.98 ms over 1,540 questions | Includes facade/harness overhead and is not the engine timing |
| FathomDB facade query | p50 1.485 ms; p95 1.921 ms; p99 2.263 ms | Local facade interval |
| FathomDB engine query | p50 1.442 ms; p95 1.855 ms; p99 2.199 ms | Local engine-only interval |

These observations must not be converted into a cross-system speed ratio.
A future speed comparison needs matched client boundaries, warm-up, request
concurrency, result materialization, and percentile reporting on both arms.

Sources:
[native Mem0 receipt](../../../experiments/runs/mem0-oss-locomo-native-20260824T1325Z-9de95019/record.json)
and
[FathomDB arm receipt](../../../experiments/runs/fathomdb-locomo-official-seam-20260824T1309Z-3762b22a/record.json).

## Systems without a completed matched score

| Comparator | Intended benchmark | Current state | What is required before a score exists |
| --- | --- | --- | --- |
| Graphiti/Zep | LOCOMO under the same shared memory protocol as Mem0 | Not run | Pin an official native build and backend, preserve its temporal graph behavior, then run identical questions, cutoffs, answerer, and judge |
| HippoRAG-2 | Official MuSiQue reproduction; supporting-evidence retrieval plus answer EM/F1 | FathomDB historical-300 retrieval complete; native comparator not run | Reconcile the official corpus representation, reproduce the pinned native build, and run both arms under the matched official protocol |
| Newer Mem0 | Refresh of native MEMORY-01 | Candidate | Select and pin an actual newer upstream release; rerun both systems on the same frozen LOCOMO input rather than comparing with old output |
| Newer Microsoft GraphRAG | Refresh of GLOBAL-01 | Candidate | Pin the release and repeat the matched AP-News/AutoE contract; never compare scores from changed models or question sets as a version trend |

The acquired but unexecuted HippoRAG-2 source was pinned at
`c617143f01477243992a63b2e2151cc003dd3b21`. Historical Graphiti attempts were
blocked before native execution. FathomDB now has a MuSiQue retrieval number,
but neither competitor has a completed matched FathomDB comparison in this
repository.

Sources:
[readiness results](../2026-08-11-competitor-baseline-readiness-execution-results.md),
[comparison contract](../2026-08-11-competitor-comparison-plans.md), and
[program register](../README.md).

## Reporting rules for future additions

1. Pin the competitor version or commit, native harness, input hashes, model
   identities, and complete resolved configuration.
2. Reproduce the native comparator before adapting FathomDB to its seam.
3. Give both systems the same frozen documents, questions, cutoff, answerer,
   judge, and budget wherever the protocol permits.
4. Keep retrieval metrics, generated-answer metrics, and latency/resource
   metrics in separate tables.
5. Preserve incomplete, blocked, or unavailable cells as typed evidence; do
   not turn them into zeroes.
6. Treat results from changed corpora, prompts, judges, or timing boundaries as
   separate experiments, not a longitudinal version series.
