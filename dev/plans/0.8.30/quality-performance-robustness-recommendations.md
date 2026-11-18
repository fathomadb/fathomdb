---
title: 0.8.30 recommendations for correctness, performance, latency and robustness
date: 2026-11-18
status: PROPOSED
target_release: 0.8.30
---

# 0.8.30 quality and delivery recommendations

The owner directs 0.8.30 toward correctness, performance, low latency and
robustness, with a bug hunt, testing-method review and CI/CD efficiency review.
The [0.8.29 retrospective](../../notes/0.8.29-retrospective.md) supplies the
observations behind these recommendations. The purpose is fewer real defects
and faster useful feedback, with less repeated work.

This is a proposed technical agenda, not an activated release plan, new list of
mandatory gates, run authorization or promise to implement every item. Reconcile
it with the existing [feedback/reset/S50 allocations](release-followups.md) and
[fork-maintenance allocation](fork-maintenance-plan.md). Those placements are
not cancelled here. Select detailed work for the quality objective rather than
letting an upstream rebase, new ABI or architecture matrix expand automatically.
CUDA13 delivery remains in 0.8.31; real resets still require concrete authority.

## Recommended order

| Order | Work | Useful result before expanding |
| --- | --- | --- |
|1 | Fix diagnostic loss and the known normal-publication defects. Start the small deterministic bug-hunt cases below in parallel. | Failures are attributable; ordinary release completion no longer depends on a stale token or an immediate consistency assumption. |
|2 | Establish a small, source-bound.30 baseline on representative existing workloads. | Correctness oracles, service/queue latency and resource measurements describe the same selected source and configuration. |
|3 | Resolve high-risk correctness uncertainties; profile the actual hot paths. | Minimized failing cases or explicit unresolved hypotheses; no optimization accepted merely because a component once looked expensive. |
|4 | Make targeted fixes or performance changes with genuine RED→GREEN and reference checks. | Correct results, lifecycle and tails remain qualified on the changed route. |
|5 | Run the required coherent-phase full verification once, then affected recovery and final installed-artifact/release checks. | Completed scope with original failures/skips retained and no passing-campaign replay for metadata changes. |

Do not recreate all.29 matrices before beginning useful work. A small early set
should include concurrent natural writes, a100k hybrid/SDK case, malformed open
and wire values, cancellation/close and an actual recovery boundary. These are
risk-first sentinels, not a replacement for a later selected contract or full
release qualification. Use existing harnesses and expand only where a concrete
risk, change or accepted support claim warrants it.

## Correctness and robustness bug hunt

A bug-hunt report should say **confirmed defect**, **not reproduced**, **expected
contract behavior**, **operator/harness failure**, or **unresolved observation**.
Attach the smallest reproducer and intended outcome; do not promote every
suspicious code pattern into a release-blocking defect.

| Target and existing evidence | Proposed probe | Oracle and scope limit |
| --- | --- | --- |
| S50 natural-write score observation, sample860/query360/op9971 | First inspect the preserved databases/report. For a future bounded reproduction, record only the relevant vector candidates/distances/ranks, text ranks, tie order, projection cursor/snapshot and selected route. | Separate fixed-state exactness from allowed concurrent-state movement. Both original differing operations used reference routing. Async projection remains unproved; add no general tolerance or automatic160-cell replay. |
| Record identity and feedback, [S50 census](../0.8.29/features/slice-50/census.md) | Distinct records with identical bodies; repeated public refs across revisions/pages; same record contributing through both arms; mixed valid/ambiguous feedback. | Assert stored-record identity, ranking/provenance and zero partial feedback writes. Preserve selected context-pack duplicate refusal. Richer response labels require their already-allocated design decision, not a silent ABI change. |
| Coverage, spans and snapshot publication, [stored coverage tests](../../../src/rust/crates/fathomdb-engine/src/stored_coverage_tests.rs) | Compose local rollback, external commits, failed capture publication, erase/rebuild/reopen, pinned-reader changes and UTF-8 boundaries. | Check exact known/unknown/truncated populations, `R=K+U`, byte/fraction arithmetic, unknown versus zero, same-reader ownership and bounded SQL work. An internally consistent SQLite file alone is insufficient. |
| Open admission and FFI error paths, [S160 admission proof](../0.8.29/features/slice-160/status.md#typed-open-admission-fix-and-current-artifacts) | Corrupt profile BLOBs/revisions/dimensions; malformed null/decimal/i64/u128 values; repeated refused opens followed by a healthy separate-process reopen. | Preserve typed kind/stage/hint/anchor and error precedence; no leaked locks or partial writes. Reuse existing installed Python/Node cases and exact wire controls. |
| Queue, cancellation and close, [S70 test map](../0.8.29/features/slice-70/test-map.md) | Deterministic barriers around admission, queued cancellation, started timeout, close/reopen and publication. Exercise aliases and multiple engines. | Distinguish overload, absent provider, started timeout and actual closed state; no cross-engine hook theft, stranded waiters, partial mutation or shutdown hang. Avoid speculative queue rewrites. |
| Checkpoint/recovery composition, [S100 execution](../0.8.29/features/slice-100/execution.md) | Target changed commit/checkpoint boundaries using the existing child and observer controls; verify preserved main/WAL/SHM and reopen behavior. | Confirm actual delivery at the intended live point, then canonical/projection/queue/survivor state. A healthy run or MI timeout is not fault-delivery proof. Process crash does not claim power-loss recovery. |
| Known-loss CUDA lifecycle, [held F9/F10/F11 cases](../0.8.29/features/slice-110/test-map.md) | Retain 0.8.29 healthy/OOM/software-loss proof; select separately authorized real-reset close/last-clone/normal-exit cases when venue and bounds are approved. | No forward or fresh allocation after terminal loss; safe cleanup and fresh-process recovery on the actual selected venue.3090 is not Orin proof; do not reset the host or change R580/primary12.6. |
| Default Node/native growth, [S130 status](../0.8.29/features/slice-130/status.md) | Use the existing sampler to distinguish JS heap, native live ownership and allocator reservation under the actual default profile. | Retain default FAIL/OFF_NOT_RUN. The instrumented semispace4 pair is separate evidence; no forced-GC or silent heap-flag change to manufacture a plateau. |
| Measurement operators, [S30 execution](../0.8.29/features/slice-30/execution.md) | Late failure/report cancellation, resumed completed members, small output limits, zero-length WAL/SHM companions and descendants surviving leader exit. | Correct measurements/charges, original exits, no lost completed work and certain owned cleanup. Test measurement correctness and actual hazards, not a new tamper/path-leak framework. |

For confirmed defects, preserve a human-intent regression that fails before the
fix. For non-reproductions, retain the scope and stop; do not broaden until the
hypothesis names a specific missing state or schedule. Keep original failures
when a focused recovery passes. A later green result is not a diagnosis of an
earlier unknown Rust101 or missing optional npm binary.

## Performance and low-latency work

### Measure the current route before choosing an optimization

Use the existing P/G/S50 adapters, SQL-work controls, M lifecycle/output-limit
cases and installed consumers. Record the selected source, features, native
artifact, data/query set, scale, view/readiness state, warm/cold policy and
instrumentation profile once with the result. Existing source-scoped evidence
can guide the investigation; it does not become a new 0.8.30 timing observation.

Separate queue wait, database service, eligibility/visibility checks, vector/text
retrieval, fusion, hydration, reranking and serialization. Report end-to-end
latency as well as components, including p50/p95/p99, timeouts/refusals and
throughput. Compare like routes: provider-free P1 at100k and SDK hybrid at100k
are not interchangeable baselines. Keep snippet-enabled and snippet-disabled,
text/structured and cold/warm measurements distinct.

| Opportunity | Why investigate | Decision rule before implementation/promotion |
| --- | --- | --- |
| Direct eligibility/visibility scan | The 0.8.29 observed all-true scan cost48.336–49.223 ms, roughly32% of its read cohort. | Profile current route frequency and cost. Prefer eliminating repeated work or improving query shape before selecting another cache; require same-snapshot output/eligibility correctness. |
| Cache HIT/MISS/invalidation | Repeated-read prototypes did not establish natural-write benefit. Source validity may change with time at unchanged generation/nonce. | Measure key construction, HIT and MISS overhead, invalidation and tail latency under actual read/write schedules. Do not adopt a generation-only key that changes semantics. |
| Winner hydration and materialization | S50's two100k hybrid repeats favored winner hydration by about 20 ms in mean paired latency; text deltas were mixed. | Preserve exact result identity and affected snippets/provenance. Optimize a demonstrated current bottleneck; do not infer universal gains from those two cases. |
| Serialization and allocation | S60's overhead defect was fixed by cached size accounting and avoiding clones; final wire bytes, not an intermediate object, define the budget. | Profile bytes, copies and temporary allocations on current public formats. Preserve prefix/zero-progress, Unicode, error and budget semantics. |
| Concurrency and resource pressure | Busy-host throughput failures and Node profile differences make unpaired observations misleading. | Separate queue saturation, CPU contention, I/O, heap behavior and native ownership. Evaluate useful throughput and tails together; do not hide timeout/error rate behind a faster median. |

Run on the ordinary shared CPU host as directed; do not introduce a quiet-machine
or CPU-controller prerequisite. For comparisons, use matched inputs/environment
and paired/interleaved old/new order with enough repetition to assess variability.
Keep contemporary host context, but do not claim it proves causality. Necessary
GPU exclusivity, foreign-process observation and owned cleanup remain required
for GPU work. A cooperative lock is not a guarantee against unrelated software.

Retain existing accepted numerical criteria unless explicitly reviewed. The
1495.233/1482.090 writes/s pair did not authorize a lower 1000/s threshold; tracked
AC083 targets are not binding gates. Do not create a universal latency percentage
or statistical noninferiority claim from a convenient run. Prefer a small
hypothesis-driven comparison over full-matrix calibration or a new sweep by default.

## Testing approaches to retain, strengthen and reduce

Apply the [quality testing roadmap](../../notes/fathomdb-quality-testing-roadmap.md)
with a narrower emphasis on FathomDB-owned composition:

| Method | Use in 0.8.30 | Avoid |
| --- | --- | --- |
| Stateful/property testing | Real-store write/supersede/erase/project/reopen sequences; cursor/budget/filter boundaries; codecs, projection and round trips. Retain minimized seeds and intended invariants. | Replacing real database integration with an in-memory mock, or counting a trivial property scaffold as meaningful coverage. |
| Independent differential/metamorphic checks | Small hand-understood fixtures, disabled-optimization/reference routes, filter-before-cap cases and duplicate-body/distinct-record cases. | Two paths sharing the same faulty predicate as the sole oracle, regenerated goldens or broad exactness claims across moving snapshots. |
| First/persistent failure injection | Existing commit/queue/provider/capture seams and owned child-process crash controls. Validate post-state, reopen, handles and exit. | Whole-engine allocation/VFS fuzzing, unbounded OOM or power-loss claims where the failure model was not tested. |
| FFI and installed consumers | Boundary values, malformed variants and native error paths in Rust/Python/TS, plus actual package loader/floor/lifecycle checks where affected. | Treating conversion-only tests as proof that native dispatch, timeout delivery or packaged loading ran. |
| Targeted branch/mutation checks | A few plausible wrong-order, wrong-condition or missing-cleanup defects in hot/critical paths. Keep cases that reveal a real oracle gap. | A whole-workspace coverage percentage, thousands of repetitive workflow-text mutations or new test-only macros throughout a moving design. |
| Expensive scientific/model checks | Reuse valid 0.8.29 credit by actual source/feature/fixture/artifact scope. Repeat only the changed claim; run priced episodes only when the selected interface/behavior question needs them and authority exists. | Replaying the paid72, all160 native members or a full platform matrix after a status/comment/helper-only edit. |

For the testing review, inventory each expensive suite's claim, recent duration,
confirmed defects found, known misses and overlap with other suites. Use existing
runner results and regression references; do not create a new receipt system.
Classify product, harness, environment and administrative failures separately.
A useful prioritization measure is actionable defect yield per runtime and
maintenance effort, not test/commit count. Unknown cost or yield stays unknown.

Use scoped lint/type/test feedback while changing code, then one stable,
coherent-phase full verification as required. Complete an interrupted unchanged
phase by its unrun/failed registrations. Reopen passed scope only when a change
invalidates it. Actual release checks and registry consumers remain distinct
from inherited source proof. Do not fabricate a RED for an already-correct control.

## CI/CD efficiency recommendations

Routine CI should remain informative and proportional for this single-maintainer
repository. The owner has not requested merge queues, required-check expansion,
nightly campaigns or soak periods. This does not weaken the separate approval,
exact-tag, reviewer, worker-access or selected-GPU controls of production release.

| Priority | Concrete change and existing seam | Evidence that it helped |
| --- | --- | --- |
| P0 | Preserve full failed-suite diagnostics and outcomes before collect-all continues; use [agent-output](../../../scripts/lib/agent-output.sh), [suite runner](../../../scripts/lib/agent-suite-run.sh) and existing spill collection. Keep capture/redaction bounded and preserve other callers' fail-fast behavior. | A deterministic failure beyond200 lines remains attributable when a later child is cancelled; original exit, completed outcomes and unrun tail survive. Do not depend solely on a late `if: failure()` upload after a60-minute job. |
| P0 | Make normal [release promotion](../../../.github/workflows/release.yml) use the qualified OIDC mechanism from [the recovery workflow](../../../.github/workflows/promote-npm-release.yml); share the implementation. | Normal finalization promotes only the main package to latest and completes GitHub Release without a manual recovery dispatch or legacy NPM_TOKEN. Use the supported pinned CLI, not an unverified latest upgrade. |
| P0 | Add bounded registry visibility verification after mutation, printing expected and observed values. Make npm smoke retain install diagnostics, optional dependency versions and expected native path. | A fixture returns an old tag twice then the target; verification succeeds within its bound. Persistent mismatch fails. A missing native binary remains failure, with enough output to distinguish metadata, compatibility and transport causes. No blind retry of runtime/data-corruption assertions. |
| P1 | Optimize [the Windows workflow checker](../../../scripts/tests/test_windows_wal_attribution_ci_job.sh): parse relevant input once, remove recursive full rescans and retain meaningful mutation cases. Reclassify suites by actual duration. | The856.43 s static-check hotspot drops substantially while known invalid workflows still fail and real Windows WAL runtime tests remain. Measure the existing suite rather than add a benchmark campaign. |
| P1 | Split developer-tool setup from native construction in [owned Python setup](../../../scripts/setup-python-owned.sh) and bootstrap paths. Reuse an owned, matching artifact within a job/checkout where valid. | Tool-only preparation starts no native wheel build. Source/lock/features/interpreter/build-input changes invalidate reuse; another checkout's editable bindings are never borrowed. |
| P1 | Profile the verification DAG and separate genuinely slow Rust/native work from cheap structural checks. Use durations already emitted; consolidate duplicate package/platform builds when producer identity permits. | Reduced queue/setup/repeated-build time without dropping a selected claim. Build-once/promotion needs exact trusted source/build-input/artifact linkage; unknown identity falls back to a build, not an exemption. |
| P1 | Match local hooks and ordinary CI to the changed surface: Markdown/state for docs, actionlint for workflows, Rust checks for Rust/build inputs, affected platform jobs for platform changes. Retain required final release checks. | An administrative edit does not trigger a cold native matrix. Source changes still exercise the affected contract; a local passing scope is never mislabeled a full-workspace result. |
| P1 | Inspect matching registered workers and local services before enabling/approving the exact production job; consolidate existing owned cleanup. | An old runner168 equivalent is surfaced before it can claim the job. Only the selected worker runs; descendants and owned containers are quiescent before the GPU lease is released. Keep reviewers and exact-tag routing unchanged. |
| P1 | Carry the implemented Pages lifecycle fix and test completion rendering as normal state transitions. Keep one active/published selector and one current status. | Active denial cannot fall back to historical authorization; malformed/no-active/completed states behave explicitly. A completed ladder renders truthfully without null pointers or an empty-view workaround. |
| P2 | Consolidate timing on the existing monotonic helper and distinguish queued/setup/execution/reporting time. | Clock adjustment cannot produce a negative elapsed duration such as the observed−650 ms. Completion estimates describe remaining critical-path work and GPU release separately. |

These are implementation candidates, not extra release gates. First fix known
failures and measured large costs; avoid a general CI rewrite. Pinning and cache
identity should use existing mechanisms, not a new signing/attestation service.
Do not bypass hooks or force-move a published tag. A failed post-publish consumer
gets a bounded, justified recovery against the same immutable version, not a
rebuild/republication tour.

## Keep the process small

Update the existing normal release path when recovery exposes a defect; do not
make a permanent second manual route the expected workflow. Reconcile old memory
and current repository instructions, especially force-retag advice, mandatory
rehearsal language, stale settings snapshots and full-gate frequency. Preserve
owner preferences separately from the current repository/runner configuration.

One current controller should name completed, remaining and blocked work, its
source subject and the next useful action. Historical records retain actual
subjects and raw failures. Do not append four “current” summaries to reconcile
one changed fact. Review coherent design/final changes independently; avoid
per-helper approval and model-review loops. Reactivate idle agents explicitly
when assigning work rather than silently waiting on a message-only delivery.

Use `/mnt/data/` for owned build/scratch and retained evidence. Clean only known
inactive reconstructible output after checking ownership; archive needed
artifacts before deletion. Do not remove live scratch, a frozen test subject or
shared model data. The 0.8.29 cleanup's65.12 GiB recovery is a useful precedent, not a
new license to prune unrelated work.

## Planning decisions still needed

Before activating 0.8.30, select the smallest useful bug-hunt and profiling set,
map it to current code/contracts and explicitly reconcile the existing feedback,
reset and fork allocations. Define any paid/GPU/live-fault bounds only when
those cases are selected. No old one-use approval is replenished by this note.

The quality-roadmap recommendations applied here are bounded failure injection,
resource/lifecycle checks, real-store state/property cases, boundary tests,
independent reference checks and targeted mutation/coverage review. Deferred
unless separately justified: whole-workspace MC/DC, exhaustive allocation/VFS
failure sweeps, a new general fuzzing framework, automatic full-scale/paid
campaigns, new GPU venues and CUDA13 delivery. New.29 evidence changes priority:
S50 lacks rank/snapshot attribution, default-Node growth remains unresolved, CI
lost a real failure diagnostic, static scanning dominated the fast tier, and
normal promotion retained a known broken credential path.

Use these as the inputs to a concise release plan. Success should mean defects
fixed, latency or robustness improved and useful feedback arriving sooner—not
more wrappers, receipts, test counts or mandatory checkpoints.
