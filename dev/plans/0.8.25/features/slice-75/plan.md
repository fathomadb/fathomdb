---
title: 0.8.25 Slice 75 — integrated release closure
status: READY
depends_on: 73
design: design.md
design_status: APPROVED
---

# Slice 75 plan

## Outcome

Close the non-publishing 0.8.25 implementation ladder on one unchanged product
candidate. Prove final-code integration, populated schema-26 upgrade, the
binding release performance and model obligations, installed Linux artifacts,
and the hosted native validation routes. Consume applicable Slice 71–73 evidence
instead of repeating those investigations.

Package production, the version cut, registry staging, tags, publication,
post-publication smoke, and merge to `main` remain outside this slice.

## Reconciliation of the draft

The original draft was compared with the completed Slice 71–73 records, the
current code and workflows, parent plans, assigned R25-75 proof package, and
the retained performance/platform evidence. The following changes are
approved subject to design review:

1. Replace `fast` + `heavy` + `all` with one `agent-verify --tier=all` run.
   `all` already contains 106 fast and 3 heavy suite labels.
2. Do not run `scripts/check.sh`; it repeats the Rust workspace with
   `AGENT_LONG=1` while omitting runtime Python/TypeScript, Clippy, and security
   coverage. Run only the distinct long tests and performance gates below.
3. Reject the draft 10k/50k × cold/steady bespoke workload. Four small
   real-database interaction tests cover the identified final-code seams more
   directly and with stable correctness assertions.
4. Add two explicit populated schema-26-to-33 tests. Per-step migration tests
   are supporting evidence, not a substitute for the prior-release upgrade,
   reopen, projection, and lifecycle path.
5. Run AC-076, AC-020, AC-021, AC-059b, AC-034a/b, AC-073, and AC-075 on the
   final product tree. Reuse Slice 71 AC-072 and 71B measurements only after a
   path-diff applicability check, with one final AC-072 confirmation in the
   isolated performance binary.
6. Treat synthetic AC-019/AC-013b as descriptive only. One real EU7 run owns
   both the AC-073 mixed-tail verdict and AC-075 real vector-stage fidelity.
7. Make live model execution fail closed through positive counts. Generic
   network-suppressed suites and a green hosted model job do not close this
   cell.
8. Build Linux x64 candidate wheel/N-API bytes once and reuse them for
   installed smoke and the zero-spend GLOBAL-01 native-search witness.
   GLOBAL-01 remains data-plane evidence and makes no answer-quality claim.
9. Require final-head artifact rows for Linux x64/ARM64, macOS x64/ARM64,
   Windows x64 CPU, and Jetson/Tegra Python CUDA. Start and confirm the local
   Windows GitHub runner before workflow dispatch; use GitHub macOS runners;
   run the Tegra route on the Jetson at `10.83.10.13`.
10. Replace the stale Tegra workflow's hard-coded 0.8.24 branch/version with a
    release-state/version-derived contract that works for 0.8.25 and future
    releases. Publishing the `+tegra` wheel remains outside Slice 75. Tegra npm,
    musl, ARMv7, Windows ARM64/ia32, and Windows CUDA remain out of scope.
11. Restore the parent proof package's nonpublishing Rust/CLI rehearsal:
    release-build the workspace and CLI, package the three independently
    packageable leaf crates, and smoke the built CLI. Dependent-crate packaging
    remains deferred until the separately authorized version cut and ordered
    registry rehearsal; otherwise Cargo resolves published dependencies rather
    than the candidate.
12. Prove runtime floors without rebuilding artifacts: reuse one Linux x64
    abi3 wheel and N-API package for Python 3.10, 3.11, and 3.12 plus Node 18
    and the release-current Node runtime.

No global acceptance identifier or public API is added.

## Requirements

- **R75-1 — exact candidate:** Every current result identifies one product-tree
  SHA and clean state. Later receipt-only commits must prove the product tree
  unchanged.
- **R75-2 — executable evidence:** A checked-in manifest fixes every required
  command, controlled environment, timeout, feature set, expected count,
  marker, threshold, and permitted exclusion. Missing, zero-test, uninventoried
  skip, ignored, timed-out, or unknown required results fail closure.
- **R75-3 — final interactions:** Real-database tests cover 71B batching and
  visibility coalescing plus AC-072 deferred hydration/linear fusion when
  dependencies, frozen reads, lifecycle/erasure, custom triggers, projection
  readiness, and concurrent mutation are present.
- **R75-4 — prior-release upgrade:** A populated schema-26 database upgrades
  through exactly steps 27–33, reopens idempotently, retains canonical/search
  and projection behavior, and supports new provenance/dependency, frozen-read,
  lifecycle, erasure/recreate, and readiness operations after upgrade.
- **R75-5 — release gates:** The deduplicated local matrix proves full default
  coverage, selected long reliability, binding text/vector latency, reader
  concurrency, real mixed-tail behavior, and real vector-stage fidelity.
- **R75-6 — actual models:** Live-model tests execute with the intended cached
  model and with `FATHOMDB_SKIP_NETWORK_TESTS` absent. A fallback skip cannot
  pass.
- **R75-7 — installed artifacts:** Candidate Linux x64 Python and N-API
  consumers resolve only installed artifact bytes and perform normal
  open/write/search/dependency/frozen-read/close behavior. The packaged
  GLOBAL-01 witness calls `Engine.search` exactly once.
- **R75-8 — platform truth:** Linux x64/ARM64, macOS x64/ARM64, Windows x64
  CPU, and Jetson/Tegra Python CUDA are tested at the final head. CPU, CUDA/CE,
  Tegra, ABI, and package claims are not conflated.
- **R75-8a — runtime floors:** The same Linux x64 wheel runs under CPython
  3.10, 3.11, and 3.12, and the same N-API package runs under Node 18 and the
  release-current Node version. Runtime cells do not rebuild artifacts.
- **R75-9 — reuse:** Retained evidence records its original candidate, receipt
  digest, semantic scope, relevant input set, and diff-to-final result. A
  changed relevant input invalidates only the affected cells.
- **R75-10 — non-publishing boundary:** Closure reports an implementation
  candidate, not published 0.8.25 packages. It performs no version cut, tag,
  registry mutation, release creation, or merge to `main`.

## Sealed execution matrix

All commands run from a clean final candidate. The receipt retains exact argv,
controlled environment, return code, duration, raw-log SHA-256, counts, and
required markers. Commands below are the only broad/long campaigns.

| Cell | Exact command or route | Required result |
| --- | --- | --- |
| Default tree | `timeout 7200 env AGENT_VERBOSE=1 bash scripts/agent-verify.sh --tier=all` | 109 registered/ran/passed; 0 failed/suite-skipped/suite-excluded. The exact nested network-body exclusions are visible and discharged only by Live models; any other nested skip fails. |
| Final interactions | `timeout 900 cargo test -p fathomdb-engine --features operator,test-hooks --test slice75_feature_interactions -- --nocapture --test-threads=1` | 4/4 passed, 0 ignored/skipped. |
| Schema 26 upgrade | `timeout 900 cargo test -p fathomdb-engine --features operator,test-hooks --test slice75_schema26_upgrade -- --nocapture --test-threads=1` | 2/2 passed; before=26, after=33, steps 27–33, then zero steps on reopen. |
| AC-021 | `timeout 180 env AGENT_LONG=1 cargo test -p fathomdb-engine --features operator --test lifecycle_reliability ac_021_zero_sqlite_schema_warnings_under_concurrent_reads_and_ddl -- --exact --nocapture --test-threads=1` | 1/1; 60 s; zero `SQLITE_SCHEMA`; projection add/drop plus rebuild operations observed. |
| AC-059b | `timeout 180 cargo test -p fathomdb-engine --test cursors ac_059b_write_cursor_is_satisfied_by_projection_cursor_and_queryable -- --exact --nocapture --test-threads=1`, followed by `timeout 180 env AGENT_LONG=1 cargo test -p fathomdb-engine --test cursor_read_after_write projection_cursor_bounds_observed_row_count -- --exact --nocapture --test-threads=1` | Direct contract 1/1 plus companion race 1/1; 1,000 race reads; zero cursor violations. |
| AC-034a/b | `timeout 1800 env AGENT_LONG=1 cargo test -p fathomdb-engine --test durability_soak ac_034a_and_b_power_cut_zero_corruption_and_p99_lost_commit -- --exact --nocapture --test-threads=1` | 1/1; 100 trials; zero corruption; p99 lost commit <=100 ms. AC-034c remains explicitly unavailable because its VM/sysrq fixture does not exist. |
| Perf build | `cargo test --release --no-run -p fathomdb-engine --test perf_gates` | One successful release build reused by the next three cells. |
| AC-076 | `timeout 3600 env -u AC_FULL_SCALE AGENT_LONG=1 AC012_CORPUS_N=10000 cargo test --release -p fathomdb-engine --test perf_gates ac_012_text_query_latency_on_fts5_path -- --exact --nocapture --test-threads=1` | 1/1; 1,000 measured queries; p50 <=20 ms and p99 <=150 ms. |
| AC-072 confirmation | `timeout 3600 env -u AC_FULL_SCALE -u AC013_SCALE_TREATMENT -u AC013_DRAIN_TIMEOUT_MS AGENT_LONG=1 AC013_CORPUS_N=10000 AC013_VECTOR_DIM=384 cargo test --release -p fathomdb-engine --test perf_gates ac_013_vector_retrieval_latency -- --exact --nocapture --test-threads=1` | 1/1; 10k/384d/1,000; p50 <=80 ms and p99 <=300 ms. |
| AC-020 | `timeout 600 env AGENT_LONG=1 cargo test --release -p fathomdb-engine --test perf_gates ac_020_reads_do_not_serialize_on_a_single_reader_connection -- --exact --nocapture --test-threads=1` | 1/1; concurrent <= `1.5 * sequential / 8`. |
| AC-073 + AC-075 | `timeout 10800 env -u FATHOMDB_SKIP_NETWORK_TESTS -u EU7_FORCE_FULL_RECOMPUTE AGENT_LONG=1 EU7_N_VALUES=7667 EU7_QUERIES=100 EU7_BOOTSTRAP=1000 EU7_LATENCY_SAMPLES=1000 EU7_STRESS_PER_THREAD=250 FATHOMDB_EU7_OUTPUT=<RUN_DIR>/eu7.json cargo test --release -p fathomdb-engine --features operator,default-embedder --test eu7_real_corpus_ac eu7_real_corpus_ac_validation -- --exact --ignored --nocapture --test-threads=1` | Real corpus >=7,667; one body executed; AC-073 stress p99 <= max(10x same-run baseline p99, 150 ms); AC-075 vector-stage recall CI high >=0.90; no skip marker; output written only under the run directory. |
| Live models | Manifest cells `model-cache`, `model-ts`, `model-engine`, `model-python`, and `model-cli` | TypeScript 33/33 with per-file counts 2/1/5/2/11/6/6, engine 9/9, Python 13/13, CLI 2/2; no skipped tests; model identity/cache digest retained. |
| Rust/CLI package rehearsal | Manifest cells `rust-release-build`, `rust-leaf-packages`, and `cli-installed-smoke` | Release workspace builds; all three leaf crates package normally; the built CLI opens, integrity-checks, and closes a fresh database. Installed SDK/GLOBAL cells own write/search. No registry write or dependent-crate substitution is claimed. |
| Linux artifacts | Manifest cells `linux-artifact-build`, `linux-artifact-current-smoke`, and `linux-runtime-floor-smokes` | Installed wheel and N-API hashes/paths retained; source fallback absent; Python write/search, dependency, and frozen-read markers pass; the fixed 13-module/180-test N-API set supplies the broader feature sweep; the same artifacts pass Python 3.10/3.11/3.12 and Node 18/current lightweight smokes. Bytes are pre-version-cut candidate artifacts, not production packages. |
| Linux x64 CUDA package | Manifest cell `linux-cuda-package` | A canonical package-set manifest binds the wheel, N-API main/platform packages, and CLI archive hashes to the checked-out SHA and verified CUDA preflight; installed CPU, auto, and forced-CUDA paths pass, with Python/N-API embed and rerank model forwards plus verified RTX 3090 allocation; CLI GPU/reranker doctor paths pass from the sealed archive. Slice 72 CE evidence supports reranker performance but does not replace this combined package cell. |
| GLOBAL-01 | Manifest cell `global-01-native` | Candidate-artifact Python runs the resolved Slice-75 config naming the built CLI; exactly one `Engine.search`; expected source rank 1; recall@3 and reciprocal rank 1; no network/model/spend. |
| Windows runner | Manifest cell `windows-runner-preflight` | The VM is started; runner labels are exactly `self-hosted`, `Windows`, `X64`, and `windchill3-windows-11`; it is registered to `fathomadb/fathomdb`, online, and its service is running before dispatch. Missing or wrong registration stops the matrix. |
| Hosted native validation | Manifest cell `hosted-native-validation` | Linux x64/ARM64 and macOS x64/ARM64 GitHub rows pass. Windows x64 CPU is explicitly routed to the confirmed self-hosted labels and passes installed wheel+N-API smoke; applicable Slice 73 13-module/180-test evidence is retained. |
| Jetson/Tegra | `gh workflow run jetson-tegra-cuda-evidence.yml --ref release/0.8.25 -f candidate_sha=<FINAL_SHA> -f candidate_version=0.8.25 -f publish_to_pages=false` after making the workflow release-generic; runner host is `10.83.10.13` | Exact SHA checkout; a declared staging-only version overlay creates one `fathomdb-0.8.25+tegra-*-linux_aarch64.whl` without dirtying source; wheel installs; CPU, auto-CUDA, and forced-CUDA processes pass; forced CUDA records verified Orin allocation/model-forward evidence; no publication job runs. |
| MkDocs | `timeout 600 .venv/bin/mkdocs build --strict` after dependency preflight | Documentation and navigation build with zero warning/error and no dependency skip. |
| Exact-head CI | Manifest cell `hosted-ci` | Every required pull-request job at the frozen SHA concludes success with no unauthorized skip; advisory jobs and every path-conditioned job are enumerated. Hosted security supplies the strict AC-037 route when local user namespaces are unavailable. |

`scripts/check.sh`, separate `fast`/`heavy` runs, synthetic AC-019 as a
verdict, paid GLOBAL execution, 100k/1M tracked performance tiers, and an
exhaustive scale-by-feature-by-CUDA matrix are excluded.

## Retained evidence and platform map

- Slice 71 AC-072 three-repetition and 71B write receipts remain valid when the
  final product diff does not touch their declared engine/schema/fixture inputs.
  Any Slice 75 product change reruns the affected focused guard before the
  final matrix.
- Slice 72 CE CPU/CUDA evidence remains valid only when model bytes,
  reranker/search code, build features, workers, locks, and receipt tooling are
  unchanged. Its four performance ratios must remain <=1.10.
- Slice 73 remains the deep Windows Node/N-API receipt when its package and
  runtime inputs are unchanged; the exact-head hosted Windows smoke is still
  mandatory.
- The five hosted native validation rows are Linux x64/ARM64, macOS x64/ARM64,
  and Windows x64. Shipped claims are classified separately as Linux x64 CUDA,
  generic Linux ARM64 CPU, macOS CPU, Windows CPU, and Jetson/Tegra Python CUDA.
  Python abi3-py310 and Node >=18 are exercised using the same built bytes.
- Tegra is mandatory for 0.8.25 and future releases. Its wheel keeps the
  `fathomdb` distribution and `+tegra` local version on the first-party index;
  Slice 75 validates but does not publish it. Tegra npm remains impossible and
  out of scope.

## TDD and implementation

1. **RED:** Extend the already-registered agent-tier contract suite with
   manifest-validator tests that reject missing cells, relaxed
   limits, wrong counts, skips/zero tests, uncontrolled environment, stale
   retained evidence, source-installed artifacts, and false answer-quality
   claims. This keeps the aggregate suite-label count at 109. Add failing
   final-interaction and schema-26 tests for any exposed gap. Preserve the RED
   commit.
2. **GREEN:** Implement the smallest manifest validator, focused execution
   helpers, and test fixtures. Amend AC-021 and EU7 only where necessary to make their registered
   protocols executable and fail closed. Generalize the Tegra workflow and its
   version-overlay builder contract. Add deterministic Windows/hosted/Jetson
   helpers that capture exact service, PR, workflow, run, job, matrix, and SHA
   identities. Add the Slice-75 installed cross-SDK and GLOBAL resolved-config
   bindings. Do not change public APIs or performance limits.
3. **REFACTOR:** Remove duplication, run formatting plus the affected focused
   tests, and obtain independent code review.
4. Freeze the product candidate and execute the matrix. After a narrow fix,
   rerun only cells whose declared inputs changed plus the manifest validator;
   run the single default-tree gate once on the final product tree.
5. Obtain separate evidence verification from retained results. Update the
   status and release state only after every required cell passes.

## Acceptance criteria

1. Plan/design review passes and the executable manifest validates.
2. Four final-interaction tests and two schema-26 upgrade tests pass at the
   frozen product SHA.
3. Default tree reports 109/109 suites with no suite skip/failure/exclusion;
   only the inventoried nested network bodies are excluded and Live models
   executes them separately.
4. Selected long reliability cells pass their exact durations/counts; AC-034c
   is recorded unavailable, never as a pass.
5. AC-076, AC-072, AC-020, real AC-073, and real AC-075 meet unchanged limits.
6. Live model tests have all expected positive counts (33/9/13/2) and no skip.
7. Installed Linux artifact smoke and the zero-spend packaged GLOBAL-01 native
   witness pass without source fallback or answer-quality claims.
8. All five hosted native validation rows, the separately classified shipped
   CPU routes, Linux x64 combined CUDA package, exact-head Jetson/Tegra CUDA,
   MkDocs, and full required hosted CI pass; each retained Slice 71–73 receipt
   passes applicability validation.
9. Independent code and evidence reviews report no unresolved material
   finding.
10. The committed Slice status and generated release-state views agree; the
    worktree is clean and temporary evidence/build roots are removed.

## Stop conditions

Stop closure for a changed/dirty product candidate, missing corpus/model/cache,
required skip or zero-test result, relaxed threshold, unasserted real AC-073,
schema-upgrade loss, source fallback, stale receipt, platform contradiction,
or unresolved material review finding. Record the exact blocker; do not replace
it with synthetic, historical, or descriptive evidence.
