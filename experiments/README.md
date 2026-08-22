# experiments/ — durable, git-friendly experiment tracking (fathomdb side)

A file-based experiment index for fathomdb **eval runs**. The machine
source-of-truth is append-only; the human table is generated. This mirrors
memex's scheme: the two repos' indices share one record + index-row schema and
are cross-compatible — the index row's `repo` field (`"fathomdb"` here,
`"memex"` there) distinguishes them.

The [overall performance benchmarking and experiments program](../dev/performance-benchmarking/PROGRAM.md)
decides what to run and in what order. This directory records what actually
ran. It already contains safe receipts for the LOCOMO campaign; it is not a
future-only capability.

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

## PROGRAM Track Runner

For new work governed by
[PROGRAM](../dev/performance-benchmarking/PROGRAM.md), the experiment harness
uses [Track Runner](../dev/performance-benchmarking/TRACK-RUNNER.md). Before
preparing a runner or configuration, execute:

```bash
./scripts/track-runner.sh check
./scripts/track-runner.sh brief <TRACK-ID>
```

The control is read-only and does not authorize a live run. A new PROGRAM
configuration declares `program_track: <TRACK-ID>` in its typed resolved
configuration, while the existing common receipt/index schemas remain stable.
Historical configurations and receipts are not rewritten just to add this
field. Corpus acquisition, GPU/model work, paid calls, and external writes
continue to require their explicit authorization gate.

Each new FathomDB measurement cell must start with
`python -m experiments.fathomdb_test_setup <external-root> <test-id>`. The
bootstrap refuses to reuse a test directory, writes explicit embedding and
reranker device policies, creates a new database, and records machine-readable
`doctor gpu`, `doctor reranker-gpu`, and post-open `doctor check-integrity`
evidence. A GPU embedding cell must pass `--embed-device cuda:N --embedder
default --warm-cache`: `auto` is not a GPU claim. `warm-cache` is explicit so
its download bytes and elapsed time are retained outside the measured cell; the
subsequent open records the resolved device and allocation witness. The safe
setup metadata belongs beside external artifacts, not in a committed receipt.

## SCALE-02 local-first envelope

`python -m experiments.scale_02` owns the A0 FTS-only efficiency envelope. Its
[configuration](configs/scale-02/a0-envelope.v1.json) binds ANSWER-01's A0
decision, the SCALE-01 primary, the 10k/17,272/25k/40k/50k ladder, fresh
databases, repetitions, cache states, resource/storage metrics, uncertainty,
and the advisory policy. Validate it or qualify all frozen inputs without
creating output:

```bash
python -m experiments.scale_02 validate experiments/configs/scale-02/a0-envelope.v1.json
python -m experiments.scale_02 dry-run experiments/configs/scale-02/a0-envelope.v1.json /external/new-root
```

The workload and advisory policy were approved together by the HITL on
2026-08-22. Execute one increasing ladder point at a time against a new
external root. Each point creates five fresh databases and writes a standard
receipt plus index row, including a blocked receipt if authorized execution
fails. The 10k and 17,272 prefixes are real; larger points add explicitly
labelled deterministic derived-fixture rows and therefore make efficiency,
not additional real-corpus fidelity, claims.

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
