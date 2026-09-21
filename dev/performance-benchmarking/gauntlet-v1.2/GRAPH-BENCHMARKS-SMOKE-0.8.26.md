# Graph benchmark implementation smoke — FathomDB 0.8.26

**Disposition:** graph evidence, traversal, and retrieval plumbing passed. The
CUDA-only seed qualification reproduced all 300 historical top-10 rankings,
and the historical 300-question native-expansion run completed without errors.
These are directional 0.8.26 observations, not a 0.8.25 versus 0.8.26
comparison.

The compact machine-readable receipt is
[`results/graph-smoke-0.8.26.json`](results/graph-smoke-0.8.26.json). Large,
reproducible SQLite databases, duplicate output trees, the temporary CUDA
build, and the temporary virtual environment were not retained.

## Fidelity and correctness

This section contains no timing measurements.

| Cell | Observation | Result |
| --- | --- | ---: |
| GRAPH-EVIDENCE-01 | Matrix rows | 24 |
| GRAPH-EVIDENCE-01 | Target revision exact | 1.000 |
| GRAPH-EVIDENCE-01 | Terminal edge exact | 1.000 |
| GRAPH-EVIDENCE-01 | Canonical source exact | 1.000 |
| GRAPH-EVIDENCE-01 | Typed refusal | 1.000 |
| GRAPH-EXPAND-01 | Target order exact | yes |
| GRAPH-EXPAND-01 | Origin exact | yes |
| GRAPH-EXPAND-01 | Work units exact | yes |
| GRAPH-EXPAND-01 | Work-bound all-or-nothing | yes |
| GRAPH-EXPAND-01 | Query seed exact | yes |

GRAPH-RETRIEVAL-01 exercised one synthetic question to prove native expansion,
evidence resolution, and deterministic candidate promotion. Its retrieval
scores are deliberately omitted because a one-question synthetic smoke is not
a meaningful quality result.

### GRAPH-RETRIEVAL-01 historical-300 retrieval quality

The qualified historical run used the retained MuSiQue-Ans 300-question
cohort. The fused BM25+dense top-20 ranking is the control. Depth-1 and depth-2
native expansion produced the same scores because neither treatment changed a
candidate list.

| Retrieval metric | Control | Depth 1 | Depth 2 |
| --- | ---: | ---: | ---: |
| Complete support@5 | 0.443333 | 0.443333 | 0.443333 |
| Complete support@10 | 0.610000 | 0.610000 | 0.610000 |
| Complete support@20 | 0.916667 | 0.916667 | 0.916667 |
| Support recall@5 | 0.658333 | 0.658333 | 0.658333 |
| Support recall@10 | 0.770000 | 0.770000 | 0.770000 |
| Support recall@20 | 0.916667 | 0.916667 | 0.916667 |
| nDCG@5 | 0.630227 | 0.630227 | 0.630227 |
| nDCG@10 | 0.675379 | 0.675379 | 0.675379 |
| nDCG@20 | 0.723144 | 0.723144 | 0.723144 |
| First-support MRR | 0.756240 | 0.756240 | 0.756240 |

Both expansion depths had a `0.000000` candidate-change rate, `0.000000`
useful-expansion rate, and `1.000000` no-op rate. The paired complete-support
deltas at 5, 10, and 20 were all `0.000000`, with 95% bootstrap intervals of
`[0.000000, 0.000000]`. This is a valid null result: the graph execution path
worked, but this graph projection and promotion policy supplied no new
retrieval evidence.

## Speed and resource observations

This section contains no retrieval-quality measurements.

| Cell | Measurement | Result |
| --- | --- | ---: |
| GRAPH-EVIDENCE-01 | Expand p50 / p95 / p99 | 3.373 / 3.373 / 3.373 ms |
| GRAPH-EVIDENCE-01 | Resolve p50 / p95 / p99 | 2.904 / 2.904 / 2.904 ms |
| GRAPH-EVIDENCE-01 | Full matrix | 118.484 ms |
| GRAPH-EXPAND-01 | Latency p50 / p95 / p99 | 9.204 / 9.885 / 10.178 ms |
| GRAPH-EXPAND-01 | Single-worker throughput | 107.95 operations/s |
| GRAPH-EXPAND-01 | Concurrency 2 / 4 / 8 | 192.32 / 346.56 / 452.29 operations/s |
| GRAPH-EXPAND-01 | Database size | 3,309,568 bytes |
| GRAPH-EXPAND-01 | Maximum RSS | 72,104 KiB |
| GRAPH-RETRIEVAL-01 | Native expansion p50 / p95 / p99 | 2.974 / 3.483 / 3.528 ms |
| GRAPH-RETRIEVAL-01 historical depth 1 | p50 / p95 / p99 | 2.749 / 10.152 / 21.587 ms |
| GRAPH-RETRIEVAL-01 historical depth 2 | p50 / p95 / p99 | 3.682 / 21.488 / 41.417 ms |

## GPU execution boundary

The three-cell smoke is not an encoding benchmark. Each cell creates explicit
documents, entities, and edges and opens FathomDB with the default embedder
disabled. GRAPH-RETRIEVAL-01's smoke uses an explicit seed, then measures native
graph expansion and evidence resolution. There is no neural encoding operation
for a GPU to accelerate.

The separate one-time MuSiQue seed qualifier is an encoding workload and did
use the GPU. It ran with forced CUDA on RTX 3090 UUID
`GPU-5f9cfc90-2be1-06a7-ce39-5a6d294b209b`; the active process was observed at
650 MiB VRAM and 44% GPU utilization. Forced CUDA and the recorded selected
UUID prevent silent CPU fallback.

The qualifier reproduced the historical CLS-pooled encoder rather than using
the current mean-pooled embedding path. It matched 300 of 300 retained top-10
rankings and emitted a frozen top-20 seed manifest. The manifest SHA-256 is
`862b304d07986498d82a512effcfa7f5baea07d65836f6d60e494abde6f3b6c5`;
its canonical rankings SHA-256 is
`05229d9643d97fbc80412fc9d34af8bcfc20d12888478c1698a25fab044c7b01`.

The historical native-expansion stage does not encode text. It consumes those
frozen seeds and measures graph traversal, evidence resolution, candidate
promotion, and retrieval quality, so running that stage on CPU is correct and
does not conceal an encoding fallback.

## Verification boundary

- Focused graph benchmark tests: 19 passed, including GPU-only historical
  encoder parity, duplicate-passage identity, and native-expansion coverage.
- All three graph cells passed through the top-level graph gauntlet smoke.
- Standalone and gauntlet receipt comparison passed before temporary artifacts
  were cleaned.
- The retained qualified seed manifest and compact historical run receipt are
  under [`results/`](results/); the 612 KiB reproducible observation trace and
  temporary CUDA build/environment were not retained.
- The run source was dirty at commit
  `b35212f678626e09ea7293e00ee337288ce74ff1`; these results are directional
  implementation evidence, not release evidence.
