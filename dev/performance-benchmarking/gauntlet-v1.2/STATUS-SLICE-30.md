# Performance Gauntlet v1.2 — Slice 30 status

**State:** DESIGN REVISION  
**Slice:** 30 — default ten-cell execution map and compatibility seams

## Reconciliation since the project plan

- Slice 10 fixes public `--dry-run` as metadata-only and path-opaque. Slice 30
  adds an import-safe config-aware planner for Slice 50 preflight and Slice 70
  execution; it does not weaken the public dry-run contract.
- The first code-grounded design review returned **CHANGES REQUIRED**. What
  appeared to be one declarative map actually crosses four independent
  contracts: runner invocation, release overlays, protected-write evidence,
  and IR/CE output handoff. Slice 30 is divided into 30A–30D so each owner has
  its own RED/GREEN, review, and verification gate.
- Gauntlet scripts and adapters are resolved from the current gauntlet checkout.
  Product builds, Cargo working directories, and source identity come from the
  target release checkout. No invocation may accidentally build current `main`
  while claiming the target tag.
- Existing historical configs, manifests, receipts, result indexes, and output
  trees remain read-only. Every new overlay/receipt/output is rooted beneath the
  new gauntlet run.

## Slice 30A — exact invocation map

Add `scripts/perf-experiments/gauntlet_cells.py`, an import-safe,
standard-library planner. A cell plan has exactly `cell`, `adapter`, `cwd`,
`env`, `unset_env`, `invocations`, `inputs`, `outputs`, `timeout_s`, and
`workload_identity`. Each invocation has an argument vector and stable label;
no shell command string or benchmark implementation is embedded.

The exact mapped contracts are:

- AC-076: one `run-ac012.sh` invocation from the target checkout with
  `AGENT_LONG=1`, `AC012_CORPUS_N=10000`, output-local `LOG_PATH`, and
  `AC_FULL_SCALE` unset. The existing runner retains its own build and selector.
- AC-072: one `run-slice80-ac072-campaign.sh` invocation with 10 positional
  arguments: target checkout, built binary, new raw root, source commit, then
  six SHA-256 values for binary, input, runner, scanner, validator, and
  dispatcher. The dispatcher retains R1/R2/R3 and the 10k/384d/1,000-query
  protocol. Its legacy config is provenance-only and must be labeled as such.
- AC-081: seven distinct `run-slice80-ac081-cell.sh` argument vectors, each with
  target checkout, built binary, new raw log, source commit, binary hash, and
  input hash. The runner retains its single exact selector/fresh process.
- AC-073: parse only the `eu7-real` command from the frozen closure manifest
  with `shlex.split`; permit only `${RUN_DIR}` substitution; retain timeout
  10,800, both environment unsets, and pins 7,667 documents, 100 queries, 1,000
  bootstrap samples, 1,000 latency samples, and 250 stress operations. Require
  target `data/corpus-data` to canonical-resolve to configured corpus data and
  set `FATHOMDB_EMBEDDER_CACHE` to resolved
  `assets.tc5.embedder_model_cache`. Validate the frozen corpus contract of 13
  `raw/*.jsonl` files and canonical SHA-256
  `af1484a4873e61d98647ea44ab3cb452a3b42d9d7ccd32babb68f50ba531b65e`,
  plus the model-cache hashes. Missing/disconnected corpus/cache, a SKIP, or
  absent EU7 output is never runnable/completed.

Slice 30A generated placeholders are limited to
`artifact:perf_gates.{path,sha256,input_sha256}`. Slice 50 resolves them after
one target-checkout build. AC-072 uses six hashes, not “seven hashes.”

## Slice 30B — release overlay compatibility

### AC-075 TC-5

- Add `artifact:tc5_benchmark.{path,sha256}`. Slice 50 builds
  `fathomdb-tc5-benchmark` with `tc5-benchmark-cuda`; the overlay and `--binary`
  must bind the same path/hash.
- Invoke `python -m experiments.tc5_gpu_v2 run --config <overlay> --arm bridge
  --output-root <new> --binary <artifact>` with GPU UUID, corpus, qualified
  manifest, model cache, wheel/CLI/native runtime identities, K=192, 100
  queries, the historical seeds, exact-f32 top-10 truth, and 1,000 resamples.
- Preserve 0.8.23 as no-candidate. Preserve the sealed 0.8.25 rule requiring
  version 0.8.25 and package 0.8.24. Add only the 0.8.26 rule requiring version
  and package 0.8.26; do not broadly accept arbitrary versions.

### SCALE-02

- Add release to the loaded config model and use `config.release` throughout
  validation, runtime attestation, summaries, and records. Historical 0.8.23
  config behavior remains unchanged.
- Add CLI `--record-base-dir`; all five target points share one new output-local
  registry so prior-point checks see earlier target receipts without falling
  back to historical indexes.
- Keep points 10,000/17,272/25,000/40,000/50,000, five fresh repetitions,
  corpus/workload/uncertainty/advisory policy, cold/steady/mutation/storage/RSS/
  throughput semantics, and target runtime hash checks unchanged.

## Slice 30C — directional protected-write contract

Do not edit or filter the sealed Slice 71B v1 protocol. Its checked-in manifest
binds the live v1 runner hash and its validator deliberately requires the exact
18-cell factorial.

Add a separate versioned `protected-production` manifest, runner, validator,
and receipt schema that reuse the low-level probe/build/environment helpers but
own a new identity. The contract is exactly two fixtures (`scale02`, `ac013`) ×
production treatment × three repetitions, with explicit target source commit,
manifest, and output root. It preserves raw logs, environment snapshots,
failure disposition, probe/build/runtime identities, and fails closed on any
partial/unbound sequence. The v1 manifest, runner behavior, hash validation,
receipt schema, and historical raw root must remain byte-for-byte valid and
untouched.

## Slice 30D — CE and IR adapters

### Installed CPU/CUDA cross-encoder profile

- Fixed read-only inputs include
  `dev/plans/runs/0.8.25-slice-72/baseline-cpu/baseline-cpu.json` (SHA-256
  `c093a4e131515496552f90bc6e8adf7d3b864b371815c4ceeab408c3bb4c2b39`)
  and `dev/plans/runs/0.8.25-slice-72/baseline-cuda/baseline-cuda.json`
  (SHA-256
  `e40fbccc9c357d6ada801e234b8a9bf372418195d1cb32b6646cd8d28cc1196f`).
  The frozen manifest SHA-256 is
  `4535895d0ae0e2bb1febd61e036c9013f7c607114b250dab3458980fa6cfaae9`.
- The output-local overlay changes only fields actually present: target
  candidate SHA and selected CUDA UUID/model. Output/runtime paths stay in
  artifact receipts and command arguments.
- Plan two candidate artifact builds, candidate CPU/CUDA profile cells, and the
  exact verifier invocation over baseline CPU/CUDA plus candidate CPU/CUDA.
  Preserve both standalone/engine paths and 3 cold, 5 steady, 20 calls.
- Require the same RTX 3090 model as the historical validator in addition to
  UUID. The historical 1.10 ratio is reported informationally; it does not
  become a new release gate. Retain all four cell JSON files and verifier
  stdout/stderr/exit status. The existing verifier may exit before producing an
  aggregate receipt above 1.10; that missing aggregate is acceptable only with
  the retained informative verifier failure.

### SEARCH-01 / IR-C

- Add a thin `run-search01.py` adapter around
  `eval.earp.characterize.run_characterization` only. SEARCH-01 is the IR-quality
  characterization; `fresh_store`/`fresh_store_warm_query` belonged to a
  separate historical performance command and are removed from this cell.
- Require corpus hash
  `fe973fcd49fbbda083158f69fe720f17858ab8528e171fa2188eec84131c7d4e`,
  qrels `ir-c-reused-v2`, gold SHA
  `4caabdd7f8325481fb80f4c81bfe3688111c7141e53b75f85e17c123b2ab8771`,
  10,506 snapshot rows, 4,597 gold queries, cutoffs `(5, 10)`, an aware
  timestamp, and a typed nonzero blocked result. The example EARP config's
  placeholder gold hash and default-embedder scenario are not authoritative.
- The adapter contains no ingest, retrieval, scoring, percentile, bootstrap,
  or record-writing implementation.

### LOCOMO A0

- Add a thin output-local adapter (or explicit new CLI parameters) that calls
  `experiments.fathomdb_locomo` and `experiments.locomo_metrics` while retaining
  deterministic path handoff. It must accept a base directory/run identity and
  emit a JSON result naming predictions, provenance, facade/raw metrics, and
  summary paths. It must never write the repository `experiments/runs` index.
- Enforce canonical A0: turn-level ingestion, FTS-only, top 10, 272 sessions,
  1,540 eligible questions, raw and normalized corpus hashes, provenance
  manifest hash, and target CLI/Python runtime.
- `locomo_metrics` owns retrieval/provenance metrics only. Timing remains in
  facade metrics/raw sidecars and must not be attributed to its summary.

## Required binding table

| Cell | Required resolved bindings |
| --- | --- |
| AC-076 | target source; output root |
| AC-072 | target source; provenance config; generated `perf_gates` identity |
| AC-081 | target source; generated `perf_gates` identity |
| AC-073 | target source; frozen closure manifest; AC-073 corpus; shared TC-5 embedder cache |
| AC-075 | TC-5 config/corpus/manifest/embedder cache; GPU; target Python/wheel/native/CLI; generated TC-5 benchmark |
| SCALE-02 | SCALE config; TC-5 corpus/manifest; target Python/native/CLI; isolated record base |
| Protected writes | new protected manifest; target source; generated probe/binary identities; new output root |
| CE profile | CE manifest; model cache; exact GPU model/UUID; frozen baseline cells; target source/Python |
| SEARCH-01 | SEARCH config; IR-C root/snapshot/manifest/gold; target Python/wheel/native; new output root |
| LOCOMO | LOCOMO config; harness checkout/Python; dataset/provenance; target CLI/Python; new output root |

Every planner rejects a missing/null required binding. Unused bindings are not
presented as consumed or authoritative.

## Acceptance

- 30A–30D each complete a separate RED/GREEN, design/code review, and
  independent verification gate before aggregate Slice 30 closes.
- A config-aware plan resolves the selected default cells in canonical order to
  exact existing runners or thin adapters without subprocess execution.
- Historical workload identities and repetition counts above are exact.
- Existing sealed/historical configs and protocols remain valid and unchanged.
- The orchestrator and adapters contain no benchmark fixture, retrieval,
  scoring, percentile, or bootstrap implementation.
- Focused tests cover every required/null binding, exact argv/env/unsets,
  placeholders, overlays, repetition counts, output-local behavior, and all
  owning-module regressions.

## Evidence

The initial design review returned **CHANGES REQUIRED** with eleven findings.
The revised 30A–30D design above incorporates each finding. Re-review pending.
