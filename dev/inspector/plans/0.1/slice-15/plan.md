---
title: Inspector v0.1 Slice 15 — requirements and verification baseline
date: 2026-10-29
product: FathomDB Inspector
target_release: 0.1
status: COMPLETE
desc: Release-wide requirements, acceptance and test-verification planning
---

# Slice 15 — Requirements, acceptance and verification baseline

## Status and purpose

Draft baseline: `d05912c99`; execution source: `9486c5ba9`.
Baseline: **S15-2026-10-30**. State: COMPLETE for the reviewed planning deliverable;
requirements/verification baseline prepared and D-01 explicitly adopted. Dependencies: none.
This is the first preparation slice for the entire Inspector project. It
produces the requirements and verification baseline consumed by Slice 16 and
every feature Slice 20–60; it does not implement the application.

Scope includes every need, INS-01–09, INS-X01–X11, V01-N01–N07/S01–S04,
F-01–F-07 and T-01–T-05, including High investigation depth. All assessed Need
and Should work remains mandatory. Optional work stays in ROADMAP. The
Inspector remains FathomDB-native: readable/structured records, FTS/BM25,
metric-faithful vectors and interactive directed graphs with coordinated
identity. Documentation and use build understanding; no Inspector lesson or
narrative is introduced.

Engine startup is allowed under the
[current contract](../../../requirements.md#engine-startup-allowance-2026-10-29).
The Inspector-only source boundary is unchanged. Engine/MCP/SDK/schema changes,
application implementation, publishing and future-feature implementation are
outside this slice.

## Inputs and reconciliation

Read the [master plan](../../plan-0.1.md), needs, requirements, acceptance,
current design, historical reviews with their policy notes, compatibility
research and all five feature drafts. Follow the
[shared workflow](../slice-workflow.md) preparation-slice rules.

1. Enumerate changes since `d05912c99` to this draft, related repository code,
   interfaces/tests, allocated features and findings. Separate source-proven
   existing behavior from missing Inspector behavior and unsupported assumptions.
2. Evaluate each delta as keep, adjust, reject or already done with evidence and
   a short reason. Correct duplicates, contradictions and overbuilding before
   setting the baseline. Do not reopen settled owner decisions.
3. Review the [quality/testing analysis](../../../../notes/fathomdb-quality-testing-roadmap.md).
   Record applicable, deferred and changed-evidence recommendations in the
   verification plan; the recommendations do not automatically create gates.

## Requirements and acceptance work

- Reconcile the need-to-requirement mapping, conceptual distinctions and
  priorities. Keep Critical versus High as emphasis, not optional versus required.
  Explicitly account for deferred/incidental needs without promoting them.
- Make every required criterion observable and falsifiable: preconditions,
  action, expected facts, scope/context, failure/partial outcomes and evidence.
  Keep stable Inspector-local IDs; do not modify the global acceptance register.
- Settle D-01 through an explicit owner ruling before closing the affected
  disclosure criteria. Specify retained history/inactive data, owed physical
  erasure and telemetry bookkeeping separately. Record other genuine support
  or evaluation-policy decisions; engineering choices are not approval rituals.
- State the required support/resource envelope and which values remain proposals:
  platforms/browser/input scale, CPU-only baseline, model prerequisites,
  row/byte/work/time budgets, cancellation and shutdown. Architecture-specific
  implementation choices belong to Slice 16. Define pass/fail versus diagnostic
  measurements; do not quietly turn the formative four-of-five target into a gate.
- Define allowed Engine lifecycle-effect classes independently of Engine output.
  Distinguish no Inspector mutation commands, preflight preservation, effects
  after Engine startup/failure, worker progress and per-operation observations.
  Equivalent Engine runs help attribution but are not the sole correctness oracle.
- Complete identity/revision/cardinality, class-specific eligibility, provenance
  completeness, lexical route/effective tokenizer/scoring, and vector
  metric/population/selection/centering criteria. Missing facts remain unknown;
  a missing required positive capability is a blocker, not a passing unavailable case.
- Include local service origins/credentials and stored-content rendering,
  disclosure/privacy, no implicit downloads/cache repair, accessible keyboard
  interactions, faithful large values and installed launch/close behavior.
  Add or refine local criteria where existing wording cannot test these outcomes.

## Test and verification planning

Maintain [the shared verification plan](../verification-plan.md) as the
single release-wide test/verification plan and traceability matrix. It records planned methods, not an executed test suite. Reuse canonical
requirements/acceptance instead of creating another acceptance register.

For each mandatory criterion or coherent group, record: need/requirement/AC,
positive and boundary/negative cases, fixture/setup and independent oracle,
test layer/interaction, owning implementation slice, verification stage,
expected evidence, and blocked/implemented/planned status. No required criterion
may lack a responsible slice or a verification method. Later slices maintain
their rows and report actual evidence; a planned row is not a pass.

Plan these complementary layers, with expected target locations and runnable
commands selected when the implementation/toolchain exists:

| Layer | Required planning coverage |
| --- | --- |
| Unit/contract and property tests | References/identity scopes, exact revisions, pagination/round trips, typed request/result validation and unknown variants; meaningful properties for codecs/projection/round-trip work. |
| Real-database integration | F-01–F-05/F-07 contents, operational modes, lifecycle, origins, lexical routes and measured vector facts; engine-backed effects, preflight refusal, startup failure and generation changes. No mocked database. |
| Browser and accessibility | Actual document/tree, lexical, vector and directed graph interactions, selection/expansion/back navigation, keyboard access, limits, rendering untrusted content and loopback access controls. Payload or label assertions alone are insufficient. |
| Fault/resource and performance | F-06 limits, large values/high degree, cancellation/late results, model/native bounds, request ownership, worker/handle cleanup and shutdown. Specify aggregate bounds and failure model; distinguish UI cancellation from execution interruption. |
| Compatibility and installation | Approved producer/backend/platform combinations, current-schema and unsupported/malformed cases, stale UI/service/session rejection, packaged launch/attach/all views/exit. Source similarity does not qualify artifacts. |
| Human evaluation | T-01–T-05 documentation-assisted transfer to an unseen database, predefined rubric/assistance/participant selection, plus High investigation checks. Record proposed targets, owner decisions, unavailable participants and unperformed evaluation honestly. |

Define fixture construction and ownership before implementation: intended records,
identity collisions/revisions, ordinary incomplete provenance, collection modes,
erasure/disclosure, readiness/runtime-unavailable cases and cross-view links.
Use hand-checkable lexical cases, independent numerical distance calculations
and known directed adjacency; do not derive expected results from Inspector
output or regenerate golden ranks. Control Engine worker progress using supported
seams and explicit observations rather than race-sensitive sleep assumptions.
Plan teardown, seeds and replay for faults. New harnesses/probes needed later
belong durably under Inspector test/build wiring, not scratch-only scripts.

Adopt focused fault, resource, boundary/property and contract testing from the
quality analysis. Defer whole-workspace MC/DC targets, general SQLite fuzzing,
exhaustive OOM/VFS/power-failure campaigns and unapproved historical/platform
matrices; justify those deferrals from Inspector scope. Carry stable-contract
regression recommendations into the 1.x evolution assessment without building
future campaigns in v0.1.

## Review, downstream handoff and completion

1. Complete the canonical needs/requirements/acceptance updates and verification
   matrix, then directly commission **gpt-6.1-sol, high** review for omissions,
   circular oracles, scope drift and untestable criteria. Resolve material findings.
2. Give a direct **Terra** (`gpt-6.1-terra`) verifier the final documents to audit
   traceability, fixture feasibility, ownership, links and scoped doc checks.
   This verifies the planning deliverable; it does not certify runtime behavior.
3. Update Slice 16 and each feature plan with the exact prerequisite baseline,
   assigned criteria/fixture/test obligations and changes to its draft. Keep one
   matrix, not copied per-slice acceptance lists that drift.
4. Create a short `status.md` at execution with baseline, decisions/blockers,
   updated documents, review dispositions, verification and handoff/cleanup.
   Use the existing checkout; integrate/clean only any extra slice-owned resources.

**Completion:** Required outcomes have explicit coherent AC, every mandatory
criterion has a fixture/oracle/method/owner/stage, release-blocking policy and
support/evaluation decisions needed by this baseline are resolved, and all
successor plans consume it. Record unresolved required choices as BLOCKED,
not complete. No application code, artificial RED/GREEN or runtime-green claim
is required for this preparation slice. When feature code is implemented,
the existing TDD, Sol code review and Terra runtime verification remain binding.

Execution and review outcomes are recorded in [status](status.md). No
application implementation, RED/GREEN or runtime qualification is claimed.

## Execution delta decisions (2026-10-30)

| Item since draft `d05912c99` | Evidence | Disposition and impact |
| --- | --- | --- |
| Added preparation slices and shared workflow | Only intervening commit `9486c5ba9`; `git diff d05912c99..9486c5ba9` | Already done: retain 15→16 prerequisites and direct Sol/Terra planning review. No source/interface/test/pin changes occurred. |
| All five feature drafts/allocations | Plans/designs 20/30/40/50/60, unchanged feature baseline `037a8522e` | Keep required vertical document/tree, lexical, vector, directed graph and integrated UIs; adjust handoffs to shared matrix instead of copying acceptance. No existing Inspector app to credit. |
| Historical review findings and all Need/Should actions | Astra/Fable review/assessment and policy notes; N01–N07/S01–S04 matrix links | Keep substantive obligations; reject migration/prevalence, immutable-startup and mandatory shared-service premises already qualified by source/owner. Optional dashboard/historical-index/maps remain ROADMAP. |
| Existing Engine capabilities versus missing Inspector | Shared plan source evidence at `9486c5ba9` | Keep Engine startup allowance. Adjust proof boundaries: exact historical locator/full edge/cache-only and bounded native routes remain Slice 16 feasibility work, not source-proven APIs or runtime passes. |
| Requirements/AC/evaluation gaps | Canonical needs/requirements/acceptance; quality analysis | Adjust: explicit zero/one/many origins, effective lexical/vector evidence, class boundaries, security/a11y/installation/aggregate budgets; matrix M01–M39. Engineering support targets and fixed rubric are specified without promoting four-of-five to gate. |
| Operator disclosure D-01 | Explicit owner ruling 2026-10-30; canonical policy table | Adopt: history/inactive visible by default; `--no-show-inactive` hides them. Erasure-pending content refuses by default; `--show-erased` displays/exports still-retained content. Telemetry structure/counts default; `--show-payloads` permits payloads. Independent overlapping gates, all delivery paths; no reconstruction or general-export expansion. |
| Compatibility/evolution research | Pinned .28/.29 source comparison, architecture research/review and evolution stress test | Keep qualification of selected composition; reject implied multi-producer support from source equality, no historical shims/future support promise. Slice 16 consumes matrix and carries stable-contract recommendation through 1.x. |

Owner ruling during execution supersedes the prior open D-01 blocker. The exact
three launch flags/defaults and narrow erasure-pending export are now canonical
requirements, INS-X06/X11 and matrix M20/M39. Independent gates compose across
all display/evidence/copy/link/export paths; no additional confirmation or
general saved/export tooling is introduced. Final focused Sol/Terra review and scoped documentation verification passed;
see status for evidence and future runtime qualification boundaries.

Owner clarification (2026-10-31): the canonical D-01 export requirement now
applies only to legitimately existing readable retained-erasure content.
Physical deletion stays correct. This supersedes any unconditional positive
Engine fixture assumption in this historical Slice15 baseline; see current
requirements and M20/M39. Other positive requirements are unchanged.
