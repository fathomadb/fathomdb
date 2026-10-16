---
title: FathomDB 0.8.28 Slice 40 — status
status: ACTIVE (Defects A–C and Units D–G landed and measured at engine 5b09afd44; Unit H added 2026-10-16, docs for review)
target_release: 0.8.28
observed_on: 2026-10-15
---

# Slice 40 status

Step log for [plan.md](plan.md). Commits are on
`slice/0.8.28-40-fts-ranking-defects` (base `574b09e78`). The output record
for phase 1 is `output.json` in this directory.

| Step | State (2026-10-15) |
| --- | --- |
| W1 docs | done; see output.json for the commit |
| W2 RED tests | done; 6 RED in `fathomdb-query`, 12 RED in `slice40_fts_ranking.rs`, guards green (test-plan § 5) |
| R1 | APPROVE-WITH-FIXES on `6360cd61a` (findings F1–F6; Q1 keep positional rank, Q2 follow-up, Q3 add short-token AC-038 rows) |
| W3 | done: F1 AC-038 short-token rows (RED), F2 possessive tests, F6 design wording; F3 and F4 apply at W4/W5; F5 lockfile restored |
| W4 | done at `13879d4af`: every Slice 40 test GREEN, no test edited; CHANGELOG `### Fixed` entries (F3) |
| W5 | engine suite (default features) exit 0, 474 passed; `fathomdb-query` exit 0; lint and lint-md exit 0; `agent-verify.sh` exit 1: Python suites failed on an unclean tree and passed on rerun; `test-check-board-currency` and `test-commission-manifest` fail on the 0.8.28 release-state files from the baseline (Slice 30 and 31 not LANDED, Slice 31 SHA not on the board); needs an orchestrator ruling (test-plan § 6) |
| R2 | not started |
| C-W1 Defect C docs | done 2026-10-16: requirements § 1.3, REQ40-C1..C6, AC40-27..32; design § 2b; test-plan § 3b |
| C-W2 Defect C RED tests | done: `slice40_projected_fts_stream.rs`, 2 RED (route and plan witness, fallback witness), 3 GREEN regression net (test-plan § 5.3) |
| C-R1 | APPROVE-WITH-FIXES on `04bf5e5e6` (Q4 keep `projected_` prefix, Q5 no CROSS JOIN, Q6 seam unchanged; F1–F6 applied) |
| C-W4 Defect C fix | done at `a3830681d`: stream statement, fallback, witnesses, CHANGELOG; 5/5 Defect C tests GREEN, no test edited |
| C-W5 | default engine suite 474 passed; F2 test list all green; lint and lint-md exit 0; `agent-verify.sh` exit 1 on `test-check-board-currency` and `test-commission-manifest` only (release-state files, unchanged from W5); test-plan § 5.4, § 6.2 |
| D-W1 Unit D docs | done 2026-10-16: requirements § 1.4, REQ40-D1..D6, AC40-33..48; design § 2c; test-plan § 3c, § 5.5 |
| D-W2 D3 RED tests | done: 5 RED in `fathomdb-query` (test-plan § 5.5) |
| D-W2 D1/D2 RED tests | done on `slice/0.8.28-40-comp03` (worktree `comp03-slice40`): 27 RED, pre-existing 9 pass, 2 skip (no `lancedb` in that `.venv`) |
| D-R1 | APPROVE-WITH-FIXES on `706bcab91` / `efa10e0c5` / `990f094ad`: Q-D1 delete the superseded test, Q-D2 plain-body hybrid and drop `fathomdb-fts-projected`, Q-D3 sync the AC-038 copy, Q-D4 install `lancedb==0.40.0`, Q-D5 `comp01-hybrid` accepted |
| D-W4 | done: harness `8ef511554` on `slice/0.8.28-40-comp03` (tests aligned at `34cacdf87`, `c08a25691`); D3 alone at `5b09afd44`, CHANGELOG `8d5d93cef`; all Unit D tests GREEN (test-plan § 5.6) |
| D-W5 | W-slice: engine default suite 474 passed, lint and lint-md exit 0, `agent-verify.sh` exit 1 on the two release-state suites only; W-harness: COMP tests 37 passed, `tests/experiments` 811 passed with 4 pre-existing failures, ruff clean, `agent-lint.sh` stops at the pre-existing COMP-03 v1 receipt classification (test-plan § 6.3) |
| D measurements (orchestrator) | not started: D1 sweep on cuda:0, NFCorpus with the D3 wheel plus `comp01-hybrid`, 10k TC-5 latency, D2 answer run |
| EFG-W1 docs | done 2026-10-16: requirements § 1.5, REQ40-E1..E4, F1..F5, G1..G5, AC40-49..66; design § 2d–2f; test-plan § 3d–3f, § 5.7, § 10 |
| EFG-W2 RED tests | done on `slice/0.8.28-40-comp03`: 31 RED (`test_sdk_parity_slice40.py` 9, `test_baseline_check.py` 8, `test_comp03_evidence.py` 14); none in this worktree (no doc checker pins verb text) |
| EFG-R1 | APPROVE on `ad2773e6a` / `9d864e39e` (Q-E1 update the three capability assertions; Q-E2 declared commit plus digests; Q-E3 `package_root`; Q-G1 pooled plus fathomdb-fts; Q-G2 470 questions; Q-F1 provisional pre-fill) |
| EF-W4/W5 | done: harness `bc4ca421b` (F), `d709db1a8` and `5e6ff1a34` (E); docs here; cross-SDK parity equal in every case and repetition (test-plan § 5.8) |
| G | review REJECT (F1–F10), revised; re-review APPROVE-WITH-FIXES (N1–N4, v4c) applied; W4 at harness `1419aae4f`: verdict 4000/30000/on (matches the replay), configs v3/v4a/v4b/v4c; W5 green except the four pre-existing failures (test-plan § 5.10, § 6.5); measured (orchestrator): v3 reject, v4a reject, v4b adopt, v4c adopt; mix (4000/30000/on plus chronological, no sentence) FTS 53, hybrid 52, LanceDB 54 of 60; harness records at `f59472d4c` |
| Final re-runs (engine `5b09afd44`) | COMP-01 13/13 PASS (nDCG@10 FTS 0.3265, hybrid 0.3555, LanceDB 0.3265); COMP-02a 10k FathomDB ready 1.44 s, p50 3.9, p95 12.6 ms vs SurrealDB 4.40 s/146/159; 17k 2.50/6.7/20.8 vs 7.74/252/272; COMP-02b 10k p50 unfiltered 2.9, c33 8.3, c10 9.3, c01 29.7 ms vs SurrealDB 63/19.7/4.1 (c01 short lists 6 per rep, pre-existing, explained in the COMP-02b note: no cohort document holds those query terms after stop words); tc5-10k-latency 5/5 PASS; COMP-03 sweep 30/30 PASS; receipts and baseline on `slice/0.8.28-40-comp03` |
| H-W1 Unit H docs | done 2026-10-16 (owner ruling): requirements § 1.6, REQ40-H1..H10, AC40-70..94; design § 2g; test-plan § 3g, § 11; evidence includes a raw-SQL plan probe for H3 |
| H-R1 plan review | APPROVE-WITH-FIXES (F1–F15) applied; rulings Q-H1..Q-H4 (coordinator), Q-H5 add COMP-02a/02b to the manifest and Q-H6 keep H1 on a hybrid-only failure (owner, 2026-10-16); Q-H7 moot; re-review APPROVE at `39d1f67dc`, N1 owner ruling (clause (c): point ≥ +1 question, CI lower bound ≥ −1 question), N2 and N3 applied |
| H-W2 harness and H1 selection | harness RED `1e472ba0b`, GREEN `6e3933784`, manifest nine ids `53909c28f`; H1 verdict `3f0789271`: adopt `had`, `long`, `suggest` (+4 questions Recall-all@10, CI +1..+8; NFCorpus nDCG@10 0.3259); H2 pool verdict `f9ae5c27b`: pool 10 (pools 20/50/100 excluded on overall Recall-all@10, 373 vs 370/369/367). Provenance: two workers shared the attempt root `~/.cache/fathomdb-comp/h2-pool-selection`: the plan's command on cuda:0 (forward, wrote the verdict) and an orchestrator helper on GPU 1 (scratch `h2_pool_reverse.py`, the same `comp03_answer.retrieve` call and per-question files) that wrote questions 358–500 (143 questions). The verdict's `inputs.runner` and `inputs.device` reflect only the last writer (cuda:0). Cross-check: questions `e6041065`, `gpt4_1d4ab0c9`, `gpt4_7bc6cf22` and `gpt4_fe651585_abs` at pools 10 and 100, recomputed on GPU 0, gave 8/8 identical id lists. The verdict file is not hand-edited |
| H1 superseded | owner ruling 2026-10-16: the H1 words are not added to the default `STOPWORDS` (stays 63); Slice 41 adds a configurable extra stop-word list, empty by default, and the benchmarks pass `had`, `long`, `suggest` through it; H1 engine RED `ee0c5e917` reverted at `fb42f612e`; H2 gate reference is the final-head baseline |

## Follow-ups (not in this slice)

- Potential, not in this slice (ruling Q-E2): a build-info accessor
  (engine commit, features, `cuda_compiled`) in all three SDKs. It is a
  public-surface change that needs its own interface-doc and parity work.
  Until then receipts record the declared commit plus artifact digests.

- `search_text_only` can return fewer than `limit` results when `limit` is
  near 100 and the fixed 100-row FTS window holds duplicate rows
  (requirements § 4). Pre-existing; the orchestrator files the ledger
  entry at landing.
- Consider adding possessive/contraction fragments (s, t, ll, re, ve) to
  STOPWORDS; needs its own measurement (R1 F2). First data point on the
  10k TC-5 corpus: 50 of 100 queries carry a fragment, and `"s"` alone
  matches 3,912 of 10,000 documents; the median candidate set per query
  is 3,806 rows with fragments and 2,744 without. Second data point,
  COMP-03 LongMemEval-S (60 questions): 41 carry a 1–2 character content
  token after the Defect B fix, almost all short pronouns and particles
  ("my", "me", "so", "up", "m", "ve"), not the "C"/"D3" cases the fix was
  for; a retrieval-only rerun on this slice's engine changed 19 of 60
  top-5 lists (session Recall@5 0.893 → 0.911, MRR 0.848 → 0.841). Extend
  the list with short pronouns and particles as well as the fragments.
- The public docs give `Σ 1/(60 + rank)`; the code uses `RRF_K = 30` and
  3:1 text weighting (design § 5).
- COMP-01 NFCorpus re-run: done, see `nfcorpus-recovery.md` (nDCG@10
  0.3087 → 0.3260, forecast 0.3260).
- Potential addition to this slice, not added, deferred by owner ruling
  2026-10-16: fielded
  title boost through declared `searchable` projections. On NFCorpus it
  measured +0.002 nDCG@10 with a confidence interval that includes zero
  (`nfcorpus-recovery.md` § hypothesis tests), so it waits for a corpus
  where titles are a real signal.
- COMP-02b projected-FTS latency: added to this slice as Defect C by owner
  ruling 2026-10-16 (requirements § 1.3, design § 2b). Evidence and the
  post-fix measurement in `comp02b-projected-fts-latency.md`.
- Potential addition to this slice, not added, deferred by owner ruling
  2026-10-16: key the attribute-filter `EXISTS` on the FTS row's cursor
  (`p.write_cursor`) instead of the node's, so SQLite probes
  `canonical_attributes` before fetching the node record. Raw SQL on the
  10k corpus: 19.6 → 12.1 ms p50 at the 1% cohort, neutral at 10% and
  33%, same row set (`comp02b-projected-fts-latency.md` § after the fix).
- Added to this slice as Unit H3 by owner ruling 2026-10-16 (a cohort
  prefilter on the rank stream; requirements REQ40-H7): a filter-first plan for very selective attribute filters,
  which fetches the cohort's cursor set first and ranks only inside it,
  the way a btree prefilter does. Needed to compete at 1% selectivity,
  where the rank stream walks about a thousand ranked rows for ten
  survivors.

- Unit D measured (orchestrator, 2026-10-16): D3 gate PASS, hybrid
  verdict ADOPT on both engines, answer v2 FTS 49/60, hybrid 48/60,
  LanceDB 46/60; see `comp03-unit-d-results.md`.
- Potential, not added, owner decision pending: a public SDK surface to
  enrol a node kind for dense vectors without declaring a `searchable`
  projection. Today a plain write under `use_default_embedder=True`
  embeds nothing and `search` silently runs FTS only; the only public
  switch is `ProjectionSpec(..., vector=True)`, which embeds the full node
  body. Carry to the ledger at landing.
- COMP-03 Unit G measured (orchestrator, 2026-10-16): answer accuracy stays at
  parity across stores within noise; see
  `dev/performance-benchmarking/2026-10-16-comp-03-unit-g-result.md` on
  `slice/0.8.28-40-comp03`.
- Baseline gains `comp03-answer-mix` (owner ruling 2026-10-16): FTS 53, hybrid 52, LanceDB 54 of 60,
  ±3 questions, report-only; provisional while COMP-03 work continues (harness `5a1d0eb12`).
- Potential, not added: raise the COMP-03 evidence cap or render evidence
  more compactly; at k = 10 the 20,000-character cap binds (p50 20,326),
  so the depth lever is cap-limited.

## Notes

- The worktree's `.venv` bootstrap ran `npm install` in `src/ts`, which
  rewrote `src/ts/package-lock.json`; restored with `git restore` (R1 F5)
  and never committed.
