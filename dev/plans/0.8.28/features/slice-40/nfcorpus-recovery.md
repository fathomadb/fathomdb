---
title: FathomDB 0.8.28 Slice 40 — NFCorpus recovery measurement and durable-approach experiments
status: MEASURED (orchestrator, 2026-10-16) at slice head 1d0031bc9
target_release: 0.8.28
observed_on: 2026-10-16
---

# Slice 40 NFCorpus recovery measurement

Rerun of COMP-01 (BEIR NFCorpus, 3,633 documents, 323 queries, nDCG@10)
with the committed COMP-01 harness (`experiments/comp_fts.py` at
`626c60584` on `llm/comp-01`), a dedicated venv (LanceDB 0.40.0,
pyarrow 26.0.0, Python 3.12), and wheels built with `maturin build --release`
from this worktree: the pre-fix baseline at `6360cd61a` and the fix at
`1d0031bc9`. Same inputs as the original run (manifest SHA-256 values
unchanged). Four fresh databases per arm, `AB/BA/BA/AB`; ordered results were
byte-identical across repetitions on both arms.

## 1. Recovery

| Arm | nDCG@10 | MRR@10 | Recall@10 | Empty | p50 ms | p95 ms | p99 ms |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| FathomDB before Slice 40 | 0.3087 | 0.4943 | 0.1456 | 16 | 1.14 | 5.13 | 8.24 |
| FathomDB with Slice 40 | **0.3260** | **0.5292** | **0.1562** | 15 | 1.09 | 4.88 | 8.41 |
| LanceDB 0.40.0 | 0.3265 | 0.5283 | 0.1529 | 15 | 1.50 | 1.72 | 1.86 |

- The forecast was 0.3260 (raw FTS5 with the corrected translation); the
  engine now scores 0.3260.
- Paired per-query delta, fixed minus baseline: +0.0173, bootstrap 95% CI
  [+0.0098, +0.0251]; 61 queries improve, 19 worsen, 243 tie.
- Paired delta, fixed FathomDB minus LanceDB: −0.0005, 95% CI
  [−0.0057, +0.0043]; 52 wins, 55 losses, 216 ties. Parity. FathomDB is
  ahead on MRR@10 and Recall@10.
- Duplicate-body documents in top-10 lists across the 323 queries: 164
  before, 38 after, 65 for LanceDB.
- Latency is unchanged at the median; the long-query tail remains the
  open latency item (follow-up in status.md).

## 2. Durable-approach experiments (raw FTS5 simulation, not engine features)

Measured with the same inputs on a plain SQLite FTS5 index using the
Slice 40 translation, body-dedupe first, as baseline (0.3260, equal to the
engine). Paired bootstrap 95% CIs over 323 queries. These simulate the
proposed mechanisms; none is implemented in the engine.

| Hypothesis | Variant | nDCG@10 | Delta | 95% CI |
| --- | --- | ---: | ---: | --- |
| Fielded BM25 (title / text columns, FTS5 column weights) | title 2.0 | 0.3279 | +0.0019 | [−0.0009, +0.0046] |
| | title 3.0 | 0.3281 | +0.0022 | [−0.0016, +0.0059] |
| | title 5.0 | 0.3273 | +0.0014 | [−0.0034, +0.0060] |
| Phrase arm (adjacent-term phrases) fused by RRF | weight 1 | 0.3273 | +0.0014 | [−0.0063, +0.0096] |
| | weight 3 | 0.3206 | −0.0054 | [−0.0150, +0.0051] |
| NEAR(…, 5) arm fused by RRF | weight 1 | 0.3242 | −0.0018 | [−0.0091, +0.0065] |
| All-terms AND arm fused by RRF | weight 1 | 0.3258 | −0.0002 | [−0.0017, +0.0012] |
| Near-duplicate collapse (5-word shingles, ≥50% overlap) | 13 pairs found | 0.3260 | 0.0000 | — |

Verdict: on NFCorpus none of the three hypotheses is validated. The title
boost trends positive at weights 2–3 but the interval includes zero; phrase
and proximity arms are null or negative; the corpus has 13 near-duplicate
pairs and collapsing them changes no top-10 list. Exact-duplicate handling
is already delivered by the Slice 40 fusion change. Any of these would need
a corpus with stronger structure (declared titles, repeated agent-memory
content) to show a measurable effect.

## 3. Artifacts

Scratch, not tracked: `comp01-baseline/`, `comp01-fixed/` (manifest, eight
cells, report.json), `hyp234.py`, `wheels-prewarm/`, `wheels-fixed/` under
the orchestrator session scratchpad.
