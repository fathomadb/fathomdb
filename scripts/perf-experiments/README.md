# Performance experiment runners

## Performance Gauntlet v1.2

`run-gauntlet.sh` is the top-level entry point for repeating the existing
performance cells against another FathomDB release. It preserves the established
cell runners, inputs, and output formats rather than reimplementing them.

The Slice 10 entry point supports a metadata-only dry run. It intentionally does
not read the supplied paths or create the output directory:

```bash
scripts/perf-experiments/run-gauntlet.sh \
  --release 0.8.26 \
  --source-root /path/to/fathomdb-0.8.26 \
  --config /path/to/gauntlet.json \
  --output-root /path/to/results \
  --dry-run
```

The canonical cell order is AC-076, AC-072, AC-081, AC-073, AC-075,
SCALE-02, protected writes, CE profile, SEARCH-01, and LOCOMO. Use `--cells`
with a comma-separated subset; the runner still restores canonical order.

AC-073 and AC-075 corpus encoding are CUDA-only and pin an exact RTX 3090
UUID. The runner fails closed if the CUDA toolkit, driver, or selected device
is unavailable; it never falls back to CPU encoding. SEARCH-01 and LOCOMO use
FTS-only retrieval and perform no neural encoding.

### Optional graph suite

`--suite graph` selects `graph-evidence01`, `graph-expand01`, and
`graph-retrieval01` without changing the default ten cells. The zero-cost
end-to-end check is:

```bash
PYTHONPATH=src/python .venv/bin/python \
  scripts/perf-experiments/run-graph-suite-smoke.py \
  --release 0.8.26 \
  --source-root /path/to/fathomdb \
  --output-root /tmp/fathomdb-graph-gauntlet-smoke
```

`materialize-graph-retrieval-seeds.py` is the separate one-time MuSiQue
top-20 qualifier. It requires an explicit RTX 3090 UUID and
`FATHOMDB_EMBED_DEVICE=cuda:0`; unavailable CUDA or any CPU fallback is a hard
failure. After its 300/300 historical top-10 parity check qualifies a manifest,
`run-graph-retrieval-historical.py` runs the depth-one and depth-two native
expansion arms. The checked-in retrieval config deliberately leaves the seed
manifest digest null until that qualification succeeds.

## 0.7.0 perf-experiments harness

Driver + per-lever + aggregator scripts that produce the data
required to lock HITL Q1 / Q2 / Q4 in
`dev/plans/0.7.0-perf-experiments.md`.

## Layout

```text
scripts/perf-experiments/
  README.md                         (this file)
  run-experiment.sh                 (driver; takes EXP_ID + LEVER_ID)
  collect-host-spec.sh              (host CPU/kernel/glibc/SQLite/rustc capture)
  run-ac012.sh                      (AC-012 only; respects AC012_CORPUS_N)
  run-ac020.sh                      (compatibility name; AC-081a/b successor only)
  run-ac013.sh                      (AC-013 only; respects AC013_CORPUS_N)
  emit-output-json.py               (assembles per-experiment closure JSON)
  aggregate.py                      (walks dev/plans/runs/0.7.0-PERF-EXP-*.json; emits master table)
  ledger-check.sh                   (cross-checks proposed lever against do-not-retry ledger)
```

## Quick start

Dev-box pre-screen (cheap, fast, 24-core x86_64 — informational):

```bash
EXP_ID=W1.1 LEVER_ID=L-A0 AC012_CORPUS_N=100000 \
  bash scripts/perf-experiments/run-experiment.sh
```

Canonical CI (authoritative for verdicts):

```bash
gh workflow run perf-canonical.yml --ref <branch> \
  -f experiment_id=W1.1 \
  -f lever_id=L-A0 \
  -f ac012_corpus_n=1000000 \
  -f run_full_scale=true \
  -f perf_record=false
```

## Closure JSON convention

Each experiment writes
`dev/plans/runs/0.7.0-PERF-EXP-<EXP_ID>-output.json` per the schema
in `dev/plans/0.7.0-perf-experiments.md § Closure-JSON schema`.

## Aggregation

```bash
python3 scripts/perf-experiments/aggregate.py \
  --runs-dir dev/plans/runs \
  --out dev/plans/runs/0.7.0-perf-experiments-results.md
```

## Ledger discipline

Before adding a new lever, run
`bash scripts/perf-experiments/ledger-check.sh <LEVER_ID>`. The
script reads
`dev/notes/performance-whitepaper-notes.md § 5` and surfaces any
match — if matched, the lever's honest-retry argument must already
be recorded in
`dev/plans/0.7.0-perf-experiments.md § Lever taxonomy`.
