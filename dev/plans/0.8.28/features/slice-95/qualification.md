---
title: Slice 95 — candidate qualification matrix
status: IN_PROGRESS
target_release: 0.8.28
---

# Candidate qualification matrix

This is the compact execution inventory required by S95-1 and S95-6, not a new
validator or acceptance oracle. [Plan](plan.md) and [design](design.md) retain
scope; [status](status.md) records the eventual verdict. Executable inputs must
be reviewed and frozen before final broad work. Publication remains held.

## Hosted CI inventory

All commands are owned by `.github/workflows/ci.yml`; bind final outcomes to its
exact dispatched commit and actual jobs, rather than copying an older run.

| Jobs | Purpose and route | Selection and evidence policy |
| --- | --- | --- |
| changes | Pinned paths-filter, trusted ci-lite classification, validated manual candidate | Automatic paths remain proportional; successful exact-candidate dispatch selects all eight scoped drivers even with empty diff. Invalid dispatch cannot allocate qualification work. |
| verify-fast / verify | Canonical agent-verify fast (lint/type/security/cheap suites) and complementary heavy (Rust/Python/TS) | Total/disjoint registration, real nonzero failures and actual counts; pinned cold prerequisites. Fast diagnostics survive genuine classifier failure, not invalid candidate validation. Owned redacted spill evidence on failure. |
| rust-workspace-race-report | Parallel Rust diagnostic | Compile jobs=2; test threading unchanged; debug/incremental and target caching disabled, cache save disabled. Actual Cargo exit/count/log and resource transitions remain separate from advisory wrapper/upload results. |
| security | Strict AC-036/037/038/050a/050c on ubuntu-22.04 | Live namespace/catch lines and 0 violations/blockers/downgrades required. Verify-fast's optional live downgrade is not a substitute. |
| default-embedder-tests | Cached BGE CPU/live Rust/Python/Node tests | Warm failure is visible skipped live coverage, never a release pass; no retries/quarantine. |
| wheel-size-gate | Five CPU target wheel-size cells | Shipped default-embedder size against existing platform budgets. No-feature size informational; actual five rows required. |
| native-artifact-runtime-validation / collect-native-artifact-receipts | Five fresh candidate wheel+Node/addon installed-consumer rows and existing collector | Linux x64/ARM64, macOS x64/ARM64, Windows x64; isolated strict modular TS consumer plus runtime calls. Receipt collector rejects missing/duplicate/cross-candidate rows. Native hashes do not cover main package/declarations; retain those identities separately. |
| windows-wal-checkpoint-diagnosis / windows-wal-attribution | Real Windows WAL diagnosis and installed hook-wheel attribution | Exact candidate and explicit PowerShell external exit checks; distinct WAL receipt. Shipped package tests cannot depend on disposable private hooks. |
| gitleaks | Pinned tracked-tree scan | Always-on advisory policy unchanged. Generated evidence is scanned separately locally. |
| shell-lint | Pinned ShellCheck install/lint and job contract | Independent always-on cheap signal, including docs changes. Fixtures isolate host binaries; real installer/version checks stay strict. |
| board-currency / ledger-integrity / plan-anchors | Git/state/ledger/plan consistency | Independent always-on, no docs-only shortcut; no hand-edited generated current pointers. |
| governed-surface-pin / pinned-override-rot / c1-contract-conformance | Approved surfaces, dependency pins and C1 contracts | Independent always-on; preserve ratified pins/frozen baselines. |
| transcript-hygiene / release-state-views / commission-manifest | Safe tracked transcripts, generated views and commission references | Independent always-on; tests remain real and fail closed. |
| design-status / steward-orient / docs | Design metadata, orientation fixture and strict MkDocs | Independent always-on; orientation fixture is a test, not adoption of a Steward role. |
| markdownlint | Markdown-only automatic shortcut | Intentionally skipped during full manual qualification, whose verifier/docs jobs cover the applicable checks. |

The eight scoped drivers are verify, race report, security, default embedder,
wheel size, native artifacts and the two Windows WAL jobs. Independent jobs
retain their ordinary-check identity and do not become candidate package receipts
merely because they ran. Queued/cancelled/skipped/zero-test rows are not passes.

## Local final candidate routes

| Obligation | Existing command/inputs and acceptance | Execution disposition |
| --- | --- | --- |
| Canonical correctness | Node25.9 PATH, checkout-owned .venv, CARGO_BUILD_JOBS=2 scripts/agent-verify.sh; strict tracing executor | One clean final broad round, then only failed/invalidated scopes. Record registered/ran/pass/fail/skip/excluded totals. |
| Foreground/resource fixture isolation | CI9/10 observed RED/GREEN, Astra APPROVE9328885b2;825 fixture assertions and144 parallel engine units pass; final exact-candidate CI pending | Preserve all assertions, true Autonomous controls and production default10GiB rejection at8GiB; six disposable-repository helpers declare1GiB through existing option. |
| Confirmed-debt admission | RED bff636c3b / GREEN aabfbee8e, Astra design/code APPROVE; independent27 real-DB tests PASS | CI8 discovered by canonical hosted serial tests; unchanged original regressions, true-debt priority/progress and genuine Autonomous saturation refusal. Final core-source phase gets one broad round. |
| Feature coverage | check-test-target-coverage.py; test-feature-complete.sh --scratch with live cached BGE/reranker/nomic/ONNX and two RTX3090s | Inventory at baseline:356 targets,36 feature-complete-only. Recompute current accounting through existing runner, no fixed-count oracle. Separate named cold-storage target avoids root disk pressure. |
| Python subscriber admission | CI12 deterministic native contention RED/GREEN; unchanged installed-wheel subscriber lifecycle/reentry/replacement suite | Preserve accepted below-capacity delivery,4096 bound/full-only loss and prompt disable; refresh affected Python artifacts/features and final exact-candidate CI. |
| Strict security | STRICT=1 scripts/agent-security.sh | Host supports namespaces/ptrace unconfined. Record both actual AC037 live pass lines and 0/0/0; no artificial host-security grant/revert. |
| AC011 writes | CARGO_NET_OFFLINE=true scripts/run-ac011-write-throughput.sh; canonical windchill3/NVMe TMPDIR | Reviewed6d580ec57:1 KB1315.423/s PASS;100 KB305.975/s PASS, helper0; before/after/enclosing readiness applicable. Preserve all earlier non-passes in the history below. Thresholds and5s/60s protocol unchanged; diagnostic tracing never qualifies. |
| AC012/013 latency | Release perf_gates exact selectors with AGENT_LONG=1; AC012_CORPUS_N=10000; AC013_CORPUS_N=10000 and AC013_VECTOR_DIM=768 | Existing text p50/p99 <=20/150ms; vector <=80/300ms,1000 samples. Synthetic recall/stress remain report-only. |
| AC081 absolute reads | Release perf_gates::ac_081_absolute_read_performance; seven fresh processes of one hashed executable | Existing <=500ms sequential/<=100ms concurrent; retired AC020 ratios stay retired. |
| Long reliability | AGENT_LONG=1 exact lifecycle_reliability::ac_021_zero_sqlite_schema_warnings_under_concurrent_reads_and_ddl and cursor_read_after_write::projection_cursor_bounds_observed_row_count | Actual60s/60DDL/60rebuilds/schema0 PASS. Original1000-search cursor path passed but lacked positive-write proof; CI14 strengthens that existing fixture. Reuse unaffected AC017/018/081c. |
| AC073 stress / legacy EU7 | Release ignored eu7_real_corpus_ac_validation; CUDA, N7667,100 queries,1000 bootstrap/latency samples,250/thread | Retain actual exit and independently evaluate stress. Superseded recall assertion can produce exit101; never label the overall run PASS. Corpus staging receipt binds13 top-level JSONLs/96,704,727 bytes. |
| AC075 fidelity | Existing experiments.tc5_gpu_v2 dry-run/run, candidate-bound bridge7667 real docs/100 queries/K192/top10/exact f32 truth | CI-high >=0.90. Reuse frozen input/model pins; renew candidate SHA, CUDA wheel/runtime/CLI/private benchmark hashes. Explicit .28 release/.27 package validator binding remains unchanged. No 2M sweep. |
| Packages/consumer boundaries | CI five-target native matrix and distinct WAL; independent Linux installed consumer/missing-export control and external Rust source archive | Preserve dependency/native/package hashes, actual resolved paths/lockfiles/runtime flags. Earlier S92 examples reusable only after explicit relevant-input comparison; CPU dev proof is not optimized/registry/CUDA proof. |
| Documentation/contracts | Slice92 baseline plus affected docs/index/lifecycle/authority/generator/parity/surface checks, strict local MkDocs | No new full audit or structural framework. Pending .28 docs/published .27 receipts/manifests stay distinct. |
| Evidence safety | Pinned gitleaks-current.sh and separate retained generated-evidence directory scan | Record separate outcomes. Redact generated secrets before retention; advisory policy unchanged, no new broad suppression. |
| Local/release rehearsal | local-dry-run.sh; separate release.yml dry_run=true,release_version,candidate_commit | Pending .28 version/Axis E/trusted provenance preparation; immutable dry-run already exempts main ancestry. No old-version substitution or pre-tag-ready claim. |

D28-09/10/11 and populated-open deferrals remain. Do not repeat frozen layout
sweeps or reopen erasure optimization. CUDA driver/toolkit stay unchanged.

## Resources and executor prerequisites

The source checkout has its own Python3.12/Node25.9 tooling. Feature builds use
`/media/coreyt/C8D47C11D47C03CA/data/linux-cold/fathomdb-s95-feature-target`
with206,392,377,344 initial free bytes, jobs2 and runner-owned debug0/incremental0.
Root NVMe remains the measured database/TMPDIR location. Sandbox read-only mounts
and denied namespaces/GPU probes do not describe the actual unconfined executor.
The idle checkout-owned debug cache was copied to cold storage and every file, mode
and symlink verified before reclaiming its original duplicate. Its logical path
remains unchanged through a symlink; relocation receipt records identities.
No shared cache is deleted. Never run timing concurrently with builds.

The standing Windows VM's old registration expired; its configuration remains
untouched. A distinct temporary runner may be provisioned from verified public
distribution binaries, excluding credentials/config/workspaces. Confirm exact
repository, labels, online/idle and executing runner identity. Remove only
task-owned registration/process/files after jobs and artifact collection finish.
No workflow trust restriction or persistent host security setting changes.

## AC011 before and after the repairs

All rows use the unchanged canonical AC011 workloads on windchill3/Linux x86_64
and local NVMe: sequential commits, 5-second warmup and 60-second measured window.
The three repair qualifications use `scripts/run-ac011-write-throughput.sh`;
the baseline row retains separately observed AC011a and AC011b results. Minimums remain 1,000/s for 1 KB and 100/s for 100 KB.
These are distinct qualification runs, not a paired causal experiment. The
instrumented timing/trace/profile rates are excluded from this table.

| Candidate stage | Exact measured source | 1 KB commits/s | 100 KB commits/s | Helper result |
| --- | --- | ---: | ---: | --- |
| Before the performance repairs | `86fb2924c3f10a6b0d4ff04fbd00b8ac7c67bf3d` | 999.651557 — FAIL | 307.292 — PASS | Individual original observations; retain the1 KB failure |
| Lazy edge-endpoint preparation | `13c6470b5b5700c61c68ce8d10257551fbd2c9ca` | 827.267649 — FAIL | Not reached | 101 |
| Compiled metadata-query reuse with fresh reads | `10ca015034f6c55c5c2193a75e1eb4056d5bb948` | 932.474344 — FAIL | Not reached | 101 |
| Transaction-owned visibility coalescing, format 37 | `6d580ec57692241c28a5113e374102cddc42e559` | 1,315.423 — PASS | 305.975 — PASS | 0 |

The original1 KB sample completed 59,986 commits in 60.006908962 seconds. The
next two samples completed 49,639 in 60.003555154 seconds and 55,950 in 60.001650826
seconds. Their failure diagnostics retain full elapsed precision. The current
passing samples report 78,932 commits / 60,005 ms and 18,364 commits / 60,018 ms;
passing durations are truncated milliseconds and rates rounded to three decimals.
Do not infer nanosecond precision or a separate measured percent speedup.

Current source bundle SHA256 is
`a827a61842a70a64044eb52c9c9f0d6d616607dc29d7421abfc0995681825b90`;
optimized executable SHA256 is
`38e50e6e00411c8c1f8d9c36f823713fb0ee787e33a96e73e40d51dff5791615`.
It uses opt-level 3, no debug assertions and the existing default/test-hooks route;
no migration hooks. The helper and performance tests are byte-identical to the
pre-repair baseline. Separate verification confirms both actual test executions,
helper/time exits 0 and applicable before/after/enclosing readiness. The Windows
VM was observed off by the parent; no separate VM-state receipt was retained.

Raw current logs/identity/readiness and the preserved executable are under
canonical `.28/target/slice95-ac011-evidence/visibility-coalescing/`. Earlier
canonical repair logs are under the same evidence root's top level and
`metadata-reuse/`; the original baseline is source-attributed in
[qualification-evidence.json](qualification-evidence.json). This throughput pass
does not substitute for final local/feature/security, native-platform/consumer,
other performance/reliability, fidelity or hosted CI qualification on format 37.
