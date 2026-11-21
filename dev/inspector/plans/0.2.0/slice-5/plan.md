---
title: Inspector 0.2.0 Slice 5 — verification adequacy review proposal
date: 2026-11-19
status: DRAFT
target_release: 0.2.0
desc: Plan a source-grounded traceability and test-strength review without executing campaigns or repairs
---

# Slice 5 — verification adequacy

Current disposition: owner S9-010, applied in [Slice 10](../slice-10/status.md),
qualifies supported single head windows in [0.2.0 scope](../scope.md) and defers
within-source multiple-window positive. Earlier findings/draft proposals below
retain their historical meaning; cache-only CPU Engine serving remains mandatory
and unqualified in 40.

The [owner checkout decision](../slice-0/checkout-decision.md) requires future
Slice 0 to create and enter the release branch/worktree from the approved final
planning HEAD. Reuse that release checkout sequentially for 0–9 reports, with
one writer. Other environment/infrastructure changes remain proposals for
actual Slice 9 disposition and conditional Slice 10 implementation. Recording
this ruling and updating drafts performs no checkout creation or slice execution.

## Goal and boundaries

Produce a review/proposal showing whether the planned and existing tests can establish the [adopted scope](../scope.md): an accurate mental model through canonical records, actual representations, native retrieval/graph interaction, two observations and safe bounded operation. Test existence, executed success and product qualification are separate facts. Slice 5 completes the review deliverable; it cannot complete the product by auditing it.

This is a DRAFT plan/design only, inspected 2026-11-19 at planning branch `llm/0.8.29-inspector-additional`, scope adoption `609dd85c41fb7980e05faa1132a248d376d8ec89`, unchanged Engine base `6edc1155f`, workspace packages 0.8.28/current format 37. The unpublished Inspector package remains 0.1.0. No tests, reviews, campaigns or changes to the source checkout were executed for this draft.

In: needs → requirements → AC → meaningful tests → scoped execution evidence; product-goal/critical-path adequacy; positive, failure, browser, keyboard, real-database, installed and human coverage; prioritized proposals with evidence confidence and owners. Read-only evidence may extend outside Inspector to locate dependencies.

Out: executing runtime suites/campaigns, adding tests or tools, changing product/source/build/environment, modifying Engine/SDK/MCP/schema/ranker, changing pins, weakening accepted criteria, broad fuzzing/coverage programs, or repairing findings. External dependencies are reported with their owner; this slice does not commission their implementation. Human activity and numerical benchmarks are never simulated.

## Inputs and prerequisites

- Read [needs](../../../needs.md), [requirements](../../../requirements.md), [acceptance](../../../acceptance.md), [HLD](../../../design/high-level-design.md), [local design](design.md), [master plan](../plan.md), [shared workflow](../slice-workflow.md) and outputs allocated from Slices 0–4. Master/workflow links are intended integration destinations; they were not present in the inspected draft checkout.
- Reuse the **one** [M01–M40 verification matrix](../../0.1/verification-plan.md), F-01–F-07 builders/oracles and T-01–T-05/High rubric. Legacy numbers are historical; use the current master mapping: feature 20 foundation, 30 lexical, 40 vectors/models, 50 graph/history, 60 comparison/person workflow, 70 final integration/installed/human. Old active Slice 20 does not prove new 20 complete.
- Read [port qualification](../../0.1/compatibility-0.8.29-port.md) and [Astra review/remediation](../../../reviews/compatibility-0.8.29-port-review.md), keeping imported historical evidence separate from current-source execution.
- Read the [quality roadmap](../../../../notes/fathomdb-quality-testing-roadmap.md), accepted [vector identity ADR](../../../../adr/ADR-0.6.0-vector-identity-embedder-owned.md), [explicit maintenance ADR](../../../../adr/ADR-0.8.28-explicit-erasure-maintenance.md) and relevant [Rust interface](../../../../interfaces/rust.md). Follow applicable AGENTS.md and actual memory: `dont-be-distracted-by-ceremony.md`, `verify-design-against-code-not-just-architecture.md`, `tdd-red-must-fail-per-finding.md`, `keep-measurement-tools-in-repo.md`, `slice-prompt-verify-test-claims.md`, `conformance-rewrite-vacuous-green-trap.md`, `perf-recall-gates-masked-and-ac013b-conflation.md`, `full-gate-run-frequency.md`.
- Future execution reuses the release checkout created and entered by Slice 0 under the linked owner decision, preserving approved planning HEAD, port/scope/plans and Engine ancestry. Slice 5 creates no replacement checkout.
- Required inputs may be incomplete. Record the exact missing baseline/allocation and its consequence; continue independent read-only analysis. No cached CPU assets, participants, GUI or Python environment are needed to review source. Their absence remains a future positive-qualification prerequisite.

## Preparation at future execution

1. Enumerate changes since this draft in scope, canonical IDs, HLD/routes, tests/harness commands, fixtures, port evidence and Slices 0–4 allocations. Read each test **fully** before making a load-bearing coverage claim, including inline `#[cfg(test)]` modules, feature gates, `#[ignore]`, platform guards and early successful returns.
2. Evaluate each changed assumption/item as keep, adjust, reject or already done, with source/revision and reason. Reconcile stale schema-34/35 assertions and legacy 60 ownership with format 37/current master; do not silently rewrite historical evidence. Revise this plan/design only as necessary for a complete bounded review.
3. Read the requirement and test from both directions: every in-scope need/requirement has observable AC, every AC has an adequate proposed/existing check, every claimed test asserts the behavior, and every execution claim identifies actual artifact/build/features/command/outcome. Note explicit deferrals/incidental needs without inventing missing scope.

## LOCAL DRAFT needs/requirements/AC and design allocations

These are proposed clarifications to existing contracts/methods, not a second acceptance register or approval to edit canonical acceptance. Slice 3 owns NRA reconciliation; Slice 4 owns architecture proposals; Slice 9 decides substantive changes, Slice 10 implements only individually approved prework items. Features own their ordinary local tests.

| Local draft change | Existing IDs / observation to require | Proposed owner |
| --- | --- | --- |
| No new user need: strengthen evidence trace | UN-01–10/12–16; INS-01–09/X01–11/V01 and T tasks: enumerate requirement-without-AC, AC-without-test, weak oracle, unexecuted/skipped and unavailable scope explicitly rather than label all “covered” | 5 proposal → 3/9; matrix edits only if approved for 10 |
| Make format-37 representation checks explicit in methods | INS-04.1/.2, INS-05.6, INS-08.4, INS-09.3/.5; M11/17/23/26: multiple vector rows/windows per canonical cursor, exact byte range/window/profile/revision/origin cardinality; not one row = one record | 40, foundation references in 20 |
| Carry queue freshness/effects into new paths | INS-X01/X04/X06/X11; M20/29/39/40: real ExplicitMaintenance frozen scope, per-read-transaction cache, fresh request after supported test-owned withdrawal, independent I/E/T overlap and no production maintenance action | 20 shared policy, each 30–60 delivery path, 70 installed |
| Allocate comparison tests to adopted AC, no replacement IDs | INS-05.7–05.10; M40: actual pinned observations, typed membership/rank/own value/context, no rerun/shared-snapshot assumption, policy/cap/late/evicted failure cases | 60, foundations 20 and inputs 30–50 |
| Tighten evidence scope, no new runtime gate | INS-X03/X07–X10; M30–38: distinguish synthetic pause from native interruption, sampled peak from peak guarantee, package help from installed interaction, no-GUI refusal from native positive, unavailable model from required cached CPU success | 5 proposal → 9/10 for approved method corrections; local tests 20–60; qualification 70 |

## Source-backed audit starting points

These observations select future work; they are not a completed adequacy verdict. Paths below are relative to [Inspector source](../../../../../src/inspector/README.md). High confidence means inspected source/explicit execution record; it does not mean newly executed pass.

| Existing matrix rows / known critical path | Inspected evidence and limit | Audit/proposal owner |
| --- | --- | --- |
| M04/29/33/35/36 — launch → guarded attach → owned Engine → close | `tests/integration/{access,preflight_guards,session}.rs`, `tests/{worker,launch}.rs`, `tests/contracts/build.rs`; high confidence in named assertions. Dirty coalescing refuses before open and in session; native test launches browser then SIGTERM, not all-view window-close qualification | 20 foundation; 7 packaging/environment interfaces; 70 installed/native close |
| M01–07/09–13/15/17–18/24/32 — catalog → exact full record/tree → source/attribute → Back | `tests/integration/{associations,eligibility}.rs`, browser `document.mjs` inspected fully: real fixture, Enter selection/expansion, exact clipboard, restored focus, no image/external request. This is a foundation subset, not every native view/accessibility fact | 20 revalidation; 30–60 own extensions |
| M20/39 — D-01 across every delivery/copy/export | `tests/contracts/policy.rs`, `tests/integration/disclosure.rs` fully inspected: eight launches, atomic absence, real queued target and export, unrelated source, 8,192 targets/200 unrelated and withdrawal freshness. `session.rs` bounds one cursor set per read transaction; SQLite interruption test asserts `budget_exhausted`. High source/record confidence; new representations/pins remain unqualified | 20 shared, 30/40/50/60 each path, 70 installed |
| M16/21/22/25 — real index → lexical request → match/score → exact origin | Worker `Request` currently has record operations, not lexical search. HLD capability claims include historical routes; current scope names newer snippet/evidence APIs. Positive lexical UI/numeric/origin cases remain feature work, not negative-unavailable passes | 4 feasibility overlap; 30 methods/tests |
| M11/17/23/26/30 — stored windows/profile → Find similar/L2; cached CPU semantic/hybrid | `_fathomdb_vector_rows`/`_fathomdb_vector_spans` source plus port format-35→37 notes require one-to-many origins/windows. Current record associations are scalar attributes/declarations, not vector qualification; current Cargo defaults exclude model integration. Required model positive remains unimplemented scope | 40 with 20 typed refs; 1/7 dependency/package prerequisites only |
| M08/18/19/27/38 — directed adjacency → complete edge → lifecycle/provenance/dependency/revision | Existing raw temporal test checks integer node half-open bounds and edge invalid-time separately. It does not prove interactive graph/dependency/history. F-07 self-loop/parallel/cycle/disconnected adjacency must be independently specified | 50 functional/browser; 70 High human |
| M15/28/40 — person workflow + two original observations | Comparison INS-05.7–.10/M40 exists canonically, PLANNED. `cache.mjs` and `response-order.mjs` fully inspected: actual response held/released, owned-Engine transition, transient DOM leak monitor. They prove useful seams, not pin lifecycle/comparison behavior | 60, with each contributing feature |
| M31/32/36 — cancel/late/native/resource/queue/detach | `resources.mjs` fully inspected explicitly labels synthetic owned blockage; checks queue 4 busy/9 cancelled, acknowledgement ≤250 ms, reaping/lock release. Three `worker.rs` scale/allocation and one `launch.rs` GUI cases are ignored; port did not rerun them. Existing `qualify-scale.mjs` is the native-catalog extension, not evidence of native query/model/graph interruption | 20 foundation; each feature; 70 aggregate |
| M33/34/35 — auth/origin/build pairing/inert content/keyboard/installed | `package.mjs` fully inspected builds/extracts default release, checks help/description/checksum and no dev runtimes; explicitly separates native GUI. Document browser covers inert fixture subset; inspect malicious payload/render sinks and negative request tests for each new view. No final distribution/all-view pass | 20–60 local; 7 build pairing remedy proposals; 70 installed |
| M14/28/37/38 — mental-model transfer + High investigation | Existing rubric requires five real developers/agent builders, familiarization then unseen instance and documentation-only assistance; High trace/check task separate. No participant evidence in port; this remains mandatory activity, 4-of-5 only formative diagnostic | 70; 6 documentation inputs |

## Future review method and bounded deliverable

1. Walk **every M01–M40 row**, canonical criterion and V01/F/T link, not just the starting table. Check the product outcomes together: distinguish derived records/projections, identify stored content and representations, locate and follow actual results/relations, compare faithfully, explain only observed readiness/eligibility/selection, remain responsive/safe. Classify trace gaps and test failures separately.
2. Extend existing matrix cells or link concise slice findings from them; do not create a second exhaustive register. For each gap use existing row/AC, exact test/assertion and source revision, why it could miss the violation, confidence/uncertainty, smallest proposed check, prerequisite, urgency and owner. Record “requirement has no AC” even if it has a suggestive test; record “AC has no meaningful test” even if files exist.
3. Distinguish source-inspected, imported historical pass, current-source recorded pass, unexecuted, ignored/gated, partial, blocked and legitimate unavailable/not-applicable. Do not change a runtime state to pass during this prose review. Evaluate assertion strength by naming a plausible wrong implementation it would detect; flag duplicate predicates, shared adapter outputs, label-only/browser-payload-only checks and automatically regenerated expected data as false-oracle risks. Do not report a hypothetical weakness as a confirmed defect.
4. Specify feasible future real-DB checks using intended fixtures created before Inspector output. Lexical: tiny written token corpus/effective route and score formula or explicitly limited raw FTS conformance. Vector: independent f64 L2 over stored f32, declared tolerance/tie classes, population/centering/window/origin checks. Graph: authored directed edge IDs/endpoints/content including self/parallel/cycle/unavailable. Identity/codec/chunk/pagination: bounded properties, collisions, Unicode, large exact numbers, limit−1/limit/limit+1. No database mocks or generated goldens.
5. Require all eight launch flags and independent I/E/T intersections for each applicable new representation/evidence/snippet/label/link/copy/export/pin path. Keep history default, active queued targets not falsely inactive, clean coalescing and per-transaction freshness. Autonomous absence is a correct negative; current ExplicitMaintenance has a legitimate positive, so historical “no retained E exists” is not a current blanket waiver. Never queue/delay/reconstruct in product to manufacture a test.
6. Propose focused first/persistent attach/model/request/response/close faults, actual owned SQLite interruption, late completion after cancellation, close during active native work, queue 1+8/excess busy, repeated navigation/eviction and bounded scratch/handle/lock/worker cleanup. Separate UI acknowledgement from quiescence/native termination. A process kill proves its failure model, not power-loss recovery. Add a few targeted wrong-condition/missing-cleanup test-strength probes only if useful and approved later.
7. Pair service assertions with actual keyboard/pointer UI selection, graph expansion/collapse, labeled metric values, Back, pin/replace/unpin, cache expiry and stale/refused/evicted behavior. Model tests require valid compatible cached CPU assets under calibrated network denial and no cache creation/repair, plus missing/partial/corrupt negatives and independent browsing. Security includes Host/origin/cookie/build/session rejection before reads, inert stored HTML/scripts/URLs, no automatic fetch, and payload/query/credential-free logs/errors/URLs.
8. Plan default-feature installed offline artifact qualification on the exact unchanged Engine/current format/browser/OS pairing, distinct from driver/fixture-control or development artifacts. Include all views, default launch flags, cancellation and native application/window close, owned resource release, no dev server/Internet and human unseen transfer/High investigation. Preserve Ubuntu 22.04-compatible distribution target; current glibc 2.39 development output does not qualify it.
9. Read the quality roadmap assessment into recommendations: adopt independent intended state, narrow property/boundary/fault/resource checks and useful branch/mutation diagnostics. Changed evidence is the real queue-budget defect, browser credential boundary and multi-window/current-format composition. Defer full SQLite fuzzing, global MC/DC, all-allocation/VFS/power-loss sweeps, relevance/ANN campaigns and unbounded platform matrices. No coverage percentage becomes a new gate.
10. Deliver concise findings and matrix-method/ownership proposals for 9: understood evidence, correctness/security/resource risk, confidence, effort band and include/postpone reasoning. Route approved infrastructure/method repairs to conditional 10 individually; required feature tests to 20/30/40/50/60 and final qualification to 70. Missing public routes or model assets are prerequisites/blockers, not implicit permission for external changes. Coordinate shared findings with 3/4/6/7 to avoid duplicate work.

## Review, verification and completion on execution

- One directly commissioned Slice 5 agent is sole writer in the selected report checkout, sequentially. Read-only reviewers may share it. Genuine design additions receive direct fresh `gpt-6.1-sol-high` design review; use a direct fresh Sol-high review of substantive deliverable correctness and Terra verification of traceability/doc checks. They review proposals; they do not implement repairs or run runtime campaigns. No Steward/Orchestrator and no reviewer chain.
- Docs-only work has no artificial RED/GREEN. Any approved later behavior fix gets its own genuinely failing RED run/commit, then separate GREEN with unchanged spec tests, independent code review and appropriate verification. Durable measurement tools introduced then live in Inspector's committed tools/tests index, not scratch-only receipts.
- Run applicable prose/link/traceability checks, `./scripts/agent-lint-md.sh` for changed Markdown, and `git diff --check`; no full workspace/runtime gate merely for this report. Verify links resolve after integration and that every proposed item has an existing row/criterion or explicit NRA gap. Terra verifies scope/state distinctions and no invented results, not product success.
- Port full `./scripts/agent-verify.sh` stopped at missing checkout-owned `.venv/bin/python`; no full green carries forward. Slice 0/7 proposals must resolve that prerequisite before future required code gates. Never borrow another checkout's environment; do not install/repair it in this slice. Feature code phases still require their final applicable full gate; focused fixes rerun only affected checks unless blast radius/required final gate warrants more.
- On execution create one concise Slice 5 status with inspected revisions, changed assumptions, deliverables, scoped checks, findings/prerequisites and owner handoff. COMPLETE means review/proposals and checks complete, not all M rows pass. Unperformed required positives remain unverified/blocked at their owning slice.
- Preserve release checkout and unrelated work. Integrate and remove only Slice 5-owned extra isolation after verified integration if one was necessary; no extra worktree by default, no primary-checkout execution, destructive cleanup, hook bypass, force push, tag or publication.

## Future execution prompt

**Prework override:** Slice 5 implementation means completing this read-only review/proposal and document deliverables only. The generic wording below authorizes neither proposed tests/campaigns nor source/environment changes. TDD/code-review/testing clauses specialize to prose correctness and scoped document checks; do not invent runtime RED/GREEN. Future repairs require Slice 9's actual individual decisions and 10/feature ownership. Do not execute this prompt during drafting.

```text
Work on and complete Slice 5 for Inspector inspector/dev/0.2.0 (and in a worktree).

Ensure the slice has planned to do any needed work regarding requirements+acceptance criteria, design, implementation, and testing; each should be slice-complete and correct. The slice might have slice-related "change plan" documentation for guidance. The slice's plan should:
- identify and enumerate changes since slice's plan was written (review drafted slice plan, review in-repo related work done, review assigned feature/functions, review allocated draft items).
- Evaluate the enumerated output, and if needed, update/approve/reject/adjust the drafted plan, ensure it is complete but not overbuilt.
- add/adjust needs, requirements, acceptance criteria.
- write/update design (design additions and changes should be reviewed by a design review subagent (gpt-6.1-sol-high).
- The plan should then describe implementation with TDD (RED/GREEN method) and code review subagent (gpt-6.1-sol-high).
- Use a subagent (terra) to test/verify.
- Write Slice status record. Clean-up after itself: While a wt or branch should not be necessary for a slice, if a new worktree or branch was used, merge back and clean-up the worktree.

Keep ceremony to a minimum. Stay focused on the work-at-hand. When resolving a specific finding, do not run full regressions unless the changes are comprehensive enough to warrant the overhead. Generally, solve for the work outlined in the slice; check for impacts in the blast radius; be pragmatic. (do not use Steward or Orchestrator.)

After the planning for requirements+acceptance criteria, design, and implementation is planned: implement the slice according to the plan.
```
