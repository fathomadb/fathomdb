---
title: FathomDB 0.8.30 release plan
status: COMPLETE
target_release: 0.8.30
date: 2026-11-17
---

# FathomDB 0.8.30 release plan

Completed and published on 2026-11-21. The [delivery record](features/slice-80/status.md)
preserves actual registry, recovery, Tegra and documentation outcomes and their
limits. The phase instructions below retain the approved execution plan.

Survey and improve correctness, performance, end-to-end latency, robustness and
CI/CD efficiency. Use the [historical findings](testing-robustness-findings.md)
and [.29 retrospective](../../notes/0.8.29-retrospective.md) to select bounded
work, preserve useful tests and qualification, and fix demonstrated defects.
Two feature slices include both bug hunting and remediation.

The owner requests this release structure and authorizes completion of prework
0–8 after planning and independent review. Slice 9 remains the interactive HITL
checkpoint. Slice 10 implements only its approved preparation. Product work,
live faults, new GPU sessions, paid runs and publication require their applicable
later commissioning or concrete authority; drafting an execution plan grants none.

## Release goal and scope

Use current `origin/main` and reconcile changes at each slice entry. Published
.29 tag `b64d32f80` and closure `569652e61` are historical anchors, not reset
points. Carry the subsequent staged-Tegra Cargo-config repair and scope note
from `d6e5d9427`. Supplemental .29 Tegra CUDA wheel/addon/Pages delivery under
seq-398 was completed in .29; match its retained evidence to the final .30
Tegra subject before claiming reuse.

The existing Rust engine, Python/TypeScript boundaries and MCP server remain
the product footprint. Format 37, accepted numerical gates, driver/toolkit and
support policies remain baseline. No migration, chunking, ANN, new precision,
retrieval backend or automatic idle-model release is selected. A proposed cache
needs current matched benefit and correct snapshot/time invalidation before
separate adoption. Physical erasure is complete except for concrete defects.
CUDA 13 delivery remains .31-owned. Preserve [.30 feedback/reset allocations](release-followups.md),
[module-model release](../../ROADMAP.md#0830-backlog) and [fork work](fork-maintenance-plan.md).

Owner clarification seq-398 required the .29 Tegra CUDA wheel/addon and Pages
delivery on `10.83.10.13`. This was completed in .29 by
[run 44246092920](https://github.com/fathomadb/fathomdb/actions/runs/44246092920),
including fresh installed public consumers. Reuse the exact supported Orin/CUDA12.6
baseline where applicable; this does not extend support to other ARM/GPU hosts.

## A. Prework effort

Slices 0–8 inspect and propose; they do not implement the changes they discover.
The minimal Git branch/worktree isolation in Slice 0 is the administrative step
needed to avoid writing proposals on main. Existing isolated planning work can
be reused; no venv/npm/toolchain/platform provisioning occurs in this effort.
Write one concise result per slice at `prework/slice-N.md`; label findings,
proposals, uncertainty and deferred execution distinctly. Link sources rather
than copying historical status packets. Shared proposals are reconciled in 9.

| Slice | Investigation and durable result | Acceptance / limit |
| --- | --- | --- |
| 0 | Identify environment setup/changes and project infrastructure. Verify/create the dedicated `release/0.8.30` branch and worktree and switch before writes; verify an overall plan exists. Inventory checkout-owned venv, Rust/Python/TS/Node/npm pins, CPU/native features, Linux/Windows/macOS and x64/ARM64 needs, relevant browsers if any, compute/VM/runner locations and access. | Record actual baseline/isolation and current versus proposed environment choices, availability and consequential gaps. No dependency installation, external provisioning, host change or new execution state. |
| 1 | Plan the library/Dependabot sweep. Enumerate direct/transitive dependencies, available upgrades, advisories, pending automated PRs and exact version/source pins with their compatibility/qualification reasons. | Give each candidate a keep/upgrade/investigate disposition and owning slice; distinguish unavailable lookup from no updates. Do not update manifests, lockfiles or forks. |
| 2 | Review cruft across program docs, `dev/`, requirements/acceptance/architecture/design/code/tests, notes/intermediate records and public/developer/contributor docs. | Enumerate scoped candidate groups with evidence and keep/deprecate-in-place/archive/delete proposals. No deletion, moves or authoritative document changes. |
| 3 | Review product needs, requirements, acceptance and architecture. Draft create/update/rename/delete changes locally and allocate each to 10 or a feature slice. | Local proposed contracts have observable acceptance and owners; preserve canonical acceptance/ADRs pending decisions. No product implementation or accepted-contract edits. |
| 4 | Review architectural changes suggested by earlier prework, then inspect code at high level for alignment. | Record actual owner/call-path examples and propose code or architecture changes. Separate verified mismatch from investigation; no structural rewrite. |
| 5 | Review verification adequacy: need→requirement→acceptance→test, product goals, known critical paths, oracle strength, feature/platform execution and runtime cost. | Map selected critical claims to real tests, including in-crate tests; record gaps, duplication and applied/deferred testing methods. Do not build a new framework or run the full test matrix. |
| 6 | Inspect available release evidence and transcripts for stale user, consuming-developer and contributor documentation. | Record source-grounded contradictions and proposed corrections with owners; identify inaccessible transcripts. No invented transcript evidence or full corpus copying. |
| 7 | Inspect build/preflight/agent-verify/CI/package/gitleaks/registry and post-product-build delay evidence, including recent failures. | Identify observed causes versus hypotheses and pragmatic fixes, preserving original outcomes, scopes and applicable command ordering. No hosted dispatch, publication or speculative platform repair. |
| 8 | Reserved placeholder. | Record explicit empty disposition; do not invent work. |
| 9 | Consolidate 0–8 proposals, score understood/risk/effort, recommend include/postpone, present concise choices interactively and collect HITL decisions. | Record actual decisions, draft 10 from them, obtain direct subagent review, allow at most four FIX-n cycles, present resulting changes interactively and capture the final decision before updating the plan. No invented approval. |

When later commissioned, Slice 10 implements only the preparation selected in
9: setup/tool/dependency/cruft/document changes, exact scope and validation.
Its plan is drafted after the decisions; a speculative all-purpose implementation
packet is not needed now. Substantive product work belongs to 20+.

## B. Feature slice ladder

Initial feature numbers are multiples of ten beginning at 20. Gaps remain
reserved for a necessary coherent follow-up; do not assign a number per defect
or silently change an adjacent slice. Scope overflow goes back to the owner.

| Slice | Deliverable and requirement | Depends on | Falsifiable exit |
| --- | --- | --- | --- |
| 20 | Efficient, diagnosable CI/CD — R30-CI/DIAG | Approved 10 preparation | Preserve first failure through cancellation, fix normal OIDC/visibility and truthful closure checks, measure selected setup/static-check savings without losing controls. |
| 30 | Correctness bug hunt and remediation — R30-COR/S50 | Approved 10; consume 20 diagnostics when needed | Classify bounded S50/identity/feedback/coverage/FFI cases; confirmed defects have observed RED→GREEN and exact state/error regressions. |
| 40 | Performance and end-to-end latency — R30-LAT | Approved 10 for baseline; relevant 30 fixes and final consumed 60 pins before promotion | Comparable baseline/treatment, measured useful latency/tails/throughput/resources, unchanged correctness; justified non-adoption is valid. |
| 55 | Data integrity and recovery — R30-TEST | Approved10 and consumed30; applicable50 runtime fixes before affected promotion | Independent cross-index/temporal/relationship state, bounded fuzz and crash/WAL proof, doctor fixes and thin behavioral fixtures. |
| 60 | Scoped fork maintenance and qualification — R30-FORK | Complete55 underseq-409; approved10; consumed50 lifecycle contracts | Selected upstream/delta map, coherent pins, independent supported/unsupported kernel/output/resource controls and installed consumers. |
| 50 | Runtime concurrency and resource bug hunt and remediation — R30-ROB | Reviewed partial source integrated; complete60 before remaining50 underseq-409 | Positive concurrent work, capacity/cleanup/reopen assertions, default Node growth disposition and safe selected module-release behavior. |
| 70 | Integrated behavior and source qualification — R30-QUAL/TEST | Selected20/30/40/50/55/60 exits | Applicable source/security/features/long/performance/fidelity and composed-behavior checks; every selected test gap has a discriminating oracle and owner; original nonpasses stay visible. |
| 75 | Platform and artifact qualification — R30-QUAL | Source-qualified70 candidate and final consumed pins | Five-target installed/native/source-archive consumers, actual Windows WAL and hosted cancellation, declared CUDA support, contracts/docs convergence and nonpublishing immutable-candidate handoff. |
| 80 | Authorized delivery and registry consumers — R30-REL | 75 and separate production authority | Every required publisher, exact-version installed consumer, co-tagging, normal finalization, public docs/download and owned cleanup completes. |

Detailed drafts live at `features/slice-N/plan.md`. Their requirements are
release-local proposals, not new canonical AC numbers. After 10, delivery20,
correctness30 and baseline40 can proceed independently where ownership permits;
Owner seq-409 selects55 →60 →finish50 →70 →75 →80. Reviewed partial50 runtime
source is integrated before55;50 remains open and is not closed/reopened.
60 follows55 and supplies50's exact-pin safe-drop prerequisite. After60, finish50
including the separately authorized default Node private-pool comparison.
70 joins all selected implementation exits;75 consumes70 and supplies80 the
qualified artifact candidate. Final fork/native inputs precede final
numerical/artifact proof. Serialize shared engine/search edits and pin/state
integration; concurrent writers each get a worktree. Reuse valid earlier proof
by actual source/features/artifact/fixture/protocol, not merely ancestry.

Keep a native-verification candidate clean while its gate runs. Stage pending
release records outside that checkout until candidate construction finishes;
the .29 Python clean-candidate guard correctly rejected parent metadata edits.

### Requirements and acceptance criteria: first-wave scope and limits

- **20:** fix the real-repository preflight and commission-manifest all-LANDED
  assumptions, covering published/no-active-release and authorized terminal
  dispositions without weakening selected-work checks. The stale platform and
  archive metadata was repaired in .29; maintain it through normal publication.
  Also provide useful full diagnostics after a >200-line failure followed by later
  cancellation; preserve actual exit/completed/unrun outcomes and fail-fast
  callers. Monotonic timing in the remaining wrapper; shared qualified pinned
  OIDC normal/recovery promotion; bounded registry visibility with expected and
  observed values, no blind republish. Missing npm binary diagnostics must name
  actual resolution/native path. Inspect matching workers before production.
  Measure the 856.43-second Windows source-check hotspot and tool-only bootstrap
  before/after; keep meaningful workflow mutations and real Windows WAL tests.
  Fix `commission-manifest` rejecting a closed ladder with .29 owner-deferred
  120/unselected150: accept explicitly authorized terminal dispositions while
  rejecting unfinished selected work. Do not rewrite historical row statuses.
- **30:** inspect retained sample860/query360/op9971 before replay. Capture
  candidate IDs/distances/ranks/ties and snapshot/cursor on one bounded
  reproduction, with exact fixed-state control. Both original calls used the
  reference route; async projection is unproved. Check distinct-record/equal-body
  ownership and atomic repeated-ref feedback; review exact occurrence labels
  before selecting an ABI. Test known/unknown coverage, `R=K+U`, UTF-8 spans,
  pinned-reader/rollback publication and a discriminating malformed-open case.
- **40:** profile one current100k hybrid/SDK route and a relevant text/lifecycle
  control. Separate queue/service/eligibility/retrieval/hydration/rerank/wire cost
  and caller p50/p95/p99, useful throughput, refusal/timeouts and resources.
  Match snapshots/configuration and interleave baseline/treatment on the ordinary
  shared CPU host. Fix comparison rules before treatment; retain all outcomes.
  The .29 eligibility-scan48–49ms is a lead, not current .30 attribution.
- **50:** use existing rendezvous for queued cancellation, started timeout,
  cross-engine/path ownership and close/reopen with positive work and subsequent
  recovery. Fixture watchdogs are not total-close SLAs: retain database
  quiescence before provider-drain timing. Separate default Node JS/native live
  ownership/reservation from the prior semispace4 pair. Conditional delivered
  checkpoint faults move to55 and need their applicable concrete authority. Safe post-reset drop precedes
  exercising or shipping B30-01 opt-in model release; review concurrent use,
  repeated release, error parity and reload in Rust/Python/TS. Other safe work
  proceeds while a prerequisite is held. Orin requires Orin proof; no host reset.
- **55:** own the selected seq-406 cohesion/temporal/relationship fixture and
  bounded operation model, up to three product-input fuzz targets, missing CPU
  process-crash/WAL composition and17-doctor gap fixes. Reuse the fixture for thin
  SDK/MCP boundary assertions and selected oracle-sensitivity checks. Preserve
  independently authored state, valid-WAL witnesses and truthful async readiness;
  no new semantics, broad framework or power-loss claim.
- **60:** retain F30-REBASE/MARKER/MULTIARCH and .29 fixes; choose justified
  upstream and actual native/PTX targets, compare K1–K9 and retained/removed
  patches, preserve private-pool primitives independently of marker cleanup,
  measure cold-start/JIT/build/size and unsupported cases. Qualify inside each
  fork, then consume exact pins; no inferred new dtype, platform or CUDA13 claim.

Owner seq-407 right-sizes the remaining work without reopening completed slices.
[50](features/slice-50/plan.md) and [55](features/slice-55/plan.md) implement their
own test/fix scope. [70](features/slice-70/plan.md) qualifies integrated source;
[75](features/slice-75/plan.md) qualifies actual installed platforms/artifacts and
converges docs. Implementation owners update affected contracts with each change.
Freeze final support/version/native inputs before their applicable artifact proofs;
changes invalidate only affected claims. Do not repeat unchanged full gates or
duplicate engine tests across languages.75 does not grant publication authority.

Sol6.2-high drafted55/75. Direct Astra-medium passed50/55/70/75 and the
master/state split after one consolidated FIX-1; this is plan review, not
runtime verification or implementation commissioning.

Bug hunts30/50 each select named hypotheses and bounds before running; stop after
those cases and a discriminating follow-up unless new evidence justifies more.
Classify confirmed product defect, fixture/operator defect, expected behavior,
bounded non-reproduction or unresolved consequence. Do not require finding a
bug or rewriting already-correct code. Critical wrong-result/data-loss findings
need repair or explicit owner disposition; .29's S50 exception is not a general
score tolerance. No paid72/full160 campaign repeats for a release-number change.

### Reserved-gap work

Unassigned slice numbers remain reserved. A needed coherent follow-up requires
explicit scope placement; seq-407 inserts55/75 for the approved50/70 split. Broad cruft deletion, general version refresh and
Inspector integration remain postponed under seq-399. Scope gaps do not grant
implementation authority.

## C. Slice planning and execution contract

A direct `gpt-6.2-sol` high subagent drafts each20+ slice plan; this commission
writes plans only. The master is critically reviewed by direct Astra-medium,
with at most one FIX-1 revision and re-review before commit. Apply the same
single-FIX limit to each updated feature plan, reviewing coherent batches with
individual verdicts. Slice9's separate preparation loop permits up to four
FIX-n cycles. If a review still fails at its limit, preserve findings and stop
the affected transition instead of silently adding cycles or claiming PASS.

Each feature plan includes the following complete sequence for later execution:

1. **Entry reconciliation:** enumerate changes since drafting, related in-repo
   work, assigned functions and allocated drafts; update/approve/reject/adjust
   the plan against actual sources before relying on it.
2. **Slice prep:** state goals and non-goals; propose local needs, requirements
   and acceptance, inspect product architecture, draft design and challenging
   decisions, research primary-source approaches where necessary, check fit and
   revise. Record prerequisite work and who owns it for the next reviewer.
3. **Design:** update the real requirements/design/interface/ADR deltas needed
   for slice-complete behavior; direct Astra-medium reviews design additions.
4. **Implementation plan:** identify meaningful failing behavioral tests before
   RED→GREEN→refactor; preserve human oracles and real DBs. Direct Sol6.2-high
   reviews code. A directly spawned Terra independently verifies actual affected
   tests/artifacts; compile or symbol presence does not replace runtime proof.
5. **Updated-plan gate:** direct Astra-medium critically reviews the reconciled
   slice plan, one FIX-1 at most, re-review, then commit after PASS. This gate
   precedes implementation, not an extra gate for every helper. Then implement
   when the slice is commissioned and its dependencies/authority are satisfied.
6. **Closure:** record concise status and actual test exits/skips/limits, update
   affected SDK parity/docs and release state once, merge any temporary branch
   back into the owning release branch, and clean only owned inactive worktrees
   and scratch after needed evidence is retained. Reuse the existing release
   checkout for serialized work; isolate concurrent writers.

Spawn Astra/Sol/Terra directly; do not use Steward, Orchestrator, or an MCP agent
runner. Scoped lint/type/tests cover work and blast radius. Do not rerun full
regressions for a narrow finding correction; run the required full gate once on
a stable coherent source phase, then recheck failed/affected/unrun scopes. Broad
changes can justify another phase. Source qualification, hosted artifact proof
and post-registry consumers remain distinct.

## Cross-cutting DoD: testing and policy

Apply the [quality roadmap](../../notes/fathomdb-quality-testing-roadmap.md):
real-store state/property and round-trip tests, independent same-state oracles,
first/persistent failure and cleanup checks, positive concurrency progress and a
few targeted semantic mutations where assertions are in doubt. One deep owning
suite plus thin installed-binding checks avoids copied matrices. Review actual
suite duration, claim, useful defects/misses and overlap before consolidation.
**R30-TEST is an implementation obligation after Slice9 decisions:** selected
harness/CI test improvements belong to20, correctness/reference/property gaps
to30, concurrency/resource gaps to50 and composed data/recovery/fuzz/doctor gaps
to55. Slice70 accounts for every
selected gap and its owning fix or explicit disposition, real-store or independent
oracle, and a plausible wrong result that the oracle rejects. A consolidation
must preserve its meaningful semantic/platform coverage and show cost/overlap
evidence where that motivates the change. Proposal-only prework5 does not close
this requirement; no separate test framework or broad campaign is required.
Owner seq-406 adds the [coverage audit and gap allocations](coverage-audit.md):
one composed canonical/FTS/vector/graph temporal-lifecycle fixture, bounded
real-store state sequences, actual product-input fuzzing, selected CPU crash/WAL
composition, all-doctor outcome/effect coverage and thin behavioral SDK/MCP
parity. Ownerseq-407 splits implementation between50 runtime and55 data integrity,
and qualification between70 source behavior and75 installed platforms/artifacts.
Each begins only after its applicable commissioning; qualification requires
actual exact-candidate proof and dispositions. Existing40 latency scope remains
unchanged. Erasure retains lower-priority critical regressions. These additions
adopt the SQLite-method roadmap at FathomDB-owned boundaries, with bounded
resource/failure tests and independent oracles rather than new storage semantics.

The .27 swallowed errors, .28 vacuous cursor test and .29 rank/state and lost
failure observations explain priority. Defer global MC/DC, exhaustive VFS/OOM,
new generic frameworks and unneeded paid/platform campaigns. Do not weaken
accepted tests, numerical thresholds or security to make the release green.

The [CUDA policy refresh](research/cuda-version-adoption.md) is a planning input;
comparable usage shares remain unknown. Reconcile it in0/1/9 and60 under the
[standing policy](../../design/cuda-version-support-review.md); owner ratification
precedes changes. Keep R580/primary12.6. Check the [overflow tripwires](../../design/overflow-chunking-reevaluation.md)
against actual changes; no new chunking decision is inferred from old results.

Every selected behavior change updates its contract/ADR and changelog, with
Rust/Python/TS parity where affected. Public docs update navigation and pass
strict MkDocs when changed. At70 obtain actual applicable full/feature/security/
long/performance/source-behavior proof, and at75 installed platform/native and
contract/doc convergence proof, or explicit authorized dispositions.
At80 require trusted exact-tag routing and fresh installed persisted write/read,
missing-record, separate-process reopen, close/exit, registry finalization and
public downloads. Partial publication remains incomplete. Retain original
failures and honest unknowns; no force-retag or hook bypass.

## Immediate next slice and interactive boundary — historical record

The following records earlier phase status; the completed state and delivery
record above supersede its then-open qualification and commissioning notes.

Scope choices are ruled at seq-399 and preparation was commissioned at
seq-400. The reviewed Slice10 implementation is integrated on `release/0.8.30`;
its [status](features/slice-10/status.md) and [verification record](features/slice-10/verification.json)
retain the failed full gate, scoped correction and open qualification owners.
Preparation acceptance is IMPLEMENTED_WITH_OPEN_QUALIFICATION. Release
readiness remains NOT_READY and publication held. Slice20 was commissioned
at seq-401 and its reviewed implementation is now integrated at
`5e7906b6d7a871611ccdddd74a9ed1410c32cc47`. Its
[status](features/slice-20/status.md) retains the full-gate failure and the two
independent scoped fixture recoveries, plus open venue/qualification limits.
Slice30, commissioned at seq-402 and extended by seq-403 for
[issue #264](https://github.com/fathomadb/fathomdb/issues/264), is integrated at
`5b0348775279883fe2ffcd52f60c23c90171166e`. Its
[status](features/slice-30/status.md) retains the original failed wrapper/test
runs, passing focused recovery and final exact native proofs. Slice40 is
commissioned at seq-405 using the full execution template and its whole plan,
and its reviewed source work is integrated at
`d64e870bd96103851b104afc927d48c64067db35`. Its
[status](features/slice-40/status.md) preserves the original failed full gate and
independently passing observer recovery. Review findings are remediated; inherited
hints/final qualification remains open. Owner seq-408 commissions the revised50
plan with `gpt-6.1-sol` high after40 completion. Its reviewed runtime/logging
portion is integrated at`daf12b6d9839816fab4cd7c72c81e2f10918bf0f`, with both
remaining obligations open. Owner seq-409 supersedes the finish50-before55
condition. Reviewed55 is now integrated at
`3595192b0a3d4587556aa00b24768a06469af3ba`; its
[status](features/slice-55/status.md) records all six selected rows and the actual
strict verifier exit0,238 passed/3 explicit skips/0 failures, with the separate
outer-monitor124 preserved.60 is now activated with the full template under
seq-409; finish50 after its prerequisite without another sequence confirmation.
70/75 have not started;80 publication still requires separate exact-candidate
authority. Owner seq-404 places the Vercel UI SDK
research in the [0.9.0 backlog](../0.9.0/backlog.md); it adds no .30 feature work. The
[state](../release-state-0.8.30.json) and [board](../runs/STATUS-0.8.30.md)
own current status.

**Complete.** Selected implementation, qualification, publication and documentation are finished. See the [delivery record](features/slice-80/status.md) for actual outcomes and retained limits.

## Final landed source roll-up

<!-- BEGIN GENERATED release-state:0.8.30:plan-landed-roll-up -->
**LANDED on `origin/main`, in full:** Slices 0 (`7c9377b87db6bf0e93e9f489ecb8b28e6629733e`) · 1 (`7c9377b87db6bf0e93e9f489ecb8b28e6629733e`) · 2 (`7c9377b87db6bf0e93e9f489ecb8b28e6629733e`) · 3 (`7c9377b87db6bf0e93e9f489ecb8b28e6629733e`) · 4 (`7c9377b87db6bf0e93e9f489ecb8b28e6629733e`) · 5 (`7c9377b87db6bf0e93e9f489ecb8b28e6629733e`) · 6 (`7c9377b87db6bf0e93e9f489ecb8b28e6629733e`) · 7 (`7c9377b87db6bf0e93e9f489ecb8b28e6629733e`) · 8 (`7c9377b87db6bf0e93e9f489ecb8b28e6629733e`) · 9 (`9b60aed656330acdcce06d3c77e6008dbef739c3`) · 10 (`9c9b786585961d5393d1155eb6b44b611b63e06d`) · 20 (`5e7906b6d7a871611ccdddd74a9ed1410c32cc47`) · 30 (`5b0348775279883fe2ffcd52f60c23c90171166e`) · 40 (`d64e870bd96103851b104afc927d48c64067db35`) · 50 (`1550b96bb6e92c01deb08855ad3685c99f649f59`) · 55 (`3595192b0a3d4587556aa00b24768a06469af3ba`) · 60 (`3f738dcf38358dc9e111a28db13e093cd35225a3`) · 70 (`6d0e6c37b6cf35e3fecfe93ae3859a32d1b94a8f`) · 75 (`cd3a6caaa61fca8625f7468f4dae8341b958a241`) · 80 (`1b975b753b8ec09cfaf6985a5ec47a24c5d97584`). SCHEMA is 37; remaining ladder = none.<!-- END GENERATED release-state:0.8.30:plan-landed-roll-up -->
