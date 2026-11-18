---
title: FathomDB 0.8.30 release plan — correctness, performance and robustness
date: 2026-11-17
status: PROPOSED
target_release: 0.8.30
---

# FathomDB 0.8.30 release plan

Deliver fewer correctness defects, lower useful-query latency, more predictable
resource/lifecycle behavior and a cheaper, more reliable development/release
loop. Use a bounded bug hunt and measurements to choose changes; do not turn
this release into another testing-framework or paperwork project.

This is the proposed release plan, in the release's own directory. It draws on
the [0.8.29 retrospective](../../notes/0.8.29-retrospective.md),
[focused recommendations](quality-performance-robustness-recommendations.md),
[receiving allocations](release-followups.md) and
[fork allocation](fork-maintenance-plan.md). It does not start implementation,
activate release state, select new GPU venues or authorize paid/fault/publish
runs. Date above is the owner's America/Chicago date.

## 1. Scope and baseline

The main engineering tracks are delivery diagnostics/efficiency, correctness,
latency, lifecycle/resources and focused testing. Existing feedback, reset,
module-level model release and fork allocations are reconciled below; they are
not silently dropped or expanded into a feature program.

Start from current `origin/main` containing published 0.8.29 and its qualified
repairs. The known closure is `569652e61d0736ae788c7cbb2d327c4481fc3e1c`;
the retrospective/recommendations commit is
`3e637ab7ea84021f560667ab0099ba5d4546c08b`. Verify actual ancestry and changes
at activation rather than resetting to those snapshots. The shipping .29 tag
remains `b64d32f80bea2853bf4939b6334e32501174ffb1`.

Owner clarification seq-398 adds required 0.8.29 Tegra CUDA wheel/addon and
GitHub Pages delivery on `10.83.10.13`. That supplemental 0.8.29 work is OPEN;
generic ARM64 CPU packages and retained 0.8.28 Tegra packages do not complete it.
Finish it in 0.8.29 before accepting 0.8.30's final Tegra/platform baseline. Independent
.30 planning and CPU/source work need not wait; this draft does not move the
requirement into 0.8.30.

Reuse 0.8.29 evidence by the claim it supports and its source/features/native/
fixture/protocol identity. Do not replay paid 72, all 160 S50 members or healthy
GPU/platform controls simply because a new release number exists. Changed
native artifacts and declared package support still need their actual applicable
consumer proof. Store format 37 is the baseline; no format/default/precision
change, migration, chunking or new retrieval backend is selected by this draft.

Footprint: existing in-process Rust engine plus Python/TypeScript boundaries;
CPU correctness tests on the ordinary host, GPU tests only for consumed CUDA
claims, and hosted CI/release workers for their real platform/artifact duties.
No new model service, paid evaluation dependency or library network call is
introduced. Do not describe all query modes as CPU-only or deterministic across
unfixed asynchronous states.

## 2. Deliverables and acceptance

These are proposed release-local requirements, not new canonical AC numbers.
Freeze concrete changed-case selectors and comparison rules in Slice 0; do not
invent tests that have already passed or change an oracle to fit a result.

| ID | Deliverable | Falsifiable acceptance |
| --- | --- | --- |
| R30-DIAG | Failures remain attributable through cancellation. | A suite fails after more than 200 diagnostic lines; a later child is cancelled. Original exit, full useful diagnostic, completed outcomes and unrun tail remain available through existing capture mechanisms, with bounded redaction and correct fail-fast/collect-all behavior. Elapsed timing stays nonnegative under clock adjustment. |
| R30-COR | Bounded bug hunt and fixes at composed correctness boundaries. | Each selected case has a real reproducer/seed, intended result and classification. Confirmed defects have observed RED→GREEN and regression tests. Coverage/identity/feedback/FFI/queue/recovery invariants below hold on real databases where applicable; unresolved critical wrong-result/data-loss findings cannot be labeled passed. |
| R30-S50 | Resolve or precisely bound the retained score observation. | Inspect preserved sample 860/query 360/op 9971 first; any selected reproduction captures candidate/rank/tie and snapshot/cursor context sufficient to distinguish fixed-state error from concurrent-state movement. Report root cause or bounded non-reproduction with explicit residual uncertainty. No general tolerance or retroactive exact pass; a new semantic exception requires owner disposition. |
| R30-LAT | Current latency attribution and justified improvements. | Comparable baseline/treatment inputs and profiles; queue/service/component and end-to-end latency, p50/p95/p99, throughput, refusal/timeout rates and resource cost. A promoted change improves the selected measured bottleneck without breaking output/lifecycle or accepted numerical criteria. Neutral/negative results justify non-adoption, not a fabricated speedup. |
| R30-ROB | Resource/lifecycle behavior and safe release semantics. | Selected cancellation/close/reopen and default-Node memory cases have exact exit/post-state/resource outcomes. Default-profile behavior is assessed separately from semispace 4. An opt-in module-model release API can ship only after its reset-safe-drop prerequisite, binding parity and release/concurrent-use/reload behavior are qualified. |
| R30-TEST | Higher-value testing with less duplication. | A concise audit of expensive/critical suites names claim, observed runtime, useful defects/misses and overlap. Retained/new state/property, differential, boundary and fault cases cover selected risks; removal/consolidation preserves meaningful oracles and actual platform/runtime controls. No database mocks or regenerated goldens. |
| R30-CI | Efficient ordinary CI and reliable normal finalization. | Normal publication uses the qualified OIDC mechanism rather than the legacy token, with bounded visibility polling and expected/observed diagnostics. Missing native packages fail informatively. Tool-only setup avoids native wheel builds; the 856.43s static-check hotspot and selected DAG/setup costs have measured before/after improvement with their intended controls intact. |
| R30-FORK | Reconciled fork maintenance with truthful supported targets. | Every selected rebase/marker/kernel change has the retained/removed delta map, exact coherent pins/features, actual supported/unsupported controls and affected installed-consumer proof. Target list and numerical cold-start/build/size tradeoffs are selected before execution; no new dtype, CUDA13 or platform claim by inference. |
| R30-REL | Qualified release, then real published consumers. | Required source/performance/security/long/platform/artifact scopes have actual outcomes or explicit authorized dispositions. After separate publication authority, every required publisher, fresh exact-version registry consumer and co-tagging check passes; docs/retained URLs are verified. Partial publication is incomplete. |

## 3. Lean slice ladder and dependencies

| Slice | Work and requirement | Depends on | Exit |
| --- | --- | --- | --- |
| 0 | Baseline and scope selection; quality/policy review; minimal plan/state setup. | Published.29; current-source reread. | Existing proof applicability, selected early cases, unchanged thresholds and necessary resource limits identified; remaining consequential choices named once. |
| 5 | Diagnostic retention and CI/CD correctness/efficiency — R30-DIAG/CI. | 0 | Failure capture, normal OIDC/visibility and install diagnostics fixed; scoped setup/static/DAG improvements measured. No new general CI framework. |
| 10 | Correctness bug hunt and targeted repairs — R30-COR/S50. | 0; use5 diagnostics when relevant. | High-risk sentinels first; confirmed defects fixed, expected behavior/non-reproductions distinguished and residual critical uncertainty explicitly resolved or held for owner disposition. |
| 15 | Latency profiling and selected improvements — R30-LAT. | 0 for baseline; relevant10 invariants before promotion. | Same-state correctness/reference controls plus comparable performance results; retain justified non-adoption. |
| 20 | Lifecycle/resource robustness — R30-ROB/COR. | 0; consumed10 fixes; reset-safe fork repair before unsafe drop/release tests. | Default-Node growth characterized/remedied where reproduced; selected close/crash cases and any selected module release implementation qualified. Live resets remain separately authorized. |
| 25 | Test-value review and consolidation — R30-TEST. | 0; findings from5/10/15/20 as they arise. | Focused state/property/reference/fault coverage and reduced duplicate work; this is not a second full qualification campaign. |
| 30 | Scoped fork maintenance and GPU policy/qualification — R30-FORK/ROB. | 0 for research/design; actual consumed lifecycle contracts/repairs for implementation. | Owner-placed B/C work reconciled and selected final cohort qualified before final native/performance sign-off. |
| 40 | Combined qualification, interfaces/docs and release readiness — all requirements. | Selected exits 5–30. | One stable coherent-phase full gate plus necessary changed-claim/long/performance/platform/artifact checks and independent final review. Original failures/skips remain distinct. |
| 45 | Authorized publication and post-publish proof — R30-REL. | 40 plus explicit production authority. | Required registries, installed consumers, finalization, public docs and owned cleanup complete; immutable shipping tag preserved. |

Numbers are integration seams, not a request for a packet or model review per
helper. Use gaps only for a materially new coherent scope that will not fit its
owning slice; do not create a new slice for every found defect. Reconcile scope
before consuming a neighboring number rather than silently growing the ladder.

After 0, delivery work 5, correctness 10 and performance baseline 15 can proceed in
parallel; 25 starts its audit alongside them. Fork research/design can also start
without waiting for whole 20. Only consumed code/contracts gate implementation.
Serialize shared engine/search/coverage edits and dependency/pin integration.
Every concurrent writer has its own worktree; root alone integrates state.
Finalize relevant fork/native inputs before final numerical/artifact evidence;
if they change later, reassess only the claims they invalidate.

## 4. Bounded bug hunt and low-latency priorities

Use existing harnesses with a small first wave, not a Cartesian matrix:

- S50 preserved natural-write observation and one representative 100k hybrid/SDK
  case, before bulk lower-risk repetitions. Both differing original calls were
  reference-route; async projection remains unproved, and their 1.240362%/
  0.850694% differences are not a CPU/GPU rounding tolerance.
- Duplicate-body/distinct-record identity and repeated-ref atomic feedback;
  coverage `R=K+U`, unknown versus zero, UTF-8 spans and pinned-reader publication.
- Malformed native/wire values, repeated refused open then healthy reopen;
  queue admission/cancellation/close across multiple engines and path aliases.
- Actual commit/checkpoint interruption and post-state/reopen where changed;
  operator timeout or fake debugger transport is not delivered-fault proof.
- Default Node/native memory growth, known-loss refusal and close/last-clone
  behavior. Retain instrumented-profile and healthy/OOM credit separately.

Keep classifications: confirmed product defect, harness/operator defect,
expected behavior, not reproduced, or unresolved observation. A minimized real
regression is the deliverable; no standalone bug-catalog system is needed.

For 15, profile current eligibility/visibility scans, hydration/materialization,
serialization/copies, queueing and resource pressure. The 0.8.29 scan cost of roughly
48–49 ms is a lead, not a current 0.8.30 baseline. Cache HIT/MISS/key/invalidation
costs and natural-write benefit must be measured; time-dependent eligibility
can change at unchanged generation. Do not optimize the 0.005–0.015 ms dependency
check merely because it is easy, or infer universal wins from the two S50 hybrid
comparisons. Compare matched/interleaved inputs on the ordinary busy CPU host;
retain noise and all original failures without weakening accepted thresholds.

## 5. Reconcile existing owner allocations

| Allocation | Owning work | Retained boundary |
| --- | --- | --- |
| Precise feedback, seq-375 / TC-e70f4224 | 10 design/contract review; implementation only for the selected correctness need. | .29 atomic ambiguity refusal remains valid. Richer occurrence labels need explicit response/store/epoch lifetime and paging/fetch semantics; no ABI is silently chosen. |
| Reset review, seq-376/377 / TC-cab4c8d7; original TC-281155a1 | 20/30 | Software-loss tests do not qualify actual reset survival. Original Orin defect needs Orin proof; 3090 is separate. No prior one-use approval is replenished. |
| Module CUDA model release, B30-01 / TC-8d3c1cde | 20, consumed fork prerequisite in 30 | Preserve the [roadmap contract](../../ROADMAP.md#0830-backlog): explicit opt-in Rust/Python/TS call, no automatic idle release; clearable ownership, typed errors, concurrency, repeated release and reload. Safe post-reset drop is a hard prerequisite to shipping/executing the unsafe release path, not a paperwork exception. |
| S50 observation, seq-393 | 10 | Investigate the saved evidence; preserve fixed-state exactness and original .29 exception. That exception is not blanket .30 acceptance. |
| F30-REBASE / TC-509dc8a6; F30-MARKER / TC-93d11583; F30-MULTIARCH / TC-323f15b0, seq-367 | 30 | .30 placement remains. Choose a justified upstream/cohort and actual target list; retain required .29 repairs. Marker removal cannot remove pool behavior; kernel/dtype/target expansion is not automatically selected by the quality theme. |

If a previously placed item cannot fit the selected targets, safety prerequisites
or available authority, identify the specific owner disposition needed. Do not
quietly cancel it, call it passed or turn its uncertainty into a hold on unrelated
CPU/design work. Existing .30 allocations provide details; do not copy them into
new per-helper requirements packets.

## 6. Testing and evidence budget

Apply the [quality roadmap](../../notes/fathomdb-quality-testing-roadmap.md):
real-store state/property cases, codecs/projection/round-trip properties,
first/persistent failures at owned seams, independent small reference fixtures,
boundary/FFI cases, resource cleanup and a few useful branch/mutation controls.
New 0.8.29 evidence raises priority for rank/snapshot attribution, malformed error
paths, active-fault delivery, default Node growth and retained failure output.
Defer whole-workspace MC/DC, exhaustive allocation/VFS fuzzing, new generic
frameworks and broad large-scale/paid sweeps unless a selected finding justifies
them. Consultation creates no extra gate.

Use genuine failing tests before behavioral fixes and preserve human-intent
oracles. No speculative queue rewrite, golden regeneration or weakening of tests
to obtain green. Run scoped lint/type/test checks while working, then the required
full source gate once on a stable coherent phase. On unchanged-source interruption,
complete failed/unrun registrations; do not replay passing suites for metadata.
A subsequent broad code change can require another coherent phase, but a helper
or review correction alone does not define one.

For representative performance cases, record source/configuration and warm/cold,
format, snippet, scale, snapshot and instrumentation distinctions once with the
result. Use existing accepted bounds and choose the comparison/sample protocol
before treatment. Expand scope on concrete doubt, not to make a larger report.
Real package/platform proof remains distinct from inherited source qualification.

## 7. Delivery efficiency and process

Slice 5 addresses the observed failures first: immediate useful diagnostics before
collect-all continues, monotonic elapsed timing, shared normal/recovery OIDC
promotion, bounded visibility polling with observed values, useful optional-
package install errors, and known-worker inventory before eligibility/approval.
The postpublication [0.8.29 CI run](https://github.com/fathomadb/fathomdb/actions/runs/44244789028/job/134601018880)
also exposes a closure-check mismatch: `commission-manifest` requires every
ladder entry to be LANDED, including owner-deferred 120 and unselected 150.
Keep those dispositions truthful and make the checker distinguish selected
unfinished work from explicit deferred/unselected work; do not fabricate passes.
Then reduce the slow static checker, mandatory native bootstrap work, duplicate
builds and inappropriate diff/platform triggers. Preserve meaningful mutation,
Windows runtime, package-loading and release access protections.

Ordinary CI remains informative and proportional. No new merge queue,
required-check expansion, nightly campaign or soak period. Existing production
trust, reviewer, exact-tag, actual installed-consumer and cleanup rules still
apply; do not mistake an old settings snapshot for current authorization.
Prefer one qualified producer and promotion when existing trusted source/build
identity supports it; unknown inputs require a build, not a claimed exemption.

Maintain this plan, concise existing slice evidence and one release-state/board
when execution is activated. Update once with substantive work; link raw history
instead of appending several competing “current” summaries. Use one coherent
design review and one independent implementation/final review for substantive
changes, not reviews or renewed approval for every helper. No extra rehearsal
campaign is selected; perform only meaningful changed-scope rehearsal when its
risk warrants it and actual shipping/consumer work at release time.

## 8. Cross-cutting completion and behavior register

Every selected change needs its regression/result, applicable SDK parity and
public interface/ADR updates. Rust, Python and TypeScript result/error/lifecycle
contracts move together where affected. Public docs changes update navigation
and pass strict MkDocs; developer/public indices are updated where actually
changed. `dev/acceptance.md` remains locked; use local requirement IDs and test
names. Source/type/security/long and performance checks retain actual scope,
exits, skips and ignored cases. “No confirmed bug” alone is not qualification.

| Proposed behavior change | Required treatment before shipping |
| --- | --- |
| Confirmed search/coverage/feedback/error fix | Intended result and precedence established by real RED→GREEN; affected bindings/docs updated. Any change to a published contract gets its ADR/interface update. |
| Module-level release call or richer feedback labels | Explicit reviewed API/lifetime/error semantics and cross-binding tests before selection; preserve the existing model-close and feedback behavior elsewhere. |
| Cache, allocation or fork/kernel change | Correctness/reference and lifecycle controls plus actual matched performance/artifact scope. No new precision, default, support claim or schema change silently bundled. |
| CI/release implementation | Normal finalization and diagnostic/cancellation behavior fixed; proportional execution remains. No hook bypass, force-retag or reduced release trust. |

This is a behavior register of planned change classes, not a claim that exact API
names or every product delta are already designed. Keep it current with actual
selected changes and their changelog lines; do not create another parallel log.

## 9. Known conflicts, policy review and open selections

The current source checks still show divergent mechanisms: normal
`.github/workflows/release.yml:1688` uses NPM_TOKEN while recovery
`promote-npm-release.yml:69–79` uses the qualified OIDC CLI; `agent-output.sh:31/40`
uses wall time while the suite runner has monotonic timing; CI spill collection
is late at `ci.yml:339/430`; bootstrap calls owned provisioning even for setup.
These observed conflicts belong to 5, not another audit packet. Old force-retag
memory and stale “pending” historical prose must not override current policy or
release state. Use current AGENTS/source plus owner rulings.

**CUDA policy review.** The standing [per-release review](../../design/cuda-version-support-review.md)
is completed as a bounded [dated refresh](research/cuda-version-adoption.md), with comparable usage shares explicitly unknown. It is a supporting input, not a
new GPU experiment or permission change. Retain the accepted importance/driver/
primary-toolkit policy unless the owner ratifies a change; new CUDA 13 delivery
and quantitative work remain in 0.8.31. Unknown comparable-month usage shares stay
unknown, and the 0.8.29 review is not relabeled as fresh. Before 30 selects targets,
reconcile the refresh, existing fork requirements and actual available venues.

**Overflow tripwires.** The [chunking review](../../design/overflow-chunking-reevaluation.md)
remains parked. No new answer-level past-cut loss or exact-scan-kernel landing is
established by this planning pass, and inspected fusion/default-embedder pins
remain unchanged. Do not infer “T1 never occurs” from the 0.8.29 format comparison.
If implementation changes those pins or establishes a tripwire, follow the
existing re-evaluation; this draft does not select chunking or ANN.

Open selections, resolved only when consumed: exact bounded native/fault profiles
and venue authority; safe model-release/feedback API details; justified upstream
revision and kernel target list; any actual support/precision change; production
publication/runner access. None requires asking permission again for already
selected ordinary source work or replacing established .29 proof.

## 10. Immediate next action

Review this proposed plan as one coherent scope. On execution authorization,
open 0: verify current source/accepted carryover, select the small first-wave
cases and resource bounds, complete only consequential policy/API/target choices,
and activate the minimal state/board with this file as the plan pointer. Then
start 5, 10 and the 15 baseline in parallel where source ownership permits. No
0.8.30 implementation, paid/model/GPU/fault run or publishing action has started here.
