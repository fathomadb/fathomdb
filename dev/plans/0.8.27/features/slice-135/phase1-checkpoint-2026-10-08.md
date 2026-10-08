---
title: FathomDB 0.8.27 Slice 135 — four-area Phase 1 checkpoint, 2026-10-08
status: PHASE1_CHECKPOINT_RECORDED_WITH_EXPLICIT_LIMITATIONS
target_release: 0.8.27
---

# Slice 135 Phase 1 checkpoint — 2026-10-08

This is the dated checkpoint for the four Phase 1 areas in the
[approved plan](plan.md): what matters, system latency, system robustness,
and logic/exception handling. It records measured evidence and unsupported
cells at exact candidate source `224e44c593c13d86ece648adabe445723db04070`
against 0.8.26 baseline `f99e002f0d2e4002f3694c9f8d4986b56089edaa`.
It is a diagnostic checkpoint, not a release-performance equivalence or
Slice 135 completion verdict. Dedicated gold-answer scoring in Phase 2 has
not started.

## Identity and audit boundary

- Git confirms both off-ladder landings, corrected embedder close
  `96796fe04` and Tegra install route `c23e2d23f`, are ancestors of the
  candidate. The working branch's runtime sources under `src/rust/crates`,
  `src/python/fathomdb` and `src/ts/src` match `224e44c59`; later Slice 135
  commits add measurement code, documents and one test-only type annotation. The candidate build checkout is the clean detached source at
  `/tmp/slice135-candidate-224e44`.
- The installed Python candidate wheel SHA-256 is
  `ee8b402f76377956034f900ef69f9e3c0296d79ea30d6728dc1283040266f85a`
  and native module SHA-256 is
  `1f213a700cf0c96b9997700d6e44ed7019b26fb91df1259bc6514fefb4a8d013`.
  The TypeScript Linux x64 GNU native addon SHA-256 is
  `5073e7ecc7dfef733da97cc5ff92be0f128f61741e1bc6ad05093d9d913b5aa2`;
  main and platform npm archive SHA-256 values are
  `8c8a97fbe26c541cddadfec7f9faef518102307f927ce6b4b095e5da721ca49d`
  and `8ce9b8e57b8f8ddf9adb3035822f14d61a4465a6da1e4ad7897d25c892998d6c`.
  Package metadata still says 0.8.26; source and artifact bytes determine
  candidate identity. The [TypeScript artifact audit](results/2026-10-08-ts-224e-candidate/README.md)
  and [Python installed-wheel audit](results/2026-10-08-python-capability-224e/README.md)
  verify those boundaries.
- The executable [broader Phase 1 bundle](phase1-224e-broader-protocol-v2.json)
  has SHA-256 `cd41cd0081778cf4e1e21cb2e18103408bcab661adcc1df9c4e1261f9a9500ce`.
  It binds the E01–E12, Python S01/S02/S02-L/S03, TypeScript S01/S02 and
  equal-frequency query-mix component protocols and independent auditors.
  The S02-L replacement was frozen before its timing; the original runner
  rejected an older wheel during preflight and collected no samples.
  The bundle names C01, GPU, Rust-baseline and TypeScript S03 omissions.
- Each paired receipt retains every valid sample, order, source/artifact
  identity, environment and resource record, semantic checks, independent
  recomputation and invalid-attempt disposition. The copied exact-source
  raw archives for Python S01/S02/S03/S02-L, TypeScript S01/S02 and engine
  E01–E12 remain local and untracked under their result directories, with
  verified `SHA256SUMS` manifests. Their linked audits and summary receipts
  are tracked. This is a retention limitation for a future clone of Git;
  the current worktree can recompute the reported cells.

## 1. What matters and Pareto path

The [query-mix protocol](phase1-python-query-mix-224e-protocol.json) fixes
ten installed-Python materialized query paths at equal frequency, drawing
500 audited calls per path, version and corpus size from separately prepared
S01/S03 real-database fixtures. The [recomputed ranking](results/2026-10-08-python-s03-224e-paired/phase1-query-mix-audit.json)
finds the smallest prefix exceeding 80% of observed elapsed call time:

| Corpus | Candidate elapsed-cost order to threshold | Share of all ten paths |
| --- | --- | ---: |
| 32 rows | Evidence, filter, vector, hybrid | 97.01% |
| 256 rows | Evidence, vector, filter, hybrid | 96.20% |

Four of ten paths dominate this equal-frequency proxy; it does not support
an assumed 80/20 split or a production-traffic frequency claim. The
[exact-source E01–E12 overlay](results/2026-10-08-e12-224e-coverage/README.md)
separately instrumented workload and selected tests: 20 selected binaries
hit 8,121 of 8,134 workload-hit engine lines and all 888 workload-hit
branch IDs. Two supplemental binaries hit all 13 selected-set line gaps.
The overlay measures selected route overlap, not global branch coverage or
oracle strength. The [deferred-identity mutation](results/2026-10-08-deferred-identity-mutation/README.md)
passed on exact source and failed at the intended assertion when the old
swallowed-row-error branch was restored in an isolated copy. That checks one
high-use search oracle against a plausible defect.

The [boundary attribution](results/2026-10-08-boundary-attribution/README.md)
keeps a roughly 0.39 ms synthetic engine vector stage separate from 10–12 ms
installed real-model vector calls; their percentage changes cannot be
subtracted or assigned to the same work. The S02 stage diagnostics and
close-memory receipt identify lifecycle leads separately. Neither the S01/S03
mix nor the E01–E12 synthetic set measures per-path CPU or queue wait, so
those two rankings are **unsupported** in this checkpoint. Whole-process CPU
and memory reports and an older D27 queue trace are contextual only. Severe
low-frequency paths are represented separately by projection recovery,
interrupted erasure, close, and FFI refusal tests below; their observed
frequency in real traffic remains unknown.

## 2. System latency and resources

All listed timings are unprofiled, paired 0.8.26/candidate diagnostics on
the same host within each protocol. Positive deltas mean a slower candidate
at that named boundary. Five pairs give a descriptive observed range, not a
significance test or equivalence proof. Host-only paging warnings remain in
the linked audits; measured-child swaps invalidate a block, and none were
accepted. Query p99 appears only where a cell has at least 1,000 valid
observations; lifecycle p99 is unsupported.

| Boundary | Exact-source result | Interpretation limit |
| --- | --- | --- |
| [E01–E12 engine](results/2026-10-08-e12-224e-paired/README.md) | Median five-pair p50: vector stage +15.67%, hybrid +14.54%, populated open +12.14% | Four warning-free query pairs, only one warning-free populated-open pair; synthetic provider and engine timer |
| [Installed Python S01](results/2026-10-08-python-s01-224e-paired/README.md) | Vector p50 -3.425% at 32 rows, -0.045% at 256 rows | 60,120 audited calls; 256-row change is near zero relative to pair variation |
| [Installed TypeScript S01](results/2026-10-08-ts-s01-224e-paired/README.md) | Vector p50 +2.3715% at 32 rows, -0.7761% at 256 rows | 60,120 audited calls; only two warning-free 256-row pairs |
| [Installed Python S03](results/2026-10-08-python-s03-224e-paired/README.md) | Pooled p50 evidence +2.668%/+2.642% at 32/256 rows; filter +2.165%/+0.952%; 256-row memory -15.228% | 500 calls per path/version/size; p99 unsupported; pair ranges in audit |
| [Installed Python S02](results/2026-10-08-python-s02-224e-paired/README.md) | Whole-sequence pooled p50 +0.287%, p95 +0.206% | 100 fresh-process sequences/version; two warning-free pairs; candidate peak RSS about 313–316 MiB vs 447–449 MiB baseline |
| [Installed TypeScript S02](results/2026-10-08-ts-s02-224e-paired/README.md) | Whole-sequence pooled p50 +3.454%, p95 +2.534%; five pair p50 deltas +2.772% to +3.675% | 100 sequences/version; two warning-free pairs; reopened-open median +158.891 ms is a stage lead, not an additive attribution |
| [Installed Python S02-L](results/2026-10-08-python-s02l-224e-paired/README.md) | `close()` pooled p50 +143.722%, p95 +132.091%; median pair p50 +140.575% | 100 cycles/version, no warnings; candidate opened-to-closed-idle PSS release +45,719 KiB vs baseline -33 KiB |

The S02-L close increase is real at its narrow timer and accompanies the
intended release of the engine-owned model before `close()` returns. It is
not itself a correctness failure; repeated close/reopen and persisted-state
assertions passed. The [off-ladder close receipt](results/2026-10-07-off-ladder-embedder-close/README.md)
and exact S02-L result cover the corrected landing. The
[Tegra route receipt](results/2026-10-07-off-ladder-tegra-route/README.md)
checks the pin/install/documentation route at its source, while a published
wheel install on the Jetson target remains a Slice 150 release check.

The [C01 qualification](results/2026-10-07-c01-qualification/README.md)
found the pinned LOCOMO corpus and nonempty Mem0/Qdrant volumes, but not
the exact external harness, config or output root; no matched native Mem0
speed ratio is claimed. A CUDA-qualified large-model host, a same-SDK
0.8.26 Rust baseline, exact TypeScript S03 timer and per-path CPU/queue
measurements were unavailable. Sequential paired runs with prescribed idle
gaps do not qualify throughput under sustained mixed load. The existing
bounded-contention receipts are source-specific context, not a substituted
exact-source throughput comparison.

## 3. System robustness

The [exact-source replay](results/2026-10-08-robustness-replay-224e/README.md)
passed 12 selected real-database tests; the separate
[crash/persistent-fault probe](results/2026-10-08-crash-permission-224e/README.md)
passed three, and the [erasure replay](results/2026-10-08-erasure-replay-224e/README.md)
passed three. Their retained logs contain fault points, expected/observed
state and recovery; linked auditors check the structured records. Earlier
five-run [persistent capacity](results/2026-10-08-persistent-sqlite-full-current/README.md),
[persistent provider](results/2026-10-08-persistent-provider-current/README.md)
and [provider/close](results/2026-10-08-provider-close-current/README.md)
receipts add repeated resource and tamper checks. Those earlier runs were on
source `3f29d649d`; between it and `224e44c59`, only `search.rs` and
`fusion.rs` changed among engine/PyO3/N-API source files. Their fault-path
behavior was replayed on exact source where named above; earlier timing and
resources stay labeled by their own source.

| Fault or schedule | Expected and observed state | Recovery and resource result |
| --- | --- | --- |
| Two writers plus two readers | Anchor readable during 211 of 213 reads while writers active; exactly 17 expected nodes after reopen | Bounded completion; file descriptors 4 before and after; integrity `ok` |
| SIGKILL before write, then after acknowledged write | First fresh reopen lacks uncommitted row; second contains acknowledged row | Both victims signal 9; integrity `ok`; process kill does not model power loss |
| One-shot pretransaction and projection-commit busy/storage/panic faults | Typed failure or truthful pending state; no partial canonical write or false durable terminal | Subsequent redispatch/drain and fresh reopen recover vector; six TC-91 tests pass |
| SIGKILL after failed projection commit and before queue cleanup | Canonical node durable; fresh reopen/drain makes vector ready; no false persisted failure | Victim signal 9; integrity `ok` |
| Persistent SQLite full, external busy lock and connection read-only refusal | Three typed `Storage` failures per selected persistent case; refused rows absent | Lift cap/lock/refusal, write recovery row and reopen exact state; integrity `ok` |
| Persistent provider failures through retry budget | Canonical row remains; terminal `Failed` and no vector survive reopen | Explicit rebuild reaches `UpToDate` vector and survives another reopen |
| Erasure precommit, postcommit and rotated telemetry sink | Primary rollback or truthful `ErasureIncomplete`; pending redaction survives fresh reopen; victim absent | Restore sink and retry: pending 1 to 0, control retained; integrity `ok` |
| Repeated close/reopen and installed FFI invalid strings | No reopened-state loss in 100 Python cycles/version; NUL and lone surrogate return typed Python/TypeScript validation errors without writes | Closed handles and fresh opens succeed; exact FFI [probe](results/2026-10-08-ffi-boundary-224e/README.md) retains counters |

The matrix is bounded, not a claim that every failure schedule was explored.
Unexecuted positions include OS filesystem permission changes, kill or
power loss inside an erasure transaction or WAL checkpoint, sustained
close/cancellation under concurrent load on the exact installed packages,
and all CUDA/provider/model variants. Temporary databases in the new Rust
replays were asserted directly and then deleted; their logs remain, whereas
the Rust SDK functional consumer retains its database. Connection-level
SQLite `query_only` is a write-refusal proxy, not an OS permission test.

## 4. Logic and exception handling

The full workspace gate is recorded below. Its static checks, type checks
and tests complement the exact-source line/branch overlay; a green gate
does not prove absence of logic faults. The [scoped boundary audit](logic-boundary-audit-2026-10-08.md)
and [row-site audit](logic-row-error-audit-2026-10-07.md) disposition the
confirmed search/graph/evidence row-error suppressions repaired test-first
before source `224e44c59`. The exact installed SDK and engine campaigns
were rebuilt after those repairs. The named deferred-identity mutant failed
at the expected semantic assertion, showing that one high-use regression
oracle detects its historical defect. The exact FFI negative-string probes
passed with typed errors and unchanged reopened state.

| Finding or boundary | Disposition |
| --- | --- |
| Current-schema swallowed SQL row errors in node/edge/vector hydration, deferred identity, importance/confidence and soft fallback | Confirmed defects repaired test-first; linked row/boundary audits name each real-database red/green receipt. No corresponding failure appeared in exact-source replay. |
| Legacy `rows.flatten()` branches | Supported open admits current schema 34; pre-step legacy branches remain historical/test-hook risks if admission changes. |
| Rank-stream `.ok()` | Intentional fallback to stable sort; malformed fallback rows propagate `Storage` in focused tests. |
| Dense-disabled reason poisoned-lock fallback | Refusal still occurs via atomic flag, but a poisoned lock may replace the precise reason with a generic reason; observability risk retained. |
| Query-vector JSON `.ok()` and guarded Python/N-API `unwrap` sites | Finite vector serialization and preceding guards make normal-path failure unlikely; exceptional allocation/panic and every exported path remain unproven, not declared impossible. |
| Rust panic to PyO3/N-API | Source wrappers use `catch_unwind`; earlier test-hooks builds passed representative AC-067 cases. Exact release-built artifacts omit force-panic hooks, so dynamic panic injection there is unsupported. |

Completed Rust and TypeScript workspace tests exercise codecs, projection,
recovery and round trips; the targeted erasure/projection fault probes and
one killed search mutant add behavior-specific evidence. The selected
coverage overlay does not enumerate every reachable/unreachable branch,
and no broad mutation score, full FFI export proof or whole-program panic
proof is claimed.

## Capability reconciliation, omissions and next work

The [44-operation register](phase1-capability-register.md) now links exact
candidate artifacts: [installed Python](results/2026-10-08-python-capability-224e/README.md)
and [installed TypeScript](results/2026-10-08-ts-224e-candidate/README.md)
each executed 41 selected positive paths with zero failed and three
provider/model unavailable; the [external Rust consumer](results/2026-10-08-rust-sdk-capability-224e/README.md)
executed 42, with two provider/model unavailable and zero supported gaps.
The Rust result is source-path and candidate-only. A selected operation
does not prove every filter, error precedence, cancellation, concurrency,
platform or model condition. No supported selected operation disappeared
to improve timing.

Ranked follow-ups from this Phase 1 evidence:

1. Repair the Python workspace gate: reconcile the Slice 130 declaration
   comparator with the documented later API changes through a reviewed
   successor oracle, and give the embed verifier's child process a stable
   import path. Rerun the full gate before any workspace-green claim.
2. Investigate TypeScript S02 reopened-open and whole-sequence overhead with
   matched stage and CPU/queue observations, then repeat under warning-free
   conditions. Its +2.772% to +3.675% five-pair p50 range is a descriptive
   lead, not a release gate.
3. Profile the exact engine vector/hybrid and populated-open leads with the
   same corpus, model and timer used by installed calls. Keep profile runs
   separate from unprofiled latency; do not infer whole-call loss from the
   synthetic engine stage alone.
4. Retain the close memory-release contract while investigating its roughly
   7.8 ms added installed-Python `close()` p50 and its interaction with
   reopened-open behavior. Test repeated close/cancel under contention.
5. Add consented per-path CPU and queue instrumentation and a qualified
   sustained mixed-load throughput cell; then revisit the four-path elapsed
   ranking with observed rather than equal-frequency usage.
6. Cover the remaining high-severity fault positions: OS permissions,
   in-commit erasure/WAL interruption and release-binding panic containment.
   Qualify the missing external Mem0 harness, CUDA/Jetson artifact route and
   provider/model cases separately.
7. Resolve raw archive publication/retention without weakening secret
   scanning. Current local archives and manifests preserve recomputation in
   this worktree; a clean Git clone has tracked audits and summaries only.

The full gate result below establishes the checkout's verification state.
This checkpoint closes the Phase 1 recording step with explicit unsupported
cells and residual risk. It does not close Slice 135, supersede accepted
release gates or authorize a Phase 2 quality claim.

## Verification

- Full workspace gate on committed evidence `b25bd8a41`: **exit 1** at the
  test step after 1,132 seconds. Lint, typecheck and strict security passed
  (zero security violations, blockers or downgrades). Of 186 registered test
  suites, 185 passed, including serial Rust, TypeScript and the repaired
  `steward-orient` suite; `test-python` failed. Python had 1,587 passed,
  30 skipped and three failed cases. This is **not** a full-workspace green
  claim. The [verification receipt](results/2026-10-08-phase1-verification/README.md)
  retains the exact gate output, Python diagnostics and gate-only Git exclude.
- The Python declaration comparator expected 1,131 pre-move declarations
  but found 1,132. Its exact set delta is two `search_frozen` signatures
  changing `pool_n: int=0` to `pool_n: int | None=None` plus the
  `DependencyTraceError` export. Commit `a8a0c74f0` made those intentional,
  [interface-documented](../../../../interfaces/python.md) changes after
  the Slice 130 snapshot. The old golden was not regenerated.
- Two `verify_embed_db` tests failed because their inspection subprocess
  could not import `eval` from its inherited Python path. A focused rerun
  with `PYTHONPATH` set to this checkout's `src/python` passed both tests
  (two passed, four deselected). The gate runner and child import path still
  need reconciliation; this scoped rerun does not make the full gate green.
- The gate-only `core.excludesFile` pointed at a local `/tmp` pattern for
  untracked Slice 135 raw-result files. It let the committed source satisfy
  Python's clean-worktree precondition while leaving those files retained
  and ordinary Git status unchanged outside the run.
- Scoped Markdown validator: passed after the report and plan update.
- Exact product diff: `git diff --quiet 224e44c59 -- src/rust/crates src/python/fathomdb src/ts/src`
  returned zero before this report; rerun after final commit. The sole `src/`
  difference is a `ClosureLookupV1` annotation in an existing Python test,
  added after Pyright rejected its two otherwise valid calls.
