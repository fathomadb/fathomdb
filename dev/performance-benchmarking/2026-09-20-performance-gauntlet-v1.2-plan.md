# Performance Gauntlet v1.2 project plan

**Status:** implemented and executed against 0.8.26  
**Project:** Performance Gauntlet v1.2  
**Purpose:** run one target FathomDB release through the existing
0.8.25-era performance and retrieval-quality workloads.

**Initial target:** implement the project, then execute the full default
gauntlet against the `v0.8.26` release.

## Outcome

Create a thin top-level orchestrator that preserves the existing runners,
configurations, inputs, and result formats. The project may make compatibility
changes needed to bind those assets to a newer FathomDB release, but it must
not duplicate benchmark implementations, scoring logic, or statistical
analysis.

The resulting measurements are informative and directional. This project does
not attempt to make them release-gating or statistically defensible claims.

## Scope

The default gauntlet contains the eight system-performance cells plus
SEARCH-01 and LOCOMO A0:

1. AC-076 text retrieval.
2. AC-072 vector retrieval.
3. AC-081 mixed reads.
4. AC-073 real-corpus stress.
5. AC-075 TC-5 fidelity.
6. SCALE-02.
7. Protected writes.
8. Installed CPU/CUDA cross-encoder profile.
9. SEARCH-01 / IR-C.
10. LOCOMO A0.

The runner executes one target release at a time with the same workload and
input identities used for 0.8.25 wherever those assets remain available.
Historical results may be displayed beside the new observations, but no
historical receipt or configuration is rewritten.

## Non-goals

- No new benchmark workloads, retrieval treatments, scoring metrics, or
  acceptance thresholds.
- No deep review or redesign of existing performance scripts. Establishing
  that they still run is sufficient.
- No tuning sweep or optimization work.
- No paid answer/judge execution.
- No publication, release authorization, or registry changes.
- No reinterpretation of retired AC-020 or known-invalid synthetic AC-013b
  evidence.
- No deletion of historical runs, user-created worktrees, or unrelated
  branches. Cleanup is limited to temporary state created by this project.

## Slice ladder

### Slice 10 — User-facing entry point

Add:

- `scripts/perf-experiments/run-gauntlet.sh` as the stable user-facing command.
- `scripts/perf-experiments/run_gauntlet.py` for the stable CLI, cell-selection
  validation, and metadata-only dry-run projection. Configuration loading,
  subprocess dispatch, status tracking, and summary generation belong to later
  slices.
- `tests/experiments/test_perf_gauntlet.py` for focused orchestration tests.
- Usage documentation in `scripts/perf-experiments/README.md`.

The intended invocation is:

```bash
scripts/perf-experiments/run-gauntlet.sh \
  --release 0.8.26 \
  --source-root /path/to/v0.8.26-worktree \
  --config /path/to/gauntlet-0.8.26.json \
  --output-root /external/fathomdb-gauntlet/0.8.26
```

The entry point supports:

```text
--preflight-only
--dry-run
--cells ac076,ac072,ac081
--resume
--fail-fast
```

Once the later execution slices land, default behavior runs all cells
sequentially, continues after a failed cell, and exits nonzero after writing a
complete summary if any cell failed. Until then, Slice 10 rejects ordinary
execution and preflight-only mode explicitly.

#### Slice 10 acceptance

- The command exposes help without inspecting external data.
- `--dry-run` resolves the default ten cells in stable order.
- Cell selection, resume, and fail-fast controls have explicit CLI coverage.
- The shell entry point contains no measurement or metric logic.

### Slice 20 — Configuration model

The gauntlet configuration references existing configurations rather than
reproducing their workload definitions. It contains only:

- `experiments/configs/gauntlet/directional-release.example.json` as the
  reviewed example for release-specific paths and runtime bindings.

- Target release and clean source checkout.
- External output root.
- Target-release CLI binary, Python, wheel, virtual-environment, and
  native-extension paths. The `perf_gates` test executable is built and
  identified during Slice 50 preflight, not predicted in user configuration.
- Paths to existing frozen AC-072, AC-073, AC-075, SCALE-02, protected-write,
  CE-profile, SEARCH-01, and LOCOMO configurations.
- Closed path groups for IR-C, AC-073, LOCOMO, TC-5/SCALE-02 corpus, embedder,
  and cross-encoder assets.
- CUDA UUID and model-cache path when GPU cells are enabled.
- Enabled cells and basic timeouts.

The orchestrator generates release-specific resolved overlays beneath the
output directory. Overlay changes are restricted to:

- Release/version and commit identity.
- Runtime executable and artifact paths.
- Runtime artifact hashes.
- Output locations.
- Fields that must change solely because an old runner explicitly permits only
  0.8.23 or 0.8.25.

Historical configurations and receipts remain unchanged.

#### Slice 20 acceptance

- A strict schema rejects missing, unknown, or non-whitelisted overlay fields.
- The resolved configuration records the target source and artifact hashes.
- A dry run proves no historical file is selected as an output target.
- The resolved overlay is written as `gauntlet-plan.resolved.json` beneath a
  new gauntlet output root.

### Slice 30 — Default ten-cell execution map

Implement only the adapters and compatibility seams needed to dispatch the
following existing work.

Implementation is gated as four internal sub-slices, each with its own
RED/GREEN, review, and verification cycle:

- **30A:** exact AC-076/AC-072/AC-081/AC-073 invocation map.
- **30B:** AC-075 TC-5 and SCALE-02 release overlays.
- **30C:** a new directional protected-production contract, leaving sealed
  Slice 71B v1 unchanged.
- **30D:** CE, SEARCH-01, and LOCOMO output-local adapters.

#### AC-076 text retrieval

Use `scripts/perf-experiments/run-ac012.sh` with the 0.8.25 parameters:

```text
AGENT_LONG=1
AC012_CORPUS_N=10000
AC_FULL_SCALE unset
```

Run once, matching the final 0.8.25 execution. Preserve its raw log and
`AC012_NUMBERS` output.

#### AC-072 vector retrieval

Build the release `perf_gates` executable once and use:

- `scripts/perf-experiments/run-slice80-ac072-campaign.sh`.
- `scripts/perf-experiments/run-slice80-ac072-cell.sh`.

The orchestrator computes the required source, binary, input, runner, scanner,
validator, and dispatcher hashes. Run the existing three-cell
10k/384-dimensional/1,000-query campaign.

Use `scripts/perf-experiments/run-ac013.sh` for preflight or smoke work only.
Do not duplicate its measurement logic.

#### AC-081 mixed reads

Build once and invoke
`scripts/perf-experiments/run-slice80-ac081-cell.sh` in seven fresh processes,
matching the accepted 0.8.25 campaign.

The compatibility-named `scripts/perf-experiments/run-ac020.sh` may provide a
short preflight, but the retired AC-020 ratio selector must never run.

#### AC-073 real-corpus stress

Load the existing command from
`dev/plans/0.8.25/features/slice-75/slice75-closure-manifest.json` rather than
copying the command into the orchestrator. Preserve:

- 7,667 documents.
- 100 queries.
- 1,000 bootstrap samples.
- 1,000 latency samples.
- 250 stress operations per thread.
- The same real corpus, model, and output schema.

Run once, as in 0.8.25. The target checkout's `data/corpus-data` must
canonical-resolve to the configured corpus root, whose 13 `raw/*.jsonl` files
have canonical identity
`af1484a4873e61d98647ea44ab3cb452a3b42d9d7ccd32babb68f50ba531b65e`.
Set `FATHOMDB_EMBEDDER_CACHE` to the resolved shared TC-5 embedder cache. A
missing/disconnected corpus or cache, a skipped test, or absent EU7 output is a
refusal, never a successful observation.

Encoding is CUDA-only. Replace the frozen command's `default-embedder` feature
with `embed-cuda`, set `FATHOMDB_EMBED_DEVICE=cuda:0`, and pin
`CUDA_VISIBLE_DEVICES` to the configured RTX 3090 UUID. An unavailable CUDA
driver, toolkit, device, or UUID is a typed refusal; CPU fallback is forbidden.

#### AC-075 TC-5 fidelity

Use `python -m experiments.tc5_gpu_v2` with the existing 7,667-document bridge
configuration and target-release artifacts.

Slice 50 builds `fathomdb-tc5-benchmark` with feature
`tc5-benchmark-cuda` and records its path/hash. The output-local overlay's
candidate benchmark path/hash and the runner's `--binary` must bind that same
artifact.

Use the exact frozen base
`experiments/configs/scale-01/tc5-gpu-v2.json` (SHA-256
`b2ea5c25eee0b93807384259262702d3bb04f1fee4f640579160091ecee2417c`).
Slice 30 creates an output-local template with typed benchmark placeholders;
Slice 50 resolves both benchmark fields, validates the resulting config with
the existing loader, writes it atomically, and records the final overlay
path/hash. Execution consumes only that final identity. The explicit overlay
pointers are `/release`, `/runtime/python`, `/runtime/fathomdb_bin`,
`/runtime/cuda_uuid`, `/inputs/corpus_root`, `/inputs/qualified_manifest`,
`/inputs/model_asset_directory`, and the added `/candidate` object; all other
workload and input identities remain frozen.

Make only the compatibility change required because
`experiments/tc5_gpu_v2.py` currently permits releases 0.8.23 and 0.8.25 and
contains 0.8.25-specific candidate metadata checks. Preserve the exact three
branches: 0.8.23 has no candidate, sealed 0.8.25 requires version 0.8.25 with
package 0.8.24, and the new 0.8.26 branch requires both version and package
0.8.26. Do not generally accept arbitrary versions.

Do not change the corpus, query selection, model, `K=192`, ground truth, seeds,
or 1,000-resample calculation.

#### SCALE-02

Use:

```bash
python -m experiments.scale_02 run-point ...
```

Run the existing ladder:

```text
10000
17272
25000
40000
50000
```

Each point retains five fresh databases and the existing cold/steady,
mutation, storage, RSS, and throughput measurements.

Compatibility changes to `experiments/scale_02.py` may:

- Replace the hardcoded `RELEASE = "0.8.23"` check with the release declared
  in a resolved overlay.
- Accept target-release runtime paths and hashes.
- Add a required `--record-base-dir`. All five target points share one new
  output-local receipt registry so prior-point checks see earlier target
  receipts without reading or modifying historical experiment records.

Use the exact approved base
`experiments/configs/scale-02/a0-envelope.v2.json` (SHA-256
`eb86d5b41e63b4854bde695200b9a0b9552a5c2474650f7cb0762b862851cd28`).
Only release, corpus/manifest paths, and target runtime identity fields may
change. Prior-point checks must bind the same release and resolved overlay,
not merely a matching point/verdict. Run 10,000, 17,272, and 25,000 in formal
ladder mode; use the existing `--post-boundary-baseline` authorization only for
40,000 and 50,000.

Both TC-5 and SCALE-02 invoke the target virtualenv Python from the gauntlet
checkout, whose runner/helper modules are recorded and hashed. This keeps the
compatibility seam in the gauntlet checkout while product imports come from
the target isolated runtime.

No workload or policy fields change.

#### Protected writes

Leave the sealed Slice 71B v1 manifest, runner behavior, hash binding, exact
18-cell validator, receipt schema, and historical output root unchanged.

Add a separate versioned directional protected-production manifest, adapter,
validator, and receipt contract reusing the existing low-level probe/build/
environment helpers. It runs exactly three SCALE-02 production cells and three
AC-013 production cells with explicit target source commit and new output root,
and retains the existing raw logs, environment snapshots, runtime identities,
failure disposition, and partial-failure evidence.

#### Installed CPU/CUDA cross-encoder profile

Reuse:

- `scripts/release/run-slice72-ce-profile.py`.
- `scripts/release/slice72_ce_artifact.py`.
- `scripts/release/verify-slice72-ce-profile.py`.
- `dev/plans/0.8.25/features/slice-72/ce-profile-manifest.json`.

Generate an output-local manifest overlay for the target release and execute
candidate CPU and CUDA cells.

The fixed baseline inputs are:

- `dev/plans/runs/0.8.25-slice-72/baseline-cpu/baseline-cpu.json`, SHA-256
  `c093a4e131515496552f90bc6e8adf7d3b864b371815c4ceeab408c3bb4c2b39`.
- `dev/plans/runs/0.8.25-slice-72/baseline-cuda/baseline-cuda.json`, SHA-256
  `e40fbccc9c357d6ada801e234b8a9bf372418195d1cb32b6646cd8d28cc1196f`.
- The frozen manifest, SHA-256
  `4535895d0ae0e2bb1febd61e036c9013f7c607114b250dab3458980fa6cfaae9`.

Plan two candidate artifact builds, candidate CPU/CUDA cell runs, and the exact
four-cell verifier input. Require the same RTX 3090 model and selected UUID.

Because this campaign is directional, preserve the exact 0.8.25 protocol:
three cold processes and five steady processes with 20 calls. Do not implement
the later proposed 100-call/three-phase replacement in this project.
The verifier's 1.10 comparison is informational for this directional gauntlet.
Always retain all four cell JSON files and verifier stdout, stderr, and exit
status. A ratio failure is an informative cell result, not a release decision;
the aggregate verifier receipt is optional because the existing verifier exits
before writing it on such a failure.

#### SEARCH-01 / IR-C

The EARP harness has reusable characterization logic but no live
characterization CLI. Add a thin
`scripts/perf-experiments/run-search01.py` adapter that calls the existing
`eval.earp.characterize.run_characterization` API with the historical inputs:

- 10,506 documents.
- 4,597 queries.
- FTS-only `search_text_only`.
- Limit 10.
- Evidence recall at 5 and 10.
- Corpus hash
  `fe973fcd49fbbda083158f69fe720f17858ab8528e171fa2188eec84131c7d4e`.
- Qrels version `ir-c-reused-v2` and gold SHA-256
  `4caabddf7ce55f417e639e3c169fe2035b09c231f36d2f39d293a596373de2bb`.

The adapter must not implement retrieval, scoring, or record writing. It stages
the target release wheel in an isolated environment and delegates to EARP.
`fresh_store` and `fresh_store_warm_query` belonged to a separate linked
performance campaign and are not part of this IR-quality cell.

#### LOCOMO A0

Run only canonical A0, not the historical 26-cell tuning grid:

- Turn-level ingestion.
- FTS-only retrieval.
- Top 10.
- The same 272 sessions and 1,540 source questions.
- The same corpus and provenance hashes.

Add a thin output-local adapter over `experiments.fathomdb_locomo` and
`experiments.locomo_metrics`. It owns deterministic path handoff, accepts a new
base directory/run identity, and emits JSON naming predictions, provenance,
facade/raw metrics, and retrieval-summary paths without writing the repository
`experiments/runs` index.

Create a release-specific configuration from
`experiments/configs/mem0-oss/locomo-fathomdb-seam.example.json`, changing only
target runtime, output, and release-compatible fields.

The metrics step continues to produce R@5, R@10, MRR, R@1, nDCG@10, and
temporal evidence recall. Timing remains in facade metrics/raw sidecars; it is
not attributed to `locomo_metrics`.

#### Slice 30 acceptance

- A config-aware dry plan resolves all ten cells to existing scripts, modules,
  manifests, or thin compatibility adapters. The public `--dry-run` remains the
  Slice 10 metadata-only, path-opaque projection; Slice 50 preflight consumes
  the resolved execution map.
- The orchestrator contains no benchmark fixture, retrieval, scoring, or
  percentile implementation.
- Each cell preserves the historical workload identity and repetition count.
- Compatibility changes are additive and covered by focused tests.

### Slice 40 — Optional legacy extension

Expose `scripts/perf-experiments/run-scale-ac013-matrix.sh` as an optional cell
named `ac013-scale-matrix`.

Do not include it in the default ten-cell gauntlet. Its 10k/100k/1M cold/warm
matrix was not part of the final 0.8.25 eight-cell set, but it remains useful
for comparison with older AC-013 history.

#### Slice 40 acceptance

- The optional cell is absent from the default expansion.
- Selecting it delegates to the existing matrix script unchanged.
- Its outputs are retained beneath its own cell directory.

### Slice 50 — Runner-health preflight

Implement `--preflight-only` to establish that the existing runners work well
enough without deeply reviewing their internals:

- Run `bash -n` for referenced shell scripts.
- Invoke `--help`, `validate`, or existing dry-run commands for Python runners.
- Confirm required runner, configuration, and input paths exist.
- Validate frozen external input hashes using existing validators.
- Confirm the target source checkout is clean and matches the requested
  release.
- Build the release test executable with
  `cargo test --release --no-run -p fathomdb-engine --test perf_gates`.
- Build the target `fathomdb-tc5-benchmark` executable with
  `tc5-benchmark-cuda` when AC-075 is selected, then record its path and SHA-256.
- Confirm the target wheel imports from its intended isolated environment.
- For GPU cells, verify the configured CUDA UUID, required GPU model, and cached
  embedder/reranker assets.
- Require a new output root unless `--resume` was explicitly requested.

Do not add a general runner audit, new statistical analysis, or new acceptance
policy.

#### Slice 50 acceptance

- Preflight performs no measured benchmark cell.
- Every unavailable prerequisite is reported against its cell.
- CPU-only selection does not require GPU prerequisites.
- Successful preflight is sufficient to begin the requested cells.

### Slice 60 — Orchestration behavior

Run cells sequentially so one benchmark does not overlap another.

For each cell:

1. Write `cell-plan.json` with the resolved runner and arguments.
2. Mark the cell `running` in gauntlet status.
3. Invoke the existing runner.
4. Preserve stdout, stderr, raw logs, receipts, resolved configurations, and
   runner-created metrics unchanged.
5. Write `cell-result.json` containing:
   - Cell ID.
   - Runner path and hash.
   - Exit code.
   - Start and finish timestamps.
   - Target release and commit.
   - Paths and hashes of resulting artifacts.
   - Completion state.
6. Continue to the next cell unless `--fail-fast` was requested.

The orchestrator does not independently recompute benchmark metrics. It may
copy headline values from existing structured receipts into the final summary.

#### Slice 60 acceptance

- Default execution is sequential.
- Failure preserves the failed cell's output and does not erase earlier work.
- Resume skips only cells whose release, configuration, runner, and artifact
  identities still match.
- The final process exits nonzero if any requested cell is incomplete or
  failed.

### Slice 70 — Output contract

Use this layout:

```text
<output-root>/
  gauntlet-plan.resolved.json
  gauntlet-status.json
  gauntlet-summary.json
  gauntlet-summary.md
  runtime/
    artifact-manifest.json
  cells/
    ac076/
    ac072/
    ac081/
    ac073/
    ac075/
    scale02/
    protected-writes/
    ce-profile/
    search01/
    locomo/
```

`gauntlet-summary.json` is the machine-readable source. Generate the Markdown
view from it.

The summary presents observed values beside known 0.8.25 values when their
workloads are compatible. It labels every comparison `directional` and reports
missing or incompatible cells rather than silently omitting them.

Existing runner-created `record.json`, `config.resolved.yaml`, `metrics.json`,
logs, and receipts remain unchanged within their cell directories. The
gauntlet does not append these directional runs to the repository's official
experiment index by default.

#### Slice 70 acceptance

- The JSON summary fully determines the generated Markdown view.
- Every requested cell has a terminal status and artifact list.
- No raw corpus-derived payload is copied into the summary.
- A partial campaign still produces valid status and summary documents.

### Slice 80 — TDD and focused compatibility coverage

Write failing tests before implementing each orchestration behavior. Cover:

1. Default expansion produces exactly the requested ten cells in stable order.
2. Every cell delegates to an existing runner or module; no benchmark
   implementation appears in the orchestrator.
3. `--cells` selects a subset without changing cell commands.
4. `--dry-run` creates no databases or benchmark outputs.
5. A failed cell is retained and later cells still run.
6. `--fail-fast` stops after the first failure.
7. `--resume` skips completed cells and rejects mismatched release or
   configuration identity.
8. Runtime overlays may change only whitelisted compatibility fields.
9. Historical configurations and receipts are never modified.
10. The final summary is produced for complete, partial, and failed campaigns.

Use fake executables and temporary directories in orchestration tests. Existing
runner-specific tests remain the oracle for runner behavior.

For every compatibility-touched component, add only the focused tests needed
to prove acceptance of a new release/runtime/output binding while preserving
the old configuration behavior.

#### Slice 80 acceptance

- Tests demonstrate red before implementation and green afterward.
- Existing 0.8.23 and 0.8.25 configurations continue to validate unchanged.
- No generated golden or snapshot oracle is introduced.

### Slice 90 — Verification

After implementation:

1. Run the new orchestration tests.
2. Run existing focused tests for every compatibility-touched component:
   - SCALE-02.
   - TC-5.
   - Slice 71B.
   - Slice 72 cross-encoder profiling.
   - EARP.
   - LOCOMO.
3. Run the gauntlet with `--preflight-only`.
4. Run `--dry-run` and inspect the ten resolved commands.
5. Run a cheap `ac076,ac072,ac081` subset.
6. Run `./scripts/agent-verify.sh`.
7. Only then launch the full 0.8.26 gauntlet.
8. Preserve the declared gauntlet result directory and durable receipts, then
   remove task-created temporary builds, run directories, worktrees, and
   branches after proving they are no longer needed.

#### Slice 90 acceptance

- Focused and full agent verification pass.
- Preflight and dry-run complete without executing a measured cell.
- The cheap subset produces the expected output contract.
- The full 0.8.26 invocation completes, or every unavailable cell has a
  retained typed result that names its missing prerequisite.
- No task-created temporary worktree, orphan branch, or superseded run
  directory remains after the final result is sealed.

## Project definition of done

Performance Gauntlet v1.2 is complete when one command can run all ten default
cells against a selected FathomDB release using the historical workloads and
external inputs, without hand-editing runner commands. It must preserve
existing output artifacts, survive partial failures and resumptions, produce
one directional summary, and contain no duplicated benchmark, retrieval,
scoring, or metric implementation. The initial project execution must also run
the full default gauntlet against `v0.8.26`, retain its declared result and
typed unavailable outcomes, and clean up only the temporary state created by
the project.
