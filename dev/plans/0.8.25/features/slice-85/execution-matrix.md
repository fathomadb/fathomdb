# Slice 85 — compact execution matrix

Status: execution in progress; local functional, package, model, reliability,
and runtime-floor routes have executed. Hosted/native closure is not yet ready.
Scope follows [the plan](plan.md). Commands/timeouts for named legacy cells
come from [the Slice 75 manifest](../slice-75/slice75-closure-manifest.json),
with the overrides below. Keep that historical manifest unchanged.

Planning baseline is `56e6d7fd`; the execution candidate will be the clean
GREEN commit after manifest implementation and code review. The run directory
is `dev/plans/runs/0.8.25-slice-85/`. Current package version is `0.8.24`.
Local preflight found Rust/Cargo 1.95, Python 3.12, Node 25.9, npm 11.19,
actionlint, lychee, MkDocs, maturin, gh and virsh. Unconfined preflight on local
host `windchill3` finds the exact registered RTX 3090 UUID, uv-managed CPython
3.10.20/3.11.15, system Python 3.12 and nvm Node 25.9.0. The matrix uses their
absolute paths. Node 25 is the sole Node target used throughout the 0.8.25
development and test cycle. This is the same host/device/CPU-affinity envelope
as the retained Slice 72 baseline.

For each row, record actual
receipt reuse or execution result. Reuse requires relevant-input applicability;
missing/unlocated receipts require execution, not a presumed pass. Do not
duplicate a test already positively exercised by the broad round or another row.

| Coverage / legacy cell IDs | Execution route and current disposition |
| --- | --- |
| Default workspace/SDK — `default-tree` | Run `AGENT_VERBOSE=1 bash scripts/agent-verify.sh --tier=all` once. Its clippy includes all targets; supplement its ordinary workspace check with `cargo check --workspace --all-targets`. Require actual Python/TS runtime execution, not missing-tool skips. |
| Integration — `final-interactions`, `schema26-upgrade` | Run both exact legacy commands once. Upgrade remains populated schema 26 to current schema 33, with reopen proof. |
| Long reliability — `ac021`, `ac059b`, `ac034ab` | Retain exact commands, AGENT_LONG flags and substantive counts. AC-034c remains explicitly unavailable. Do not substitute a quick variant. |
| Accepted read performance — old `performance` AC-020/AC-072 members | Replace with [accepted Slice 80 evidence](../slice-80/current-evidence.md): AC-081a/b/c and revised-policy AC-072. Reuse unless genuinely invalidated; never invoke the retired ratio selector. |
| Text latency — remaining `performance` member | Run the corrected AC-076 command below, independently of accepted read campaigns. |
| Real corpus stress — `eu7-real` | The exact legacy route ran. Preserve its overall failed output and extract only its candidate-bound AC-073 PASS (`p99=391 ms`, bound `730 ms`) into the structured closing receipt. Its stale AC-075 assertion is explicitly non-countable. |
| Real corpus fidelity — `tc5-bridge` | Decision `seq-31` assigns grown-corpus fidelity to TC-5; the later direct-HITL amendment makes TC-5 GPU-primary. Repeat only the registered 7,667-document, 100-query GPU smoke against exact candidate wheel/CLI/private-binary bytes. This is not the optional CPU equivalence bridge. Require the registered fixture and ground-truth digests, a valid candidate SUT-result digest and AC-075's `CI-high >= 0.90` oracle; retain the 17,272-document primary without rerunning it. |
| Live models — `model-cache`, `model-ts`, `model-engine`, `model-python`, `model-cli` | Run the exact legacy routes after cache preflight. Preserve expected positive counts 33/9/13/2 and zero skips. |
| Runtime configuration — added after Slice 75 | Require 7/4/3/3 source tests and six installed fresh-process cases. Positive selector/count evidence from the broad round satisfies a source row; otherwise run the exact focused command below. |
| Rust/CLI — `rust-release-build`, `rust-leaf-packages`, `cli-installed-smoke` | Retain release build and local package rehearsals. Prepare the CLI smoke database with the existing fixture workflow before integrity checks. Distinguish a build-tree CLI check from a staged/installed archive smoke. |
| Linux CPU packages — `linux-artifact-build`, `linux-artifact-current-smoke`, `linux-runtime-floor-smokes` | Run the legacy build with Python features `pyo3/extension-module,default-embedder` and the default npm native build, then run the installed helper and reuse those exact bytes for runtime-floor checks. |
| Linux CUDA — `linux-cuda-package` | Run the corrected CUDA route below on local `windchill3` with CUDA 12.6 and registered RTX 3090 UUID `GPU-5f9cfc90-2be1-06a7-ce39-5a6d294b209b`. Preserve its `0.8.24` local rehearsal artifact names and require real embedder and reranker forwards. This is a truthful pre-version-cut rehearsal, not the Tegra staging overlay. |
| Packaged search — `global-01-native` | Retain installed native data-plane witness with positive search/result counts; no paid evaluation or answer-quality claim. |
| Windows/native platforms — `windows-runner-preflight`, `hosted-native-validation` | Run the legacy preflight and exact-SHA five-row native matrix. Slice 79 invalidated Slice 73's N-API/TypeScript inputs. Dispatch only with normal remote-execution authority. |
| Jetson — `jetson-tegra` | Run the existing Python CUDA workflow with branch/version `release/0.8.25`, the workflow's copied `0.8.25+tegra` staging overlay, and `publish_to_pages=false`. Do not add unsupported Tegra N-API or Windows CUDA cells. |
| Documentation — `mkdocs` | Run `.venv/bin/mkdocs build --strict` once. Do not also run `scripts/check.sh` as a duplicate broad suite. |
| Hosted CI — `hosted-ci` | After local verification/review, fast-forward GREEN to local `release/0.8.25`, obtain explicit push authority, push, and confirm the remote branch head equals `FINAL_SHA` before dispatch. Collect required-job results and explicit skipped/advisory dispositions. Reuse the same hosted run for native validation where it covers that row. No tag or publication workflow. |
| Protected writes / CE — retained `slice71-write`, `slice72` | Rerun the six Slice 79 candidate-only 71B cells because the recorded broad Engine-source invalidation set changed, while recording that the changes are `debug_assertions`-gated rather than an observed release-path regression. Rerun Slice 72 candidate CPU/CUDA CE on local `windchill3`, CPU 0 and the registered RTX 3090 UUID so the retained same-host baseline remains comparable. Never rerun historical baseline/experiment matrices. |

Initial disposition: Slice 80 AC-081/AC-072 are `reuse`; the exact current
input digest is `95e15e3e4c089212431b7a173fca291539a072d98c87d3394c1fb9b4f3573274`,
identical to the accepted receipts. Slice 72 CE, Slice 79 protected writes,
Slice 73 Windows, all original unexecuted Slice 75 cells, installed artifacts, native
platforms and hosted CI are `run`/`rerun`. AC-034c alone is `unavailable` under
its pre-existing accepted disposition. The completed CUDA Engine comparison is
closed by the [CUDA Engine p95 exception](ce-engine-p95-exception.md), recorded
as `accepted-non-pass`; this does not change any other threshold or verdict.
Nothing else begins as a pass.

The legacy EU7 execution has since completed: AC-073 passed, while AC-075
failed at recall `0.772`, CI `[0.743, 0.798]`. This is not an input-invalid run
and is not converted to a pass. The old fidelity cell is stale because corpus
growth changes its first-N population; the accepted `seq-31` disposition and
completed TC-5 track supersede only that cell. The candidate-bound TC-5 GPU
smoke repetition is therefore the sole remaining local fidelity action.

## Sealed added and override routes

All commands run from the clean candidate worktree with a per-cell timeout.
The 26 legacy command arrays, feature sets, timeouts and positive counts are
adopted from `slice75-closure-manifest.json#/cells`. These are the only added
or replaced routes:

| Route | Exact command/immutable key | Timeout and positive result |
| --- | --- | --- |
| Runtime Rust owner | `cargo test -p fathomdb-engine --test runtime_configuration -- --test-threads=1` | 900 s; 7 passed |
| Runtime statement reuse | `cargo test -p fathomdb-engine statement_reuse_ --lib -- --test-threads=1` | 900 s; 4 passed |
| Runtime Python source | `python -m pytest src/python/tests/test_slice79_runtime_configuration.py -q`; omit this focused route when the broad-round log already proves the same selector/count | 300 s; 3 passed |
| Runtime TypeScript source | `npm run build:debug --prefix src/ts` then `src/ts/node_modules/.bin/tsc -p src/ts/tsconfig.json` then `node --test src/ts/dist/tests/slice79-runtime-configuration.test.js`; omit this focused route when the broad-round log already proves the same selector/count | 900 s; 3 passed |
| Runtime installed SDKs | `bash scripts/release/smoke/smoke-local-native-artifacts.sh $RUN_DIR/artifacts/python src/ts src/ts/npm/linux-x64-gnu linux-x64-gnu scripts/release/smoke/slice73-windows-napi-modules.json` after its Slice 85 runtime-smoke addition | 7,200 s shared artifact smoke; three Python and three Node child-process cases plus retained positive markers/tests |
| Runtime floors | Preserve the helper/artifact-reuse protocol while resolving its aliases: run Python 3.10 with `FATHOMDB_SMOKE_PYTHON=/home/coreyt/.local/share/uv/python/cpython-3.10.20-linux-x86_64-gnu/bin/python3.10`, 3.11 with `/home/coreyt/.local/share/uv/python/cpython-3.11.15-linux-x86_64-gnu/bin/python3.11`, and 3.12 with `/usr/bin/python3.12`; every invocation uses `FATHOMDB_SMOKE_NODE=/home/coreyt/.nvm/versions/node/v25.9.0/bin/node`. Preserve `FATHOMDB_SMOKE_LIGHTWEIGHT=1` and every remaining legacy argument. | 7,200 s total; Python 3.10/3.11/3.12 and Node 25 consume the same artifact bytes, each invocation reports three Python plus three Node runtime cases, and all finish without source fallback or skips |
| AC-076 | Command below | 3,600 s; one test, 1,000 queries, p50 <=20 ms, p99 <=150 ms |
| AC-081/AC-072 | `slice80/current-evidence.md` and exact input SHA-256 below | Reuse seven AC-081 passes, AC-081c, and C1–C5 AC-072 passes |
| TC-5 AC-075 GPU smoke | Build `fathomdb-tc5-benchmark` with `tc5-benchmark-cuda`; recreate `$RUN_DIR/tc5-runtime` with `venv --clear`; force-reinstall the exact CUDA candidate wheel from `$RUN_DIR/cuda-preflight.packages` there with `pip install --no-deps --force-reinstall`; generate `$RUN_DIR/tc5-candidate-config.json` from the frozen TC-5 v2 pins with release `0.8.25`, package version `0.8.24`, `FINAL_SHA`, retained input/model digests, registered GPU UUID and exact wheel/CLI/private-binary paths and hashes; use `$RUN_DIR/tc5-runtime/bin/python` for both `experiments.tc5_gpu_v2 dry-run ... --arm bridge` and `run ... --binary $RUN_DIR/artifacts/fathomdb-tc5-benchmark` | 7,200 s; isolated runtime consumes the pinned wheel; 7,667 real documents, 100 complete queries, zero synthetic rows, fixture `9e92d236…e622ed`, ground truth `ef6be77b…f34b`, a valid SUT-result digest, AC-075 `CI-high >= 0.90`, and candidate/artifact identity match. Candidate result: recall `0.954`, CI `[0.936, 0.971]`; diagnostic repetition matched those metrics and the SUT digest. |
| Protected writes | `slice-79/execution-manifest.json#/protected/write`, using its one offline locked release probe build and exactly three `scale02` plus three `ac013` candidate cells under `$RUN_DIR/write/{scale02,ac013}-production-{1,2,3}` | 1,800 s build; 900 s/cell; Scale-02 ack <=1543.539 ms and total <=1548.545 ms, AC-013 total <=1442.198 ms, each gated spread <=25% |
| Installed CE | Copy `slice-72/ce-profile-manifest.json` to `$RUN_DIR/slice72-ce-manifest.json`, changing only `candidate_sha` to `$FINAL_SHA`; the Slice 85 validator rejects any other field change. Use that overlay consistently with `scripts/release/slice72_ce_artifact.py`, `run-slice72-ce-profile.py`, and `verify-slice72-ce-profile.py`; reuse baseline cells at `4fc1b890`, build candidate CPU with `pyo3/extension-module,default-reranker` and CUDA with `pyo3/extension-module,rerank-cuda`, run `--role candidate --device cpu` and `--role candidate --device cuda --cuda-visible-devices GPU-5f9cfc90-2be1-06a7-ce39-5a6d294b209b`, then validate the four-cell aggregate | 7,200 s/device; each path has three cold processes and five steady processes × 20 calls; correctness/tolerance pass and candidate median-p95 <=1.10 × retained matching baseline |
| Linux CUDA package | `env FATHOMDB_CANDIDATE_SHA=${FINAL_SHA} FATHOMDB_CUDA_GPU_UUID=GPU-5f9cfc90-2be1-06a7-ce39-5a6d294b209b bash scripts/release/cuda-preflight.sh ${RUN_DIR}/cuda-preflight --rerank-cuda`; build the CLI with `CUDA_HOME=/usr/local/cuda-12.6`, `LIBRARY_PATH=/usr/local/cuda-12.6/lib64` and `/usr/local/cuda-12.6/bin` first on `PATH`; seal the four artifacts and manifest; invoke `slice75-cuda-package-smoke.sh` with the same UUID and `CUDA_HOME` | 10,800 s; witness and artifact manifest bind the candidate SHA; driverless CPU plus real-GPU Python/N-API embed and reranker forwards pass, and CLI GPU/reranker doctors are present |
| Native/CI/Tegra | `slice75-closure-manifest.json#/cells/{windows-runner-preflight,hosted-native-validation,jetson-tegra,hosted-ci}` | Exact legacy timeouts and expected rows/counts; no publication |

The installed runtime helper and bounded TC-5 candidate-binding extension are
the only existing-runner edits. Their committed REDs respectively require all
six fresh-process cases and reject an unbound 0.8.25 TC-5 configuration. GREEN
adds those cases and candidate/artifact provenance without changing the
retained frozen/dependency/lifecycle workloads or immutable 0.8.23 TC-5 config.

Three Rust integration binaries intentionally open raw SQLite before `Engine`
to construct states the public API cannot create: compatibility creates
old/future/poisoned/mismatched/locked databases, tokenizer recall creates a
pre-upgrade corpus, and provenance creates legacy/null provenance rows. They
remain test-only fixture builders. Each configures the process runtime before
either SQLite image is initialized; the retained 12-test set is the closure
proof. They do not add a production dual-SQLite mode or an ambient test hazard.

## Small command and contract corrections

The remaining text-performance command is:

```sh
timeout 3600 env -u AC_FULL_SCALE AGENT_LONG=1 AC012_CORPUS_N=10000 \
  cargo test --release -p fathomdb-engine --test perf_gates \
  ac_012_text_query_latency_on_fts5_path -- --exact --nocapture --test-threads=1
```

It must execute one test and 1,000 measured queries, with p50 <=20 ms and
p99 <=150 ms. Build outside qualification where the retained protocol requires
environment collection. Inspect the actual runner and use a short path smoke
before any longer run whose invocation/artifact route changed.

Do not reuse the old `performance.expected.passed=3` or ratio field: its cell
is now split between AC-076 execution and accepted Slice 80 evidence. Likewise,
the whole-suite total of 109 labels is current by direct registration count;
180 N-API tests remains a historical expectation to reconcile at execution
without editing substantive assertions.

CPU package feature sets must match current package/Cargo defaults and release
workflow, including the reranker where shipped. Verify rather than equating
an explicit feature list with the complete effective set. The current checkout
still carries pre-version-cut package metadata: use its actual version for
non-publishing rehearsal, or label an approved staging overlay. Do not silently
cut a version to satisfy old package filenames or the Tegra expected version.

Keep a risk-weighted API coverage map using existing suites: mutations/erasure,
eligibility, lifecycle, error codecs, startup runtime modes and SDK/native
interactions need behavioral evidence. Group equivalent lower-risk surfaces;
do not multiply every API by every platform/runtime unnecessarily.

## Review and completion

This matrix was checked against the current typed verifier, existing test
selectors, Slice 75 cell inventory, Slice 79 runtime contract and Slice 80
accepted evidence. Every legacy cell ID is retained or explicitly superseded;
legacy exclusions remain as recorded unless separately changed. No tests ran
as part of this documentation review.

Before costly execution, bind the clean GREEN `FINAL_SHA` and artifact hashes
in the final manifest. All command arrays/timeouts are the exact legacy values
in `slice75-closure-manifest.json` except: replace `performance` with the AC-076
command above plus Slice 80 receipt reuse; add current runtime-configuration
source/installed cases; rerun Slice 79's six candidate-only 71B commands; and
rerun Slice 72's registered candidate CPU/CUDA CE profile against its retained
baseline. A short implementation/runner review is needed only where that route
changes; do not repeat historical design reviews. Preserve one final broad
round, with focused reruns for actual fixes and a second broad round only by
explicit approval. Close when required rows have passing or authorized non-pass
dispositions, using the
[CUDA Engine p95 exception](ce-engine-p95-exception.md) as the Slice 72 CE
disposition reference; publication remains outside scope.

After a passing evidence review, merge to `release/0.8.25`, verify ancestry and
tree identity, and remove the temporary Slice 85 branch/worktree. Main remains
untouched.
