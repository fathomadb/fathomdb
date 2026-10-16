# COMP-03 Unit G result: evidence policy, recency, chronological order, and the best mix

Run 2026-10-16 for FathomDB 0.8.28 Slice 40 Unit G. It follows the
[Unit D result](2026-10-16-comp-03-unit-d-result.md). Engine
`5b09afd44112251ee634e034edf0368bbdce684d` for every run; harness at
`fcef83b84` on `slice/0.8.28-40-comp03`. Published LongMemEval-S results for
context:
[LongMemEval published-results catalog](2026-10-16-longmemeval-published-results-catalog.md).

## Configs, verdicts, receipts

All under `experiments/`. The configs and verdicts are in
`configs/comp-03/`, and each receipt is in `runs/<run id>/`.

| Run | Config | Verdict | Receipt |
| --- | --- | --- | --- |
| v3 | `hermes-answer.v3.json` | `recency-verdict.v3.json` | `comp-03-hermes-answer-20261016T1610Z-33d78241` |
| v3rep | `hermes-answer.v3.json` | none (noise replicate) | `comp-03-hermes-answer-20261016T1732Z-33d78241` |
| v4a | `hermes-answer.v4a.json` | `recency-verdict.v4a.json` | `comp-03-hermes-answer-20261016T1609Z-22869ddc` |
| v4b | `hermes-answer.v4b.json` | `recency-verdict.v4b.json` | `comp-03-hermes-answer-20261016T1609Z-1c856c71` |
| v4c | `hermes-answer.v4c.json` | `recency-verdict.v4c.json` | `comp-03-hermes-answer-20261016T1732Z-50579949` |
| mix | `hermes-answer.mix.json` | none | `comp-03-hermes-answer-20261016T1747Z-4e56afa9` |

- The evidence policy comes from `evidence-policy-verdict.v1.json`
  (4000/30000/on).
- `hermes-answer.mix.json` is byte-identical to the file the mix ran from.
  That file sat at a scratch path, so the receipt's `config.path` points
  outside the tree. The config digest matches the committed file. Its
  `decision_rule.sources` digests match the committed verdict files.
- The mix evidence equals v4c's: 4000/30000 with session expansion,
  chronological order on, and no recency sentence. The mix run is therefore
  an independent replicate of v4c.
- The v4c and v3rep runs were resumed after credits ran out. Their receipts
  above are the final, complete ones.

## Answer accuracy (correct of 60)

| Run | FTS | Hybrid | LanceDB | Cost |
| --- | --- | --- | --- | --- |
| v1 (top-5, 20k cap) | 42 | – | 42 | |
| v2 | 49 | 48 | 46 | |
| v3 | 51 | 50 | 50 | $3.37 |
| v3rep | 50 | 49 | 49 | $3.36 |
| v4a | 49 | 52 | 50 | |
| v4b | 49 | 50 | 53 | |
| v4c | 53 | 50 | 54 | $3.34 |
| mix | 53 | 52 | 54 | $3.34 |

**Noise:**

- v3 against v3rep, identical config: 3 discordant per arm (9 of 180).
- v4c against the mix, identical evidence: 2, 4, and 2 discordant
  (FTS, hybrid, LanceDB).

## Factor verdicts (arm of record: `fathomdb-fts`)

- **v3, recency rule:** reject. The rule is report-only.
- **v4a:** reject.
- **v4b:** adopt.
- **v4c:** adopt. By type: knowledge-update 8 to 10, temporal 9 to 10,
  multi-session 6 to 5, preference 9 to 8.
- **Mix:** policy 4000/30000/on plus chronological order, no sentence.

## Comparisons

- **Pooled two samples, chosen config against v3 config (of 120):**
  FTS 106 against 101, hybrid 102 against 99, LanceDB 108 against 99.
- **Mix against v2, paired** (seed 303 bootstrap, 10k resamples):

  | Arm | Net | Wins/losses | 95% CI |
  | --- | --- | --- | --- |
  | FTS | +4 | 5/1 | [0.000, +0.150] |
  | Hybrid | +4 | 6/2 | [−0.017, +0.167] |
  | LanceDB | +8 | 8/0 | [+0.050, +0.217] |

- **Mix against v1, FTS:** +11 (12/1), CI [+0.083, +0.300].
- **Mix, store against store:**
  - FathomDB FTS against LanceDB: −1 (0/1), CI [−0.05, 0.00].
  - Hybrid against LanceDB: −2 (1/3), CI [−0.10, +0.033].

## Reading

- **The gain is harness-side.** The evidence policy and chronological order
  belong to the harness, and they help every arm, LanceDB most. Answer
  accuracy across stores stays at parity within noise.
- **Per type:** the gain is in knowledge-update and temporal questions.
  Multi-session stays at 5 to 6 of 10 for every arm. That ceiling is
  aggregation, not retrieval.
- **Hybrid:** its retrieval gain on the 470-question sweep (Recall@5 +0.009,
  preference +0.167) does not show in answers at n=60.

## Final-head re-runs

All at engine `5b09afd44`, recorded with `--engine-commit`:

| Run | Receipt |
| --- | --- |
| COMP-01 FTS | `comp-01-comp01-3633-20261016T1755Z-b9aa1214` |
| COMP-01 hybrid | `comp-01-comp01-hybrid-3633-20261016T1755Z-c1693bec` (config `configs/comp-01/nfcorpus-hybrid.v1.json`) |
| COMP-02a 10k | `comp-02a-comp02a-10000-20261016T1756Z-09ac6bd7` |
| COMP-02a 17k | `comp-02a-comp02a-17272-20261016T1756Z-eede35ef` |
| COMP-02b 10k | `comp-02b-comp02b-10000-20261016T1757Z-d91933b9` |
| 10k latency | `comp-02b-latency-20261016T1700Z-4f96fad0` |
| COMP-03 sweep | `comp-03-retrieval-sweep-20261016T1725Z-17773486` |

- **Classification:** every receipt above, and every Unit G answer receipt,
  carries a validated measurement classification.
- **Baseline:** `configs/baselines/slice-40-baseline.v1.json` now takes its
  expected values from these runs. `comp03-answer-v2` stays provisional.
