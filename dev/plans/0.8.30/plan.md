---
title: FathomDB 0.8.30 release plan
status: ACTIVE
target_release: 0.8.30
date: 2026-11-17
---

# FathomDB 0.8.30 release plan

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
| 50 | Concurrency, lifecycle and resource bug hunt and remediation — R30-ROB | Approved 10; consumed 30 fixes; safe-drop fork prerequisite from 60 for unsafe release only | Positive concurrent work, capacity/cleanup/reopen assertions, default Node growth disposition and safe selected module-release behavior. |
| 60 | Scoped fork maintenance and qualification — R30-FORK | Approved 10 for design; consumed lifecycle contracts | Selected upstream/delta map, coherent pins, independent supported/unsupported kernel/output/resource controls and installed consumers. |
| 70 | Integrated qualification and release readiness — R30-QUAL/TEST | Selected exits 20–60 | Applicable source/security/long/performance/platform/artifact and docs checks plus independent review; every selected test gap has a discriminating oracle and owner; original nonpasses stay visible. |
| 80 | Authorized delivery and registry consumers — R30-REL | 70 and separate production authority | Every required publisher, exact-version installed consumer, co-tagging, normal finalization, public docs/download and owned cleanup completes. |

Detailed drafts live at `features/slice-N/plan.md`. Their requirements are
release-local proposals, not new canonical AC numbers. After 10, delivery20,
correctness30 and baseline40 can proceed independently where ownership permits;
60 design can run alongside safe50 work. Final fork/native inputs precede final
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
  checkpoint faults need concrete authority. Safe post-reset drop precedes
  exercising or shipping B30-01 opt-in model release; review concurrent use,
  repeated release, error parity and reload in Rust/Python/TS. Other safe work
  proceeds while a prerequisite is held. Orin requires Orin proof; no host reset.
- **60:** retain F30-REBASE/MARKER/MULTIARCH and .29 fixes; choose justified
  upstream and actual native/PTX targets, compare K1–K9 and retained/removed
  patches, preserve private-pool primitives independently of marker cleanup,
  measure cold-start/JIT/build/size and unsupported cases. Qualify inside each
  fork, then consume exact pins; no inferred new dtype, platform or CUDA13 claim.

Bug hunts30/50 each select named hypotheses and bounds before running; stop after
those cases and a discriminating follow-up unless new evidence justifies more.
Classify confirmed product defect, fixture/operator defect, expected behavior,
bounded non-reproduction or unresolved consequence. Do not require finding a
bug or rewriting already-correct code. Critical wrong-result/data-loss findings
need repair or explicit owner disposition; .29's S50 exception is not a general
score tolerance. No paid72/full160 campaign repeats for a release-number change.

### Reserved-gap work

Unassigned slice numbers remain reserved. A needed coherent follow-up requires
explicit scope placement; broad cruft deletion, general version refresh and
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
to30, and concurrency/resource/recovery gaps to50. Slice70 accounts for every
selected gap and its owning fix or explicit disposition, real-store or independent
oracle, and a plausible wrong result that the oracle rejects. A consolidation
must preserve its meaningful semantic/platform coverage and show cost/overlap
evidence where that motivates the change. Proposal-only prework5 does not close
this requirement; no separate test framework or broad campaign is required.
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
long/performance/platform/native checks or explicit authorized dispositions.
At80 require trusted exact-tag routing and fresh installed persisted write/read,
missing-record, separate-process reopen, close/exit, registry finalization and
public downloads. Partial publication remains incomplete. Retain original
failures and honest unknowns; no force-retag or hook bypass.

## Immediate next slice and interactive boundary

Scope choices are ruled at seq-399 and preparation was commissioned at
seq-400. The reviewed Slice10 implementation is integrated on `release/0.8.30`;
its [status](features/slice-10/status.md) and [verification record](features/slice-10/verification.json)
retain the failed full gate, scoped correction and open qualification owners.
Preparation acceptance is IMPLEMENTED_WITH_OPEN_QUALIFICATION. Release
readiness remains NOT_READY and publication held. The next proposed slice is 20;
feature slices20–80 remain uncommissioned. The [state](../release-state-0.8.30.json)
and [board](../runs/STATUS-0.8.30.md) own current status.

<!-- BEGIN GENERATED release-state:0.8.30:plan-immediate-next -->
**IMMEDIATE NEXT: Slice 20** (`CI`) — Efficient and diagnosable CI/CD

**Remaining ladder:** 20 → 30 → 40 → 50 → 60 → 70 → 80.<!-- END GENERATED release-state:0.8.30:plan-immediate-next -->
