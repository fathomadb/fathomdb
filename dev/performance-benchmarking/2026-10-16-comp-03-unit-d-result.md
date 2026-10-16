# COMP-03 Unit D result — FathomDB FTS vs hybrid on LongMemEval-S, and a three-arm answer run

Run 2026-10-16 for FathomDB 0.8.28 Slice 40 Unit D, under the
[side plan](plans/2026-10-15-comp-side-plan.md). It follows the
[COMP-03 v1 result](2026-10-16-comp-03-hermes-answer-result.md).

- Retrieval sweep config:
  [`retrieval-sweep.v1.json`](../../experiments/configs/comp-03/retrieval-sweep.v1.json).
  Receipt (D3 sweep):
  `experiments/runs/comp-03-retrieval-sweep-20261016T1406Z-17773486/`.
- Answer config:
  [`hermes-answer.v2.json`](../../experiments/configs/comp-03/hermes-answer.v2.json).
  Receipt: `experiments/runs/comp-03-hermes-answer-20261016T1422Z-c807ed90/`.
- Code: harness at `0edd7a078` on `slice/0.8.28-40-comp03`. Engines: the
  Slice 40 wheel before D3 (`a3830681d`) and with D3 (`5b09afd44`, the
  stop-list extension). Both wheels report version `0.8.27`, so the version
  field in the receipts does not tell them apart; the attempt roots do.

## Method

- **Questions:**
  - Retrieval sweep: all 500 LongMemEval-S questions. The 30 abstention
    questions are retrieved but not scored, so the scored n is 470.
  - Answer run: the same 60 seeded questions as v1.
- **Stores:** one fresh store per question per arm, built from rounds by
  `comp03_answer.convert_question`, as in v1. Gold comes only from
  `gold_for`.
- **Arms:**

  | Arm | Store | Query |
  | --- | --- | --- |
  | `fathomdb-fts` | plain round bodies, no embedder, no projection (the v1 shape) | `Engine.search_text_only` |
  | `fathomdb-hybrid` | the same plain bodies; `Engine.open(use_default_embedder=True)`; `ProjectionSpec(name="dense", roles={SEARCHABLE}, vector=True)` declared before writes | `Engine.search`: RRF of vector and text, 3:1 text, no rerank, no graph arm |
  | `lancedb-fts` | LanceDB 0.40 native FTS, library defaults | `table.search(query_type="fts")` |

  - The two FathomDB arms index identical text. Their pair is a pure
    vector-arm ablation.
  - **The dense switch.** A plain `doc` write under
    `use_default_embedder=True` embeds nothing: no vector kind is enrolled,
    and `search` returns text hits only. Declaring a searchable
    `vector=True` projection is the public switch that enrols the node kind
    for dense projection.
  - **What is embedded.** The projection worker embeds the full node body
    (`canonical_nodes.body`), not a projected attribute. That is why a
    projection named `dense`, absent from every body, still gives one
    vector row per document.
  - **Coverage check.** Every hybrid store is checked on the closed
    database before it is queried: distinct embedded `doc` cursors must
    equal the documents written. Every question passed.
- **Blank rounds:**
  - No arm indexes a whitespace-only round. Round IDs stay positional, so
    gold IDs and every other round are unchanged.
  - One question in the 500 has blank rounds: `gpt4_7fce9456`
    (multi-session), rounds `s040-r002` and `s040-r005`; neither is gold.
  - In both sweeps, the LanceDB record for that question predates the
    filter (`0edd7a078`): LanceDB indexed the two blank strings (253
    documents against FathomDB's 251), and the summaries count it as 0
    skipped. Since `0edd7a078` all arms are paired on the filtered set.
- **Device:**
  - `FATHOMDB_EMBED_DEVICE` named a CUDA device for each sweep. Each
    sweep's `device.json` records requested `cuda:0` and effective `cuda`.
    The D3 sweep ran on the host's second GPU.
  - A CUDA request that resolved to CPU would have stopped the sweep.
- **Metrics:**
  - Session Recall@k (any): a round of any answer session is in the top k.
  - Session Recall-all@k: at least one round of every answer session is in
    the top k.
  - Session MRR@10.
  - Turn Recall@k: a `has_answer` round is in the top k.
  - Paired bootstrap 95% CIs on hybrid − fts per question: 10,000
    resamples, seed 404.
- **Decision rule** (stated in the config before the sweep): adopt hybrid
  for the answer run iff
  - (a) preference session Recall@5 delta CI lower bound > 0,
  - (b) no other type's delta CI upper bound < 0, and
  - (c) overall delta ≥ 0.
- **Answer run:**
  - Same answerer, judge, prompts and caps as v1: `openai/gpt-5.4` for
    both, a 4,000-character per-round cap and a 20,000-character total cap.
  - `top_k` 10 for every arm (v1: 5).
  - Arm order rotates by question.
  - The hybrid arm ran because the D3 sweep's verdict was `adopt`; the
    receipt binds that sweep summary by digest.

## Retrieval sweep (D3 engine, 470 scored questions)

Session Recall@5 and @10 (any) per type. Δ is hybrid − fts with its 95% CI.

| Type | n | fts @5 | hybrid @5 | LanceDB @5 | Δ@5 [95% CI] | fts @10 | hybrid @10 | LanceDB @10 | Δ@10 [95% CI] |
| --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | --- |
| knowledge-update | 72 | 1.000 | 0.986 | 1.000 | −0.014 [−0.042, 0.000] | 1.000 | 1.000 | 1.000 | 0.000 [0.000, 0.000] |
| multi-session | 121 | 0.975 | 0.975 | 0.967 | 0.000 [−0.033, +0.033] | 0.992 | 0.992 | 0.992 | 0.000 [0.000, 0.000] |
| single-session-assistant | 56 | 1.000 | 1.000 | 1.000 | 0.000 [0.000, 0.000] | 1.000 | 1.000 | 1.000 | 0.000 [0.000, 0.000] |
| single-session-preference | 30 | 0.733 | 0.900 | 0.733 | **+0.167 [+0.033, +0.300]** | 0.900 | 0.933 | 0.933 | +0.033 [−0.067, +0.133] |
| single-session-user | 64 | 0.984 | 0.969 | 0.969 | −0.016 [−0.047, 0.000] | 1.000 | 1.000 | 1.000 | 0.000 [0.000, 0.000] |
| temporal-reasoning | 127 | 0.913 | 0.921 | 0.906 | +0.008 [−0.031, +0.047] | 0.976 | 0.969 | 0.961 | −0.008 [−0.024, 0.000] |
| **overall** | 470 | 0.951 | 0.960 | 0.945 | +0.009 [−0.009, +0.026] | 0.985 | 0.985 | 0.983 | 0.000 [−0.009, +0.009] |

Other overall metrics:

| Metric | fts | hybrid | LanceDB | Δ hybrid − fts [95% CI] |
| --- | ---: | ---: | ---: | --- |
| Session Recall-all@5 | 0.781 | 0.781 | 0.768 | 0.000 [−0.026, +0.026] |
| Session Recall-all@10 | 0.877 | 0.900 | 0.879 | **+0.023 [+0.004, +0.045]** |
| Session MRR@10 | 0.911 | 0.929 | 0.902 | **+0.018 [+0.001, +0.035]** |
| Turn Recall@5 | 0.872 | 0.887 | 0.870 | +0.015 [−0.011, +0.040] |
| Turn Recall@10 | 0.943 | 0.955 | 0.940 | +0.013 [−0.004, +0.030] |

- **Verdict: adopt.**
  - (a) holds: the preference lower bound is +0.033.
  - (b) holds: the lowest upper bounds are 0.000 (knowledge-update and
    single-session-user), not below zero.
  - (c) holds: the overall delta is +0.009.
- **Recall-all, Recall-any and MRR.** Recall-all@10 separates hybrid from
  fts on multi-session questions (0.810 vs 0.744), where Recall-any@10 is
  0.992 for both. MRR@10 rises most on preference: 0.794 vs 0.611.
- **Pre-D3 sweep.** Same arms on the engine before D3, verdict also adopt
  (a, b and c true):
  - preference Δ@5 +0.133 [+0.033, +0.267];
  - overall Δ@5 +0.009 [−0.009, +0.026];
  - MRR@10 Δ +0.024 [+0.006, +0.041].

  It has no receipt; its summary is in the attempt root listed under
  Reproduce.

## Answer run v2 (D3 engine, k = 10, 60 questions)

Correct answers out of ten per type, with v1 (k = 5) for context. v1 is a
different run at a different k; its columns are not paired with v2.

| Type | fts v2 | hybrid v2 | LanceDB v2 | FathomDB v1 | LanceDB v1 |
| --- | ---: | ---: | ---: | ---: | ---: |
| knowledge-update | 8 | 8 | 8 | 6 | 6 |
| multi-session | 4 | 5 | 4 | 5 | 4 |
| single-session-assistant | 10 | 10 | 10 | 10 | 10 |
| single-session-preference | 8 | 7 | 6 | 3 | 4 |
| single-session-user | 9 | 9 | 9 | 9 | 9 |
| temporal-reasoning | 10 | 9 | 9 | 9 | 9 |
| **total** | **49 (0.817)** | **48 (0.800)** | **46 (0.767)** | 42 (0.700) | 42 (0.700) |

Paired differences (bootstrap 95%, seed 0):

| Pair | Difference | 95% CI | Discordant (first only / second only) |
| --- | ---: | --- | --- |
| fts − LanceDB | +0.050 | [0.000, +0.117] | 3 / 0 |
| hybrid − LanceDB | +0.033 | [−0.033, +0.100] | 3 / 1 |
| fts − hybrid | +0.017 | [−0.050, +0.083] | 3 / 2 |

The receipt stores the last pair as `fathomdb-fts_vs_fathomdb-hybrid`, so
its CI is for fts − hybrid. Written as hybrid − fts, it is −0.017, 95%
[−0.083, +0.050].

Retrieval on the 56 non-abstention answer-run questions, session level at
k = 10:

| Arm | Recall@10 | MRR@10 |
| --- | ---: | ---: |
| fts | 0.964 | 0.857 |
| hybrid | 0.946 | 0.871 |
| LanceDB | 0.964 | 0.823 |

## Interpretation

- **FathomDB FTS beats LanceDB FTS on answers, at the edge of the
  interval.** The difference is +0.050 with a CI from 0.000 to +0.117, and
  all 3 discordant questions favour FathomDB. This is the first COMP-03 run
  where the paired interval does not straddle zero. It touches zero, so it
  is a lower-bound result, not a margin.
- **Hybrid is not distinguishable from FTS on answers at n = 60.**
  - Hybrid lifts preference retrieval clearly in the 470-question sweep:
    Recall@5 +0.167, with a CI above zero.
  - Answers do not follow: fts − hybrid is +0.017 [−0.050, +0.083], and
    preference answers are 8 for fts and 7 for hybrid.
  - At k = 10 both FathomDB arms already reach most preference rounds
    (Recall@10 0.900 vs 0.933), so the k = 5 gain matters less here.
- **The k change is the largest single mover.** Both FTS arms rose from 42
  (v1, k = 5) to 49 and 46 (v2, k = 10), and preference went from 3 to 8
  for FathomDB. This comparison is not paired: different runs, same
  questions, same models and same caps.
- **Effective depth is cap-limited.** At k = 10 the median rendered
  evidence is 20,326 characters for FathomDB and 20,347 for LanceDB.
  - The 20,000-character total cap on round text binds; the rendered text
    also carries date headers.
  - The cap was reached for 46 of 60 questions on fts, 45 on hybrid and 47
    on LanceDB.
  - The median question shows 9 of its 10 rounds, 8 of them untruncated;
    the minimum is 6.
  - So "k = 10" means "top 10, cut at 20,000 characters". v1's median
    evidence was 11,890 characters.

## NFCorpus (COMP-01 inputs, 323 queries)

| Run | Arm | nDCG@10 | Recall@10 | MRR@10 | Empty results |
| --- | --- | ---: | ---: | ---: | ---: |
| `comp01` campaign, D3 wheel | FathomDB `search_text_only` | 0.3265 | 0.156 | 0.531 | 15 |
| `comp01` campaign, D3 wheel | LanceDB FTS | 0.3265 | 0.153 | 0.528 | 15 |
| `comp01-hybrid` campaign, pre-D3 wheel | FathomDB `search_text_only` | 0.326 | 0.156 | 0.529 | 15 |
| `comp01-hybrid` campaign, pre-D3 wheel | FathomDB hybrid | **0.355** | 0.171 | 0.566 | **0** |

- **D3 vs Defect A/B** (orchestrator's paired analysis): FathomDB 0.32655 vs
  the recorded 0.3260, paired +0.0006, CI [−0.0000, +0.0016], 2 wins,
  1 loss, 320 ties. D3 does not cost NFCorpus quality. FathomDB FTS now
  equals LanceDB FTS (0.3265).
- **Hybrid vs FTS:** paired +0.0292, CI [+0.0191, +0.0404], 110 wins,
  49 losses, 164 ties. Empty results drop from 15 to 0. Hybrid is the only
  arm that beats LanceDB on NFCorpus.
- **Measured under contention, not comparable latencies:** the hybrid
  arm's query p50 (22 ms) and its build time (58 s for 3,633 documents)
  were measured while other GPU and CPU work shared the host.

## Costs and latency

| Item | fts | hybrid | LanceDB |
| --- | ---: | ---: | ---: |
| Sweep query, warm p50 / p95 (ms, D3) | 1.28 / 1.61 | 6.55 / 9.05 | 1.65 / 1.95 |
| Sweep store build p50 (s, D3) | 0.048 | 2.65 (embedding 2.48) | 0.039 |
| Answer-run retrieval, warm p50 (ms) | 1.29 | 6.53 | 1.65 |
| Answer call p50 (s) | 1.41 | 1.36 | 1.36 |
| Answer-run model cost (USD) | 0.78 | 0.78 | 0.78 |

- **Hybrid timings.** Hybrid query latency includes embedding the
  question. Its build time is the GPU embedding of about 250 rounds per
  store.
- **Totals.** The answer run cost $2.34; the sweeps call no models.
- **10k TC-5 latency, D3 vs pre-D3** (orchestrator, quiet host, recorded on
  the slice branch), p50:
  - body search 13.4 → 9.4 ms;
  - projected unfiltered 5.0 → 3.0 ms;
  - cohort c10 14.6 → 11.5 ms;
  - cohort c33 12.7 → 9.4 ms;
  - cohort c01 37.1 → 36.5 ms.

  p95 is lower on all multi-term cells.

## Incidents

- **Blank rounds.** The first full sweep stopped at question 457 of 500
  with `WriteValidationError`: `Engine.write` rejects whitespace-only
  bodies, and `gpt4_7fce9456` has two such rounds. The fix (`0edd7a078`)
  skips blank rounds for every arm and records `blank_rounds_skipped`. The
  456 finished questions resumed unchanged.
- **CPU CLI for `doctor gpu`.** One answer-run attempt used a CPU-built
  `fathomdb` CLI with the CUDA wheel, so the hybrid arm's `doctor gpu`
  evidence did not match the wheel that embedded. The recorded run used the
  D3 CUDA CLI.
  - Since `745c8ecba`, a hybrid store refuses a CLI whose `doctor gpu`
    `cuda_compiled` differs from the wheel's embedder resolution.
  - Every store records `fathomdb_bin` and `fathomdb_bin_sha256`.
  - A failed setup no longer leaves a store root behind for the next
    resume to trip on.

## Not covered

- Cross-encoder rerank and the graph arm.
- Evidence depth beyond the 20,000-character cap, or k beyond 10.
- The D3 stop-list effect on answers on its own: the answer run used the
  D3 engine for both FathomDB arms, and no pre-D3 answer run at k = 10
  exists.
- A paired k = 5 vs k = 10 answer comparison.
- Answer-level statistical power for hybrid vs fts beyond n = 60.

## Reproduce

```bash
FATHOMDB_EMBED_DEVICE=cuda:0 python -m experiments.comp03_retrieval run \
  experiments/configs/comp-03/retrieval-sweep.v1.json \
  --dataset <longmemeval_s_cleaned.json> --attempt-root <new dir>
FATHOMDB_EMBED_DEVICE=cuda:0 python -m experiments.comp03_answer run \
  experiments/configs/comp-03/hermes-answer.v2.json \
  --dataset <longmemeval_s_cleaned.json> --attempt-root <new dir> \
  --fathomdb-bin <CUDA fathomdb CLI matching the wheel> \
  --sweep-summary <sweep attempt>/summary.json
python -m experiments.comp_fts run --campaign comp01-hybrid \
  --inputs <nfcorpus inputs> --out <new dir>
```

Raw attempts (outside Git):

- `~/.cache/fathomdb-comp/comp03-sweep-pre-d3/`
- `~/.cache/fathomdb-comp/comp03-sweep-d3/`
- `~/.cache/fathomdb-comp/comp03-run2/`
- `~/.cache/fathomdb-comp/comp01-d3/`
- `~/.cache/fathomdb-comp/comp01-hybrid/`

The two receipts bind their summaries by digest and carry measurement
classifications (`experiments/comp03_classification.py`; plans under
`experiments/configs/comp-03/`).

- The pre-D3 sweep and the two NFCorpus runs have no receipts.
- The repository classification check still stops at the v1 COMP-03 receipt
  (`comp-03-hermes-answer-20261016T1239Z-2e410a4f`), which was committed
  without a classification and is left as found. So are the COMP-01 and
  COMP-02 receipts behind it.
