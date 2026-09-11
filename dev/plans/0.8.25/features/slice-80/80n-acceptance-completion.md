---
title: Slice 80.n — acceptance completion and Slice 85 unblock
status: DRAFT
depends_on: 80.m
---

# Slice 80.n — acceptance completion and Slice 85 unblock

## Purpose and authority

Complete the remaining Slice 80 obligations, preserving accepted evidence,
then unblock Slice 85. This is a work package inside Slice 80, not a new
release-ladder entry. Slice 85 still owns final verification, CI and packaging;
unblocking it does not mean the release is ready or authorize publication.

The goal is to resolve Slice 80's remaining blockers and unblock Slice 85,
not merely to produce another stopped campaign report. Once commissioned to
execute this plan, proceed through inspection, ordinary file reads/copies,
focused harness fixes, short tests, diagnosis, review and bookkeeping without
requesting separate permission for each step. Consolidate any still-needed
long-campaign approval into one clear request before dispatch. The exhausted
AC-072 allowance is not implicitly renewed by writing/reviewing this plan;
OS changes, unrelated-process termination and publication remain separate.

### Practical execution order

1. Inspect the real code, runner, parser and retained evidence together; list
   only actual Slice 80 blockers, and mark settled AC-081/write work reusable.
2. Fix the prebuilt AC-072 route and its focused tests; independently inspect
   the complete invocation-to-verdict path before spending campaign time.
3. Run the very short real-database smoke below through that same path. Repair
   defects it exposes; do not advance just because helper unit tests pass.
4. Establish a qualified environment, run the approved full AC-072 campaign,
   and validate each result before starting the next.
5. Resolve remaining Slice 80 findings, finish reviews/state/handoff, and
   advance to Slice 85. Do not reopen unrelated passed tests or reproduce
   Slice 85's final verification in this slice.

A stop rule stops affected timing, not all useful work. Continue safe in-scope
diagnosis, repair, unaffected checks and closeout preparation. Escalate only a
concrete permission/scope need or a genuine unresolved blocker, with a proposed
resolution; do not invent another numbered work package for routine corrections.

Operate directly on release/0.8.25 with one writer per checkout. Do not use
Steward or Orchestrator roles. Read repository instructions and relevant memory;
verify branch, HEAD and existing changes before editing. Obtain independent
read-only design/code/evidence reviews at the seams specified below.

## Baseline and obligations

Reviewed baseline: `6f9f07e0`; 80.m campaign source: `ad422346`.
Read [80.m](80m-collector-completion.md), [result](../../../runs/0.8.25-slice-80/result.md),
[measurements](../../../runs/0.8.25-slice-80/measurements.json),
[reviews](../../../runs/0.8.25-slice-80/verification-review.md),
[design](design.md), [execution manifest](execution-manifest.json),
and [Slice 85](../slice-85/plan.md). Resolve actual identities from Git and logs,
not these abbreviated references alone.

| Obligation | Current disposition | Work remaining |
| --- | --- | --- |
| AC-020 retirement and successor registration | Implemented and reviewed | Verify current mappings; do not reopen the ratio gate. |
| AC-081a/b | 80.m: seven qualified passes, no warnings; medians 172.883034/52.527637 ms | Reuse with input-applicability proof; no new campaign. |
| AC-081c / REQ-018 | Real-database independence proof passes | Reuse unless its relevant implementation changes. |
| Collector self-exclusion | Repaired and reviewed in 80.m | Preserve PID-specific behavior and positive competitor controls. |
| AC-072 | 80.m R1: 69/75 ms, swap-in delta 2; R2/R3 unstarted | Qualified three-run evidence remains missing. |
| Protected writes | Six Slice 79 receipts applicable | Record applicability; no historical baseline reruns. |
| Reporting/state | Old AC-081 blockers remain in current views | Reconcile current versus historical status. |
| Slice 85 start | Blocked by incomplete Slice 80 | Complete acceptance, reviews and input-invalidation handoff. |

AC-072 remains 10,000 records, 384 dimensions, 1,000 queries, warm treatment,
p50 <=80 ms and p99 <=300 ms. Preserve the registered percentile calculation,
result assertions, runtime mode, query mix and positive-execution guards.
AC-081 budgets remain sequential <=500 ms with warning >=200 ms and concurrent
<=100 ms with warning >=80 ms. No threshold or environment-policy relaxation.

## N1 — reconcile evidence before doing more work

Create one current acceptance summary pointing to exact campaign/receipt IDs.
Mark previous attempts historical, without changing their raw bytes or verdicts.
Current AC-081 must say PASS and point to 80.m; current AC-072 must say
INCOMPLETE, with the invalid R1 and unstarted R2/R3 explicit. Neither numerical
passes nor collector readiness alone constitute acceptance.

Update plan/status/result/manifest/measurement summaries so their leading
statements agree. Preserve historical sections with clear labels. Do not rewrite
signed-off historical review text or verdicts: add a new 80.n review and point
the current summary to the applicable reviews.
Update release-state blockers to the actual AC-072 obligation and outstanding
reviews, not the resolved self-census issue. Edit release-state JSON and regenerate
its views with the repository tool; never hand-edit generated blocks. Keep
`next_slice=80` and Slice 85 NOT_STARTED until final closure.

Record an applicability table: receipt, source, executable/toolchain/features,
relevant inputs, diff to current candidate, reuse decision and reason. A changed
collector hash or documentation SHA does not itself invalidate unchanged product
performance. Conversely, matching prose or a passing old review is not proof
that a changed runtime remains covered.

N1 closes when raw hashes and current summaries agree and each retained pass
has an explicit applicability decision. No benchmark is required.

Protect the accepted AC-081 relevant-input hash
`95e15e3e4c089212431b7a173fca291539a072d98c87d3394c1fb9b4f3573274`.
Recompute it using the 80.m manifest's exact input list and command, before
implementation, at implementation completion, before readiness and before
dispatch. Implement the pre-dispatch comparison as a fail-closed guard, not
just a checklist assertion. This package edits only harnesses,
their dedicated tests, tools and records. Do not edit `perf_gates.rs`, reader
tests, Cargo/build inputs or product source to add receipt markers or convenience
hooks: those inputs are protected even when a proposed edit appears test-only.
Investigate an unexpected mismatch before spending AC-072 timing. Resolve
metadata/command mistakes locally; an actual product/input change requires an
applicability/scope decision. Do not strand closure over a bookkeeping mistake.

## N2 — separate AC-072 build from qualification

The 80.m AC-072 log records a 32.85-second compilation after its starting
environment snapshot. Its machine-wide swap delta therefore spans build, setup
and execution; it cannot identify which phase swapped. Do not claim compilation
caused the swap or retrospectively reclassify R1.

Inspect the complete AC-072 Rust test, `run-ac013.sh`, the Slice 71 cell runner,
the Slice 80 AC-081 build-once runner and existing parsers before editing.
Prefer reuse of build artifact selection, identity validation, collection and
qualification helpers. Make the smallest explicit AC-072 prebuilt-executable
route; do not silently change historical runner contracts or create a new
benchmark framework. Keep any legacy route clearly separate from this campaign.

Build once outside qualification; select the exact Cargo test artifact, seal its
SHA-256, source and relevant-input hashes, features, toolchain, SQLite version,
runtime configuration and exact selector. Verify that the required test exists
once. Use the prebuilt artifact directly with AGENT_LONG=1 and the unchanged
AC-072 fixture variables. No Cargo, rustc or rebuild belongs between start/end
qualification snapshots. Include test setup, seeding, drain and teardown in
the qualified process window; do not narrow the window to discard inconvenient
swap events. Internal query timing stays unchanged.

Use a fresh task-owned temporary artifact directory, not a reused fixed target
or copied binary whose origin is uncertain. Reuse an already sealed executable
instead of rebuilding only if its existence, digest, test listing, build flags
and exact relevant-input applicability are verified. Record that choice before
readiness; a missing artifact is a build prerequisite, not a timing failure.

First check the retained artifact
`target/release/deps/perf_gates-e6869802f14984e9`, whose expected SHA-256 is
`ff4b78f36898f8109eef2dcb64f91a99741f9619433d77de0cfef0e59d51895d`.
The plan review found it present with both required selectors and unchanged
protected inputs. Reverify, copy to the fresh artifact root, and verify the copy;
do not rely on continued presence or execute a mutable Cargo-target path.
If unavailable, use one locked/offline no-run build of the same feature set
outside qualification. Missing offline dependencies require a setup disposition,
not an opportunistic network/toolchain change during readiness or timing.

Reject missing/stale identities and dirty relevant inputs before starting a cell.
Retain test exit, one positive test execution, workload counts, sample records,
collector identity and complete ordered environment snapshots. Validate and
report both numeric and environmental outcomes before another cell can launch.
Ensure script success cannot mask a failed Rust test or invalid qualification.

### AC-072 parser and dispatcher contract

The current Slice 80 CLI is not an AC-072 campaign runner: `parse-cell` and
the seven-observation summary are AC-081-specific, and reporting commands may
exit zero while printing an invalid verdict. Do not use their exit code as
acceptance or build an unreviewed inline parser during the campaign. Reuse
their pure qualification helpers, but implement and test the smallest explicit
AC-072 cell validator and three-cell dispatcher before sealing commands.

The validator checks the exact selector, one executed/non-ignored test,
successful strict Rust exit, n=10000, 1000 retained samples, warm treatment,
successful drain and 10000 materialized vectors. Bind vector dimension 384 and
runtime mode to actual invocation/build provenance; do not pretend an existing
log field proves information it does not contain. Preserve existing fixture
behavior; do not add product assertions to solve a receipt-tool problem.
The Rust test's full-Duration assertions remain the numerical oracle: printed
milliseconds and retained microseconds are truncated and cannot override a
failing boundary assertion. Reject skipped and process-cold early returns.

Require exactly one `AC013_NUMBERS` record and one warm treatment record
from direct execution, not duplicate records reprinted by the old wrapper.
Require exactly 1,000 positive retained microsecond samples; recompute the
registered percentile ranks and verify consistency with printed integer
milliseconds, without overriding strict Rust exit. Require accepted_writes=10000,
vector_rows_after_drain=10000 and drain_outcome=ok. Preserve the fixture's
`result_counts=not_retained_per_query`; do not invent per-query result evidence.

The runner receipt binds artifact/build/campaign/collector/input identities,
selector, empty feature set, performance mode and actual child environment:
AGENT_LONG=1, AC013_CORPUS_N=10000, AC013_VECTOR_DIM=384,
AC013_SAMPLES=1000 and AC013_SCALE_TREATMENT=warm. The current test has 1,000
samples fixed in code; record both that fact and the invocation rather than
claiming the environment variable alone controls the count. A controlled child
test must prove actual argument/environment delivery, not only marker formatting.

Persist a structured outcome before returning a dispatch decision. Only
applicable PASS permits the next AC-072 cell. Failure, invalidity, malformed
evidence, timeout and missing output stop the dispatcher with nonzero status.
Keep numerical and environment verdicts separate, including when both fail.
Do not silently catch parse errors and continue. Test stopped campaigns with
one/two observations as INCOMPLETE, never a passing three-cell summary.

Freeze code, collector and executable identities before readiness and timing.
New logs under the declared evidence directory do not constitute product
identity drift. Validate declared code/build/collector inputs rather than
requiring an entirely empty `git status` after every receipt. Do not commit
between observations or rebuild because only documentation HEAD changed.
Record the build source separately from the campaign source and prove reuse
by relevant inputs. Raw output paths are create-only: existing labels/files
must be rejected, never overwritten or automatically resumed after a failure.

Use exclusive creation for the campaign directory, per-cell logs/verdicts and
final summary. A timed-out or interrupted child leaves an INCOMPLETE observation
and stops dispatch. Test bounded termination and reaping of that task's child
processes so no orphan benchmark contaminates a later readiness check. Never
terminate unrelated processes as part of cleanup.

Use this sequencing: inspect the existing artifact, implement/review the focused
harness and run the short smoke; record the campaign approval and final artifact,
command and collector identities, then commit the ready candidate/seal. Capture the resulting campaign
HEAD externally or in raw identities rather than trying to embed a commit's own
SHA inside itself. Readiness and all observations then use that frozen campaign
HEAD. Any subsequent code/seal correction returns to readiness before timing.

## N3 — reuse and extend testers with focused TDD

Inventory existing tests first. Reuse `test_slice80_read_acceptance.py`, the
collector-readiness and competitor-census shell tests, and existing artifact
selection/oracle tests wherever they cover the requirement. Do not duplicate
the database fixture, change established assertions to obtain GREEN, or mock
SQLite for semantic integration coverage.

Add only missing RED cases, stage/commit them visibly, implement GREEN, then
refactor. Harness-only controlled subprocess fixtures are appropriate to prove
invocation and error propagation; they must never count as AC-072 measurements.

Required focused proof:

- Prebuilt route launches the exact selector once with exact fixture variables;
  Cargo/rustc cannot be invoked by that route. Use a failing command sentinel
  or equivalent deterministic harness test, not a timing benchmark.
- Wrong artifact/input identity, missing or ignored test, zero counts, missing
  controls, duplicate/malformed records and failed child exit fail closed.
- Numeric failure and environment invalidity are preserved independently;
  the dispatcher does not launch a second cell after either outcome. Test this
  with controlled harness observations without spending acceptance runs.
- Both production collector paths exclude themselves and detect a separate
  live runner-shaped competitor. Preserve truncated/hashed-name coverage.
- Any newly named AC-072 runner and copied binary must be recognized by the
  shared competitor scanner and positive-control tests. Changing a filename
  must not silently evade the exclusion policy; exclude self by PID, not name.
- Collector-only readiness runs no benchmark and cannot be reported as one.
- Current-summary selection chooses the applicable 80.m AC-081 campaign rather
  than a historical invalid campaign; add a small test if selection is automated.

Run only changed-target lint/typechecks and these focused tests. Do not rerun
Rust semantics suites merely because shell/reporting code changed. Independent
review must inspect the actual execution path and proof, not only helper tests.
Seal exact commands, expected positive counts, timeouts and output paths in
the execution manifest after inspecting the implementation. N2/N3 close when
the path and tests pass review with no hidden build in readiness. Review actual
Rust fixture behavior as well as shell/Python code: environment defaults, fixed
sample count, warmup, seeding/drain, output shape, threshold assertions, exit
propagation and cleanup. Inspect the exact final diff before longer execution;
a unit-test count is not a substitute for that inspection.

## N4 — very short end-to-end smoke, then acceptance

Before the longer campaign, run one real-database smoke through the same
reviewed runner, sealed release executable, collector and validator. Use exact
selector `ac_013_vector_retrieval_latency`, AGENT_LONG=1,
AC013_CORPUS_N=10, AC013_VECTOR_DIM=384 and AC013_SCALE_TREATMENT=warm,
with a 30-second whole-run timeout. The compiled fixture still performs 1,000
warmup and 1,000 measured searches; do not attempt to shorten it by setting the
inert AC013_SAMPLES variable or changing protected Rust source. A ten-row
corpus makes this a short functionality/integration check, not an acceptance
performance measurement. Expected wall time is seconds; that estimate is not
a measured result or an acceptance limit.

The smoke exercises real database creation/open, runtime configuration, writes,
projection drain, ten materialized vectors, search, timing records, child exit,
environment qualification and final receipt generation. Its validator expects
n=10, accepted_writes=10, vector_rows_after_drain=10, samples=1000 and the
same complete marker/sample checks. Validate delivery of the child environment;
do not use a separate smoke-only runner that bypasses the actual campaign path.
Use a typed SMOKE purpose with expected corpus size 10; acceptance purpose
remains fixed at 10000. Add a focused test that smoke evidence cannot satisfy
the three-cell campaign or silently change its expected corpus size.

Store the result outside R1–R3, clearly marked NON_ACCEPTANCE. Functional,
parser, identity or timeout failure means diagnose and fix the affected path,
then rerun only the short check needed to prove the correction. No long campaign
starts until the smoke is functionally passing and its environment qualifies.
Reuse this as the live readiness check rather than stacking an additional idle
30-second observation. Collector-only and controlled negative tests remain
focused harness tests, not additional benchmark campaigns.

Inspect the smoke's load/memory/thermal/affinity/quota/governor/process/swap
controls. Distinguish existing swap allocation from new I/O. If invalid,
continue bounded read-only diagnosis and resolve ordinary harness defects.
If an external workload/host condition needs intervention, present that concrete
action to the owner. Do not disable swap or terminate unrelated applications
without approval. A passing smoke cannot guarantee zero swap in the roughly
140-second full workload; full cells still qualify independently. Do not
repeat smoke merely until a favorable environment appears.

Obtain one consolidated execution allowance covering the short smoke and
conditional full campaign, not a new approval after a passing smoke. Proposed scope:

| Work | Proposed timing budget | Stop rule |
| --- | ---: | --- |
| AC-081 | 0 | Preserve accepted 80.m evidence. |
| Short smoke | One initial NON_ACCEPTANCE smoke; focused repeats after an identified harness fix | Must pass functionality and qualification before longer work. |
| AC-072 | One campaign, three fresh processes | Qualify each cell immediately; first failure/invalidity stops remaining cells. |
| Protected write workloads | 0 for harness-only changes | Invalidation requires a separately bounded plan. |
| Broad regressions | 0 | Reserved for Slice 85. |

This table is a proposal, not authorization. Once approved, proceed directly
from a passing smoke to the full campaign without another permission handshake.
Record approval reference, source,
artifact/collector hashes, environment policy, commands, timeouts and immutable
new paths under `dev/plans/runs/0.8.25-slice-80/raw/ac072-slice80n/` before dispatch.
Use all three observations from this one campaign; do not fill missing cells
with earlier invalid or favorable historical runs. Readiness is a precondition,
not a guarantee of valid subsequent timing.

## N5 — issue classification and closure navigation

| Observation | Action | What can close |
| --- | --- | --- |
| Reporting-only inconsistency | Correct current views, retain history, focused validation. | Reporting item; no timing invalidation by itself. |
| Collector/dispatcher defect before timing | Focused TDD and review; at most one correction plus one review amendment before re-planning. | Harness item after executable proof; no acceptance claim. |
| Identity/collector defect during timing | Preserve log, stop dispatch, diagnose defect. | Completed unrelated work stays closed; no automatic replacement run. |
| Preflight environment invalid | Stop before spending timing; present bounded diagnosis and specific owner action. | No AC-072 acceptance closure. |
| Timed cell environment invalid | Retain numeric result plus invalidity; stop remaining cells. | AC-081 remains closed if inputs unchanged; campaign requires new disposition. |
| Environment-valid numeric failure | Record real failure; stop and propose focused cause investigation/TDD scope. | Do not tune or retry under this harness-only plan. |
| Three valid numerical passes | Audit raw evidence and applicability; resolve review findings. | AC-072 closes after evidence review. |
| Product/runtime change proposed | Explain necessity and affected receipts; obtain scope and measurement approval first. | Reopen only demonstrably invalidated obligations. |
| Owner waiver/defer | Persist explicit scope and unresolved gate. | Not a passing AC-072 closure; requires explicit alternative Slice 85 entry decision. |

Do not reopen accepted gates simply because another gate is blocked. Conversely,
never mark the entire slice complete because implementation or numerical limits
pass while qualification is missing. After two same-issue corrections, stop,
externalize cause and alternatives, and request direction; no silent third loop.

## N6 — final review and Slice 85 handoff

Independent evidence review must reproduce raw hashes, candidate/input identity,
counts, thresholds, qualified outcomes and stop-rule compliance. Code review
covers harness changes; reuse prior design/product reviews where applicable.
Re-run only focused tests affected by review corrections. Preserve all attempts.

Write a Slice 85 applicability/handoff map covering every inherited obligation:
accepted AC-081 and new AC-072 receipts, reader independence, six write receipts,
prior Slice 75 corrections, and the retained platform/SDK/CI/package inventory.
For each, name reuse, invalidated, missing or explicitly unavailable disposition
and its owning slice. Unexecuted Slice 85 obligations are not extra Slice 80
gates. Do not execute final packaging/full regressions to prove this handoff.

For unaffected Slice 75/platform/SDK/CI/package inventory, reference its retained
manifest and assign pending reconciliation to Slice 85. Do not reconstruct
Slice 85's entire executable matrix as an additional Slice 80 closure gate.

Normal Slice 80 closure requires all of:

- Current AC-081a/b/c applicable passes; AC-020 remains retired.
- Three applicable passing AC-072 cells under unchanged acceptance controls.
- Protected write applicability established or authorized invalidated guards passed.
- Focused code and independent evidence reviews pass with no material finding.
- Current summaries, manifest and release-state agree; all historical failures retained.
- Slice 85 receives the complete input-invalidation map and its own READY-planning tasks.

Only then mark Slice 80 complete in release-state, set `next_slice=85`, regenerate
views, run focused documentation/state checks and commit the reviewed closeout.

Commit accepted evidence, applicability map and reviews first. The subsequent
release-state/view closeout commit references that reviewed evidence commit;
do not attempt a self-referential closeout SHA or invent a future hash. Verify
the final clean branch and both commits from Git before reporting completion.
Overall `release_readiness.status` remains BLOCKED by Slice 85's final
verification/packaging obligations even after Slice 80 closes. Remove only
resolved Slice 80 blockers; advancing `next_slice` is not release acceptance.
Keep Slice 85 NOT_STARTED until its agent actually starts; its DRAFT matrix must
be reconciled and reviewed before expensive execution. Do not claim release
readiness, launch Slice 85's broad round, push, tag, publish or merge to main.

Final report states closed items, remaining items, evidence references, exact
Git SHA and whether Slice 85 is unblocked. If blocked, name the single actionable
decision or prerequisite and preserve completed work instead of restarting it.
