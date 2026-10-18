# long-record-embedding — the 512-wordpiece cut, sized and probed

Receipts for `dev/design/long-record-embedding.md`. The study does two things:

- It measures how much consumer-corpus text and gold evidence lies past
  bge-small's 510-content-wordpiece cut.
- It probes what the cut costs vector and hybrid retrieval. The result of
  record is P2b, a paired position intervention (`PREREG-P2.md`, amended by
  `PREREG-P2b.md`). P1 (`PREREG.md`) and P2 are superseded; the design doc's
  § 5.3 says why.

## Files

- `cut.py`, `test_cut.py` — where the cut falls in a body; span
  classification.
- `size_corpora.py` — rebuilds each corpus's record bodies as its harness
  does, tokenizes them with the pinned engine tokenizer, and writes
  `results/sizing.json`, including the IR-C per-source rows.
- `vector_cost.py` — vectors per record for each option; writes
  `results/vector_cost.json`.
- `probe.py`, `test_probe.py` — build the LongMemEval-S session store and run
  P1 (`results/probe_build*.json`, `probe_per_question.jsonl`,
  `probe_summary.json`). `build` also makes the latency stores
  (`--max-chars`, `--max-records`, `--copies`).
- `probe_exploratory.py` — P1's non-preregistered cuts, including the
  within-type effects (`results/probe_exploratory.json`).
- `probe2.py`, `test_probe2.py` — the P2 and P2b rotated stores and paired
  runs. `--twins` selects P2b. They write `results/probe2*_summary.json`,
  `probe2*_per_question.jsonl` and `probe2_build.json`.
- `latency.py` — single-caller latency per store
  (`results/latency_<store>.json`).
- `latency_fit.py` — combines those files into `results/latency.json` and
  fits hybrid minus text against vector rows.
- `diag_text.py`, `diag_bodies.py`, `diag_bm25.py`, `diag_counts.py`,
  `diag_twins.py` — the P2 failure diagnosis
  (`results/diag_text.json`, `diag_twins.json`).
- `store_sizes.py` — vector-table bytes per store. It was not run before
  the stores were deleted (see Deviations).

## Environment

- **Data:** `data/corpus-data/` in the primary checkout, read only.
- **Sizing and tests:** any Python with `tokenizers`. The primary checkout's
  `.venv` has 0.22.2.
- **Probes and latency:**
  - They need a CUDA build of the `fathomdb` wheel with `test-hooks`,
    because the node vector kind is registered through
    `_configure_vector_kind_for_test`.
  - The build recipe is memory `cuda-python-wheel-local-build-recipe.md`,
    with features
    `pyo3/extension-module,test-hooks,default-embedder,default-reranker,embed-cuda`.
    Install the wheel into a scratch venv with `tokenizers`.
  - Run with `CUDA_VISIBLE_DEVICES=0 FATHOMDB_EMBED_DEVICE=cuda:0`. The
    build's `open_report` records the resolved device.

## Rerun

```bash
python -m pytest dev/experiments/long-record-embedding
python size_corpora.py
python vector_cost.py
python probe.py build --db <scratch>/lme_s_sessions.sqlite
python probe.py run --db <scratch>/lme_s_sessions.sqlite        # P1, superseded
python probe_exploratory.py
python probe2.py build --twins --db <scratch>/lme_s_rotated_twins.sqlite
python probe2.py run --twins --original <scratch>/lme_s_sessions.sqlite \
  --rotated <scratch>/lme_s_rotated_twins.sqlite                # P2b
python probe.py build --max-chars 1500 [--max-records N | --copies 2] --db <scratch>/<store>.sqlite
python latency.py --db <scratch>/<store>.sqlite --out results/latency_<store>.json --n 60
python latency_fit.py
```

Each 19,195-session store takes 4–6 minutes to build on an RTX 3090. A P1
or P2 run takes about 10 minutes; most of that time is `explain=True` at
`limit=100`.

## Deviations

From `PREREG.md` (P1):

- **Drain helper.** The committed `probe.py` imported its drain helper from
  `src/python/eval/p0a_base_retrieval.py`. The first build failed because the
  primary checkout's wheel has no CUDA, and the CUDA venv cannot import that
  package. So before the run, the helper was replaced with a local drain loop
  that counts `_fathomdb_vector_rows`. Coverage was 1.0.
- **Deviations section.** `PREREG.md` names `REPORT.md § Deviations`; they
  are here instead.
- **Hybrid arm.** P1's `hybrid` arm was read from the `limit=100,
  explain=True` call, not from a separate shipped `limit=10` call as
  `PREREG.md` states. At `limit=100`, up to 100 vector hits enter fusion
  instead of 10. P2 and P2b measure `limit=10` separately.
- **Tests.** `test_probe.py` was written after `probe.py` was committed,
  though before the run, so it was not red-first. `_strata` and the
  vector-rank extraction have no unit tests.
- **Twins.** P1 did not account for body-identical twin sessions (see P2
  below).

From `PREREG-P2.md` (P2):

- **Invalid run.** P2 ran as written and failed its validity check
  (85.4 % text agreement). No rule was applied.
- **Amendment.** The cause was twin sessions merging in fusion, which the
  diagnostics establish. `PREREG-P2b.md` amends the design, and was
  committed before the P2b run.

Other:

- **Missing build receipts.** The root disk filled up during the session,
  so `probe2b_build.json` and `probe_build_short1500_n9600.json` were never
  written, and the 38,390-record store's build process died with a bus
  error after its drain.
  - Row counts checked afterwards with `diag_counts.py` showed all three
    stores complete: every node had its FTS rows and its vector.
  - The 38,390-record latency run took place while the disk was nearly
    full.
- **Stores deleted.** With the owner's approval, the stores were deleted
  to free disk before `store_sizes.py` ran. The design doc computes vector
  bytes from the vec0 layout instead.
- **Duplicate explain ranks.** Explain ranks are keyed by body, so twins
  share one rank. P2b counts twins as gold, so this does not bias it.
