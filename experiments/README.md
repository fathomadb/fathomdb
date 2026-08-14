# experiments/ — durable, git-friendly experiment tracking (fathomdb side)

A file-based experiment index for fathomdb **eval runs**. The machine
source-of-truth is append-only; the human table is generated. This mirrors
memex's scheme: the two repos' indices share one record + index-row schema and
are cross-compatible — the index row's `repo` field (`"fathomdb"` here,
`"memex"` there) distinguishes them.

> Scope note: this slice provides the CAPABILITY. No fathomdb eval run is wired
> to it yet; the eval program adopts it when it lands.

## Layout

```text
experiments/
  index.jsonl                 # append-only; ONE JSON line per run (source-of-truth)
  INDEX.md                    # GENERATED human table (never hand-edit)
  SCOREBOARD.md               # GENERATED aggregate capability comparison table
  runs/<run_id>/
    record.json               # canonical per-run record (typed schema)
    config.resolved.yaml      # the config AFTER defaults+overrides merge
    metrics.json              # flat metrics (also embedded in record.json)
  _lib.py                     # the shared, pure, typed helper (TDD'd)
```

New `record.json` files use `experiments.record.v1`; new index rows use
`experiments.index-row.v1`. Versionless historical records/rows are read as
v0 without being rewritten. Campaign configs and sidecars retain their own
schemas (for example `earp.v1` / `earp.result.v1`), nested inside this common
envelope.

`run_id = <experiment-slug>-<UTC-ts:YYYYMMDDTHHMMZ>-<config_sha8>`, where
`config_sha8` is the first 8 hex of the sha256 of the canonical-JSON of the
resolved config. Given a fixed timestamp + config, the `run_id` is deterministic.

## The standing rules

1. **An experiment is a typed CONFIG + an index line + a durable record — never
   a forked script.** New experiments are new config files (the
   `eval/*/config.py` / acquire `_config.py` convention), not bespoke runners
   with inlined constants.
2. **Config is typed and consumed-or-loudly-rejected.** An unknown or missing
   field raises at load; the same discipline governs `record.json` (see
   `_lib.record_from_dict`).
3. **Every eval run writes a `record.json` + appends `index.jsonl`** (via
   `_lib.write_record`, then `_lib.regen_index_md`) BEFORE it is "closed".
4. **Verdicts distill into the human layer.** The one-line `read` + `verdict`
   on each record is the honest finding that rolls up into
   [`dev/experiments-ledger.md`](../dev/experiments-ledger.md).

## Rules for the writer

- `index.jsonl` is **append-only**: never rewrite or reorder existing lines.
- `INDEX.md` is **generated** from `index.jsonl` (`_lib.regen_index_md()` is
  idempotent). Do not hand-edit it.
- `SCOREBOARD.md` is **generated** from the safe aggregate fields in committed
  `runs/*/record.json` receipts (`python -m experiments.scorecard`). It is a
  comparison layer, not a raw-artifact store; do not hand-edit it.
- `_lib.py` is **pure/no-network** and the timestamp is passed IN by the caller
  (a live runner supplies `datetime.now(UTC)`), so hashing/`run_id` stay testable.
- Git subprocess calls strip `GIT_DIR`/`GIT_WORK_TREE` (`_lib.git_env`) so a
  pre-push hook's inherited repo-location can never redirect them.

## Registry vs. experiment — the split

Two distinct ledgers, do not conflate:

- **Data registry** — corpus acquisition. A dataset is registered in
  `tests/corpus/scripts/manifest.json` + `tests/corpus/corpus-card.md`. See
  [`../tests/corpus/scripts/README.md`](../tests/corpus/scripts/README.md).
  Answers "what data exists and is it byte-stable".
- **Experiment index** — this directory. Answers "what did we measure over the
  data". A retrieval/answer-quality eval run gets a record + an index line here;
  it never lands in the corpus manifest, and a corpus acquisition never lands
  here.

`dev/experiments-ledger.md` is the human-distillation layer above this machine
index — prose findings and narrative; `index.jsonl` is the structured ledger.

## Native comparator receipts

External benchmark harnesses use the same generic record layout but keep their
own result schema. They must not be relabelled as EARP runs. The native Mem0
OSS LOCOMO adapter is `python -m experiments.mem0_oss`; start from
`configs/mem0-oss/locomo-native-predict.example.json`. It accepts no
credentials in its configuration. The campaign's Mem0 container receives an
Airlock key through its environment, while the resolved experiment config
records only the Airlock endpoint, aliases, and hashes of the non-secret
Compose/config overlays. The configuration also pins the isolated Python
interpreter used to execute the official harness; it never reuses FathomDB's
development environment.

Native harness output remains under a durable, access-controlled external
output root because it can contain corpus-derived questions, answers, and Mem0
memories. The receipt stores only a content-free aggregate manifest digest and
safe summaries; it never copies payloads or their filenames into
`experiments/runs/`.

The adapter ships `configs/mem0-oss/compose.airlock.override.example.yaml` and
`configs/mem0-oss/mem0-airlock.example.yaml`. They bridge the Mem0 container to
the already-running host systemd Airlock; they neither start nor configure an
Airlock container.

The initial matched FathomDB arm is exposed by
`python -m experiments.fathomdb_oss_facade`. It implements the official Mem0
OSS HTTP seam with one private FathomDB FTS database per official user ID. The
current public FTS API returns at most ten results, so the paired campaign is
pinned to `top_k: 10` and cutoff `10`; a larger retrieval-depth comparison is
not emitted until FathomDB exposes a compatible limit control.

Run the two configured arms with `python -m experiments.mem0_oss run …` and
`python -m experiments.fathomdb_locomo run …`; then pass their `record.json`
files to `python -m experiments.mem0_comparison`. The comparison writer refuses
non-complete or workload-mismatched arm receipts.

For the native arm, `python -m experiments.mem0_oss services-up …` starts only
the named campaign's Mem0 and Qdrant Compose services; then `preflight` must
pass before `run`. It never starts an Airlock container.
