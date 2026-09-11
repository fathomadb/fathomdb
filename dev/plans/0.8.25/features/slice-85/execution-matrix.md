# Slice 85 — compact execution matrix

Status: reviewed documentation baseline; execution not started.
Scope follows [the plan](plan.md). Commands/timeouts for named legacy cells
come from [the Slice 75 manifest](../slice-75/slice75-closure-manifest.json),
with the overrides below. Keep that historical manifest unchanged.

At startup, record one candidate SHA, run directory, current package version,
available executors and artifact/model paths. For each row, record actual
receipt reuse or execution result. Reuse requires relevant-input applicability;
missing/unlocated receipts require execution, not a presumed pass. Do not
duplicate a test already positively exercised by the broad round or another row.

| Coverage / legacy cell IDs | Execution route and current disposition |
| --- | --- |
| Default workspace/SDK — `default-tree` | Run `AGENT_VERBOSE=1 bash scripts/agent-verify.sh --tier=all` once. Its clippy includes all targets; supplement its ordinary workspace check with `cargo check --workspace --all-targets`. Require actual Python/TS runtime execution, not missing-tool skips. |
| Integration — `final-interactions`, `schema26-upgrade` | Retain exact selectors/features; use applicable receipts or run once if absent/invalidated. Upgrade remains populated schema 26 to current schema 33, with reopen proof. |
| Long reliability — `ac021`, `ac059b`, `ac034ab` | Retain exact commands, AGENT_LONG flags and substantive counts. AC-034c remains explicitly unavailable. Do not substitute a quick variant. |
| Accepted read performance — old `performance` AC-020/AC-072 members | Replace with [accepted Slice 80 evidence](../slice-80/current-evidence.md): AC-081a/b/c and revised-policy AC-072. Reuse unless genuinely invalidated; never invoke the retired ratio selector. |
| Text latency — remaining `performance` member | If not covered by applicable evidence, run the AC-076 command below, independently of accepted read campaigns. |
| Real corpus — `eu7-real` | Reuse only if applicable; otherwise retain the real-model/corpus command, AC-073 stress and AC-075 fidelity assertions. Confirm local corpus/models before its long run. Synthetic passes are not substitutes. |
| Live models — `model-cache`, `model-ts`, `model-engine`, `model-python`, `model-cli` | Retain the existing routes; reuse verified caches and avoid duplicate suite execution. Recount current tests, preserve their assertions and positive model execution. |
| Runtime configuration — added after Slice 75 | Reuse Slice 79 core receipts; include current `runtime_configuration.rs`, `test_slice79_runtime_configuration.py`, and `slice79-runtime-configuration.test.ts` where not already covered. Apply installed SDK cases to the actual final packages, in fresh child processes. |
| Rust/CLI — `rust-release-build`, `rust-leaf-packages`, `cli-installed-smoke` | Retain release build and local package rehearsals. Prepare the CLI smoke database with the existing fixture workflow before integrity checks. Distinguish a build-tree CLI check from a staged/installed archive smoke. |
| Linux CPU packages — `linux-artifact-build`, `linux-artifact-current-smoke`, `linux-runtime-floor-smokes` | Build each required feature artifact once; run existing installed-artifact helper, then reuse those bytes for runtime-floor checks. Resolve current release feature sets rather than assuming every older explicit build command still matches. |
| Linux CUDA — `linux-cuda-package` | Retain CUDA preflight/package/witness routes, including real embedder and reranker forwards. Resolve candidate version consistently; no hardcoded old/new version mix. |
| Packaged search — `global-01-native` | Retain installed native data-plane witness with positive search/result counts; no paid evaluation or answer-quality claim. |
| Windows/native platforms — `windows-runner-preflight`, `hosted-native-validation` | Reuse applicable Slice 73 evidence; otherwise use the current five-row native matrix. Resolve runner access before expensive work. Dispatch only with normal remote-execution authority. |
| Jetson — `jetson-tegra` | Retain the existing Python CUDA evidence route, matching actual candidate version and publish_to_pages=false. Do not add unsupported Tegra N-API or Windows CUDA cells. |
| Documentation — `mkdocs` | Run `.venv/bin/mkdocs build --strict` once. Do not also run `scripts/check.sh` as a duplicate broad suite. |
| Hosted CI — `hosted-ci` | Collect exact-candidate required-job results and explicit skipped/advisory dispositions. Reuse the same hosted run for native validation where it covers that row. No tag or publication workflow. |
| Protected writes / CE — retained `slice71-write`, `slice72` | Use newer Slice 79 write receipts; inspect Slice 72 CE input applicability. Run only invalidated candidate workloads, never historical baseline or experiment matrices. |

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
old whole-suite totals such as 109 labels or 180 N-API tests are historical;
reconcile current discovery/coverage without editing substantive assertions.

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

Before costly execution, confirm concrete commands/artifact paths and reuse
decisions. A short implementation/runner review is needed only where that route
changes; do not repeat historical design reviews. Preserve one final broad
round, with focused reruns for actual fixes and a second broad round only by
explicit approval. Close when required rows have passing or authorized non-pass
dispositions; publication remains outside scope.
