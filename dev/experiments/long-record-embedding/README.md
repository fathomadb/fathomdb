# long-record-embedding — the 512-wordpiece cut, sized and probed

Receipts for `dev/design/long-record-embedding.md`. The study measures how
much consumer-corpus text and gold evidence lies past bge-small's
510-content-wordpiece cut, and runs a preregistered probe (`PREREG.md`) of
what the cut costs vector and hybrid retrieval.

## Files

- `cut.py`, `test_cut.py` — where the cut falls in a body; span
  classification.
- `size_corpora.py` — rebuilds each corpus's record bodies as its harness
  does, tokenizes with the pinned engine tokenizer, writes
  `results/sizing.json`.
- `vector_cost.py` — vectors per record for each option, writes
  `results/vector_cost.json`.
- `probe.py`, `test_probe.py` — builds the LongMemEval-S session store and
  runs probe P1 (`results/probe_build*.json`, `probe_per_question.jsonl`,
  `probe_summary.json`).
- `probe_exploratory.py` — non-preregistered cuts
  (`results/probe_exploratory.json`).
- `latency.py` — single-caller latency per store
  (`results/latency_*.json`); `results/latency.json` is the combined table
  with the per-row fit.

## Environment

- Data: `data/corpus-data/` in the primary checkout (read only).
- Sizing and tests: any Python with `tokenizers` (the primary checkout's
  `.venv` has 0.22.2).
- Probe and latency need a CUDA build of the `fathomdb` wheel with
  `test-hooks`: the node vector kind is registered through
  `_configure_vector_kind_for_test`. The recipe is memory
  `cuda-python-wheel-local-build-recipe.md`, with features
  `pyo3/extension-module,test-hooks,default-embedder,default-reranker,embed-cuda`,
  installed into a scratch venv with `tokenizers`. Run with
  `CUDA_VISIBLE_DEVICES=0 FATHOMDB_EMBED_DEVICE=cuda:0`; the probe's
  `open_report` records the resolved device.

## Rerun

```bash
python -m pytest dev/experiments/long-record-embedding
python size_corpora.py
python vector_cost.py
python probe.py build --db <scratch>/lme_s_sessions.sqlite
python probe.py run --db <scratch>/lme_s_sessions.sqlite
python probe_exploratory.py
python probe.py build --max-chars 1500 --db <scratch>/lme_s_short1500.sqlite
python probe.py build --max-chars 1500 --max-records 4800 --db <scratch>/lme_s_short1500_n4800.sqlite
python latency.py --db <store> --out results/latency_<store>.json --n 60
```

The full-body store takes about 4 minutes to build on an RTX 3090 and about
16 minutes to probe; most of the probe time is `explain=True` at
`limit=100`.

## Deviations from PREREG.md

- None in the strata, arms, metric or rule.
- The committed `probe.py` imported the drain helper from
  `src/python/eval/p0a_base_retrieval.py`. The first build failed because the
  primary checkout's wheel has no CUDA. The CUDA venv cannot import that
  package, so before the run the helper was replaced with a local drain loop
  that counts `_fathomdb_vector_rows`. Coverage was 1.0 either way.
- `probe.py build` gained `--max-chars` and `--max-records` after the run,
  for latency attribution only. The probe store was built without them.
