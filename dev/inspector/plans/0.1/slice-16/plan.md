---
title: Inspector v0.1 Slice 16 — execution plan
date: 2026-10-29
product: FathomDB Inspector
target_release: 0.1
status: ACTIVE
desc: Release-wide architecture and high-level design with evolution through FathomDB 1.x
---

# Slice 16 — Architecture and high-level design

## Status, dependency and scope

Draft baseline: `d05912c99`. Execution baseline: `728884bfe`; state and gate
evidence are in [status](status.md). Depends on completed
[Slice 15](../slice-15/plan.md) and its recorded requirements/verification
baseline. This is the second preparation slice; every feature Slice 20–60
consumes its architectural baseline before implementation.

Design the entire Inspector v0.1 release so every mandatory requirement has a
credible component/contract and verification route. Stress the architecture
against FathomDB 0.9, 1.0 and the rest of 1.x, distinguishing architectural
adaptability from supported-version promises. Inspector's own release numbers
are independent. Do not implement future features, historical readers, plugin
frameworks, Engine/MCP/SDK/schema changes, or an application feature here.

## Inputs and reconciliation

Read Slice 15's outputs, canonical requirements/AC, shared verification plan,
existing [high-level design](../../../design/high-level-design.md), feature
drafts, relevant ADRs/interfaces and actual selected dependency code. Include
[compatibility research](../../../research/compatibility-architectures.md),
[its Astra assessment](../../../reviews/compatibility-architecture-review.md),
[.28/.29 source comparison](../../../design/compatibility-0.8.28-0.8.29.md) and
[evolution stress test](../../../research/compatibility-evolution-stress-test.md).
Historical immutable/no-startup conclusions are superseded by the owner allowance.

Enumerate changes since this draft and since the completed Slice 15 baseline:
plan/design edits, related repository work, assigned features and allocated
findings. Evaluate keep/adjust/reject/already done with source evidence. Resolve
contradictions before choosing the architecture; distinguish callable public
interfaces from private/test-only helpers and proposed sibling-branch work.

## Release-wide design work

Update the shared high-level design as the canonical architectural contract;
use this slice's [draft design](design.md) for working guidance. Avoid parallel
architecture documents carrying divergent definitions.

| Area | Required design output |
| --- | --- |
| Requirement satisfaction | Map all mandatory INS/V01 and cross-cutting criteria to components, operations, payload invariants and Slice 15 verification rows. Demonstrate actual document/tree, lexical, vector and directed graph workflows and coordinated navigation; no grid-only substitute. |
| Product composition | Select and justify UI/service/runtime, application module boundaries, launch/build/package shape and declared initial platform envelope. Prefer small modules/ordinary DI; choose new interfaces only for demonstrated ownership/variation needs. |
| Engine admission/lifecycle | Select an unchanged Engine startup route/options, product-lock ownership, preflight/schema/initial-sidecar behavior, permitted effect classes, startup failure, model/cache controls, owned shutdown and external-writer support conditions. Never use immutable SQLite against a changing Engine. |
| Data/domain contracts | Specify database/session/class/namespace/revision references, operational modes, origin cardinality, anonymous/ambiguous data, completeness, lifecycle/eligibility and disclosure. Define exact versus current resolution and how observations evolve with workers. |
| Catalog and retrieval | Define actual catalog/detail/link reads, lexical indexed-content/routes/effective tokenizer/scoring, vector representation/metric/direction/population/selection/centering, graph direction/edge detail, provenance and retained revisions. Separate configuration, stored materialization, prerequisites and request evidence. |
| Compatibility and protocol | Define bundled UI/service pairing/session checks, additive versus incompatible fields/variants, normalized operation-specific payloads, error/unknown states and qualified storage-reader/query-backend compositions. A schema number or producer string alone is insufficient. |
| Execution/resources/security | Define connection/request/scratch/native-model ownership, aggregate work/time/byte budgets, cancellation versus interruption, late results, detach and leaks. Include loopback authentication/origin checks, untrusted stored-content rendering, privacy and accessibility responsibilities. |
| Verification integration | Bind each architectural claim to planned independent fixture/contract/browser/resource/installation checks; feed missing cases and necessary criterion clarifications back into Slice 15's canonical matrix. |

Build an all-feature dependency capability map at the selected source/build
baseline: callable interface, supported operation/meaning, startup/runtime
prerequisites, evidence limits, Inspector-local work and owning feature slice.
Include exact retained-revision reads, complete edge payloads, lexical evidence,
metric-faithful neighborhoods, cache-only loading and finite execution ownership.
Engine startup being allowed does not establish these capabilities. Prefer
reuse over copied private implementations or a second ranking/eligibility engine.

Check feasibility with source/contracts and existing tests/builds where useful.
Label unexecuted claims. If an essential route needs a new Engine API or other
out-of-scope change, record a concrete blocker and the smallest separately
scoped proposal. Do not declare the architecture complete while a required
capability has no feasible Inspector-only route. New prototype/production
implementation is not silently commissioned by this planning slice.

## Architecture through 1.x

Produce an evolution table in the shared design: change scenario, affected
boundary, preserved invariants, required contract/view changes, compatibility
response, qualification evidence and future decision trigger. At minimum cover:

- Changed storage layout/encoding/schema with stable user concepts.
- Changed eligibility, tokenization, fusion or score meaning with unchanged
  table shapes; distinguish serving query semantics from historical producer behavior.
- Multiple representations/models, new vector metrics or ANN selection, passage
  results and zero/one/multiple verified origins.
- New identity scopes, lifecycle variants or genuine new data structures needing
  domain/protocol/renderer changes; no lossy generic DTO/default coercion.
- Different native/async runtimes and cancellation/resource ownership.
- UI/service restart or independent release and expiration of references.
- Potential live/WAL, encrypted/remote/non-SQLite access requiring new consistency
  contracts; these are scenarios, not v0.1 promotions.
- Stable public contracts and bounded support windows through 1.x: what is public,
  how persisted meaning is distinguished, supported producer/reader/query pairs,
  deprecation/change policy and how fixtures grow with actual commitments.

Document the failure limit: indistinguishable stored bytes with different
undocumented meanings cannot be disambiguated by capability flags. Reuse the
existing authoritative format/version facts; a new manifest or Engine metadata
requires separate scope/contract authority. A static generated build description
or queryable Engine catalog is an option to assess, not an assumed existing API.

Decide what v0.1 must establish now (honest references, typed meanings, lifecycle,
qualified compositions, bounded ownership and protocol pairing), what needs a
later contract revision, and what remains a support-policy decision before 1.0.
Do not add historical shims or parallel public API generations contrary to
accepted pre-1.0 ADRs. Keep optional expansion in ROADMAP with promotion triggers;
no claim that one current binary automatically reads every 1.x database.

## Review, downstream handoff and completion

1. Directly commission an independent **gpt-6.1-sol, high** design review of
   shared and slice design changes against Slice 15 criteria, actual code and
   the Inspector-only boundary. Ask it to falsify requirement satisfaction,
   reader/query semantic agreement, lifecycle and evolution assumptions.
   Resolve material findings and recheck changed areas before handoff.
2. Use direct **Terra** (`gpt-6.1-terra`) verification for traceability,
   source-backed capability evidence, scenario coverage and scoped checks.
   Existing executable checks may support specific feasibility claims; no
   application/runtime compatibility qualification is implied by document review.
3. Reconcile each feature plan/design against the architecture and shared
   verification matrix. Assign contracts/components/test cases; remove choices
   now settled centrally and preserve feature-local refinements. Record affected
   criterion changes and repeat only relevant reviews/checks.
4. Write a short `status.md` at execution: chosen baseline/decisions,
   requirements/design updates, feasibility evidence versus unverified claims,
   review/verifier outcomes, blockers, downstream handoff and owned cleanup.
   Use the existing checkout; integrate and clean only extra slice-owned resources.

**Completion:** Every required release behavior maps to a coherent implementable
Inspector-owned design and verification route; no unresolved required capability
or architectural policy blocks implementation; all five feature drafts consume
the baseline. The 0.9/1.x evolution table exposes real break points and policy
choices without claiming future compatibility. Unresolved required feasibility
is BLOCKED, not architecture approval by aspiration.

No application code or artificial TDD is needed for a design-only slice.
Feature implementation remains RED/GREEN with fresh Sol-high code review and
Terra runtime verification. The original request populated drafts only. The 2026-10-30 execution selects
the shared architecture; its completion is recorded in status, not inferred
from this plan.

## Slice 15 execution handoff — S15-2026-10-30

Consume [canonical acceptance](../../../acceptance.md),
[requirements/envelope](../../../requirements.md#verification-and-operating-envelope-slice-15-2026-10-30)
and [the single verification matrix](../verification-plan.md), source baseline
`9486c5ba9`. Check [Slice 15 status](../slice-15/status.md) before starting:
D-01 is adopted on 2026-10-30; implement its exact launch flags, independent
overlap gates and narrow retained-erasure export. This handoff does not waive
predecessor completion. Original draft baselines remain
for later delta reconciliation.

Assigned matrix rows: **M01–M39**. Map all rows to feasible unchanged dependency routes/Inspector-owned components; fix exact Engine/build/options, package/browser versions and enforceable resource ownership. Validate F-04 control seams, exact revisions/full edges/cache-only feasibility. Feed contract changes back to canonical AC/matrix. Carry stable-contract regression policy through 1.x.

This adjusts the draft by adding exact fixture/oracle/layer obligations,
INS-X07–X11 and explicit aggregate resource/security/accessibility/installed
contracts. It does not implement them. Preserve feature-local TDD and review;
replace planned methods with named tests/commands and executed evidence as the
feature is built. Required positive gaps block completion, even where an
unavailable/unsupported negative case works. No competing acceptance list or
Engine/MCP/SDK/schema/pin work is authorized.

D-01 final handoff: apply [the adopted policy](../../../requirements.md#adopted-operator-disclosure-policy--d-01-2026-10-30)
and matrix M20/M39. Slice 20 owns launch parsing/help/defaults, common gate
enforcement and narrow local export; 30/40 apply gates to their representations/
results/evidence; 50 extends history/graph/provenance and overlap cases; 60
qualifies all eight flag combinations and export in the installed artifact.
No flag alters Engine eligibility/ranking, restores physically absent content
or bypasses another applicable gate. No general export tooling is added.

## Execution delta decisions — 2026-10-30

`git diff d05912c99..728884bfe` contains documentation only: `9486c5ba9`
added preparation drafts, `7d7d4ea54` completed requirements/methods, and
`728884bfe` adopted D-01 and closed Slice 15. No source/dependency implementation
landed after the draft; the Inspector remains absent. Historical reviews and
compatibility research remain evidence with later Engine-startup policy notes.

| Enumerated item | Evidence / decision and impact |
| --- | --- |
| M01–M39, INS-X07–X11, F-04/06/07, zero/one/many origins | Slice 15 status/matrix; ADJUST draft to exact source routes, aggregate ownership and native UIs in canonical HLD. No required view deferred. |
| D-01 disclosure and export | Owner ruling + `728884bfe`; ADOPT, never reopen. All eight flag combinations compose independent gates on all paths. |
| Engine startup / query runtime | Unchanged open/search/runtime code; KEEP allowance, ADJUST with current-layout/marker/mean guards, child/helper finite ownership, explicit effects and cache-only Caller constructor. |
| Full edges/exact revisions / vectors | Public read/graph/search shapes remain insufficient; ADJUST to bounded current-layout S readers and separate stored-population L2 inspection, never copied serving rank/eligibility. |
| Retained erasure positive M20/M39 | REJECT fabricated closure/body seed: unchanged physical closure demands zero residue before commit. B16-01 initially blocked completion. Owner resolved it on 2026-10-31: only if content legitimately exists; physical deletion stays correct. Canonical HLD records source facts and conditional verification scope; no dependency proposal is commissioned. |
| MCP/.29 snippet/degradation reuse | Pinned comparison; REJECT unavailable sibling imports and forced shared session. ADOPT unchanged public Engine compilation/FTS/fusion/evidence/trace. |
| Framework/package/protocol | Previously unsettled; SELECT Rust supervisor/session/helper, plain DOM/SVG ES modules, one Linux tar, exact bundled pairing and local one-use credential handoff. |
| Historical formats, plugins, parallel APIs / .9–1.x | ADR fresh/current-only precedent and research stress test; REJECT overbuild, KEEP explicit evolution break points/qualification/support-policy triggers. |
| Existing drafts/review actions/feature allocations | Read all seven plans/designs and historical Need/Should actions; KEEP full release scope, reconcile all five feature handoffs to HLD components and M rows. No existing application work counts as done. |
