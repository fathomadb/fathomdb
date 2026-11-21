---
title: Inspector 0.2.0 Slice 3 — product contracts and architecture allocation
date: 2026-11-19
status: DRAFT
target_release: 0.2.0
desc: Local draft CRUD proposals and complete delivery ownership, without canonical edits
---

# Slice 3 — draft product contracts and architecture allocation

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

## Goal and authority

Produce a compact local DRAFT change plan for Inspector user needs, requirements,
acceptance and architecture, allocating every proposed change to one delivery
owner. This is product-document CRUD, never database CRUD. This planning document
does not perform the future Slice 3 review or commission feature execution.

Inspected planning baseline: adoption `609dd85c41fb7980e05faa1132a248d376d8ec89`
on `llm/0.8.29-inspector-additional`, unchanged Engine ancestry `6edc1155f`,
Inspector package 0.1.0, compiled format 37. [Scope](../scope.md) is adopted;
INS-05.7–05.10 and M40 already exist. No renewed scope approval is needed.
Historical Slice 15/16 completions and ACTIVE old Slice 20 remain historical.
New Slice 20 must revalidate its carried gaps; old future drafts are inputs.

Use [master allocation](../plan.md), [workflow](../slice-workflow.md),
[needs](../../../needs.md), [requirements](../../../requirements.md),
[acceptance](../../../acceptance.md), [HLD](../../../design/high-level-design.md),
and the single [matrix](../../0.1/verification-plan.md). This file is a proposed
change/ownership map, not a competing acceptance or verification register.

## Scope and prerequisites

- In: review product/source/earlier prework evidence; local create/rename/update/delete
  proposals; source-backed architecture fit; observable checks and unique ownership.
- Out: canonical needs/requirements/AC/HLD changes, source implementation, cleanup,
  installs, environment/library changes, runtime qualification or dependency upgrades.
- A directly commissioned Slice 3 agent is the sole writer after Slices 0–2
  provide their review/proposal outputs. Use the selected report checkout under
  the owner decision above; Slice 0 creates and enters it from the approved final
  planning HEAD and port. This slice reuses it without initial bootstrap.
- Consume accepted ADRs and real memory entries before review: vector identity
  belongs to embedder, exact frozen graph evidence, current store format; memory
  `dont-be-distracted-by-ceremony.md`, `verify-design-against-code-not-just-architecture.md`,
  `tdd-red-must-fail-per-finding.md`, `keep-measurement-tools-in-repo.md`.

## Preparation and evidence inputs

At execution enumerate changes since this draft in scope/master/feature plans,
allocated items, canonical contracts, Inspector and selected dependency source,
and Slices 0–2 proposals. For each record keep/adjust/reject/already-done with
reason and exact evidence; complete the plan without speculative frameworks.

Read the legacy `plans/0.1/slice-{30,40,50}/{plan,design}.md` carry-forward drafts,
`plans/0.1/compatibility-0.8.29-port.md` and reviews, preserving test provenance.
Source pointers below are relative to repository root and were selectively
inspected for this draft; they are questions for future review, not completed audit:

- `src/inspector/src/{worker,storage,session,policy,process}.rs` and assets:
  worker supports catalog/check/page/sample/detail/chunk/tree/associations/export/close;
  lexical/vector/interactive graph operations remain feature work.
- `src/rust/crates/fathomdb-engine/src/search_api.rs` and `dev/interfaces/rust.md`:
  public filtered/view/frozen snippet search and evidence resolvers exist.
  HLD's “no sibling snippet API” sentence is stale; projected-text/BM25F lack
  snippet options. Route-specific snapshot/span/evidence meaning still needs review.
- `src/rust/crates/fathomdb-schema/src/lib.rs` declares 37;
  `fathomdb-engine/src/vector_storage.rs` and format-35 ADR separate vector ID,
  cursor and span metadata. Layout capacity is not proof of multiple emitted
  windows; identify actual supported current writers/read paths before a promise.
- `fathomdb-engine/src/{graph_api,dependency_trace,dependency_closure}.rs`,
  `projection_registry.rs`, `fathomdb-embedder/src/loader.rs`: verify public,
  private and test-only boundaries; no copying private rankers or inferred origins.

## Local DRAFT CRUD candidates

Each executed record includes exact proposed text/location, operation type,
reason/evidence, affected canonical IDs, independent observable AC/test intent,
architecture fit, one delivery owner and prerequisite, and `needs-HITL` reason.
The IDs below label draft records only; no new global AC identifiers are created.
All Slice 3 results remain local. Slice 9 decides proposed changes; Slice 10
applies only individually approved prework remedies. Feature owners refine and
implement approved contracts within scope, with local review and tests.

| Record / type | Candidate, reason and affected IDs | Independent check and design fit | Delivery owner / prerequisite / HITL |
| --- | --- | --- | --- |
| D3-01 update | Reconcile current record-first references, observation/readiness and queued-scope wording; UN-01–10,12–16, INS-01–06/09.1/09.5, X01–07/10/11, V01-N01–03/N06/S01/S03. Port is foundation, not completed full release. | Intended typed identities/revisions, ordinary-write manifests, all eight D-01 combinations; A/S/P/O/R reuse, per-read-transaction queued membership freshness. | 20; ratified 9/10 and carried port evidence; HITL only for proposed scope/policy change. |
| D3-02 update | Qualify current public lexical/snippet/evidence routes against legacy text draft; INS-03.1/04/05.3–05.6/08/09.2, V01-N05/S02. | Authored indexed-text/UTF-8 match facts and named scoring formulas, no Inspector-derived ranks; Q actual returned evidence versus S later index observation. | 30; 20 and route review from 4; scope-changing dependency need goes to 9. |
| D3-03 update | Current stored vector/window/profile/source mapping versus head-only assumptions, Find similar and serving semantic/hybrid distinction; INS-04/06/08.4/09.3, V01-N04/N07. | Independent f64 L2 with tolerance and declared population, known vector/cursor/window links, cached CPU positive and denied-network failures; S distinct from Q fused scores. | 40; 20/30 and actual current writer capability; HITL if required multiwindow support needs excluded Engine work. |
| D3-04 update | Carry interactive graph and High investigation intact; complete edge content, directed scope, retained revisions and class-correct lifecycle; INS-03.3/06.5/07/08/09.4, V01-N02/N03/S04. | Prewritten adjacency, edge-body, temporal-boundary and dependency/revision manifests; S exact stored graph distinct from Q frozen evidence. | 50; 20/30/40 required seams; any missing mandatory public/local feasible route is blocker for 9, not unavailable pass. |
| D3-05 create | Add local architecture detail for already-canonical two pinned observations, not new INS scope; INS-05.7–05.10/M40, T-04 and person workflow. | Preseeded overlap/disjoint typed IDs and method values; differing query/seed/filter/limit/read contexts, graph membership unranked; O bounded pins plus U comparison. | 60; 20–50 observations; HITL only if persistence/export/score-normalization scope proposed. |
| D3-06 rename | Replace ambiguous current-delivery “60 integration” labels with 70 in proposed current allocation; preserve historical labels/evidence and new comparison owner 60. | Every M row resolves to exactly one current accountable owner; earlier local duties remain explicit contributions; no criterion renumbering. | 10; exact wording approved at 9; yes, individual remedy approval at 9, not reapproval of adopted scope. |
| D3-07 delete | Remove unqualified “no snippet API” and any present-tense schema-34-only capability claims from active architecture when replacing them with source-qualified route facts; preserve historical evidence. | Public method/export/gate evidence supports replacement; no inferred provenance/snapshot. | 30; D3-02/source review from 4, approved reconciliation; HITL if removal would weaken a criterion. |
| D3-08 update | Propose final support/docs/human ownership and retain all High evaluation; INS-X08/09/10, INS-06.4, T-01–05/M37–38. | Actual installed exact Linux/Chromium pairing, independent unseen task rubric and High session, scoped resource results; K/U integration. | 70; all features, 6 audience evidence and 7 packaging evidence reconciled at 9; scope changes need HITL. |
| D3-09 update | Reconcile single matrix's legacy ownership with current master, distinguish actual evidence from planned/unrun gates; M01–M40 and all F/T. | No missing/duplicate primary owner, no mandatory positive waived; no fabricated tests/status; existing harness intent checked independently. | 10; Slice 5 adequacy review and 9 approval of individual text remedy; feature tests remain with feature owners. |

No new user need is currently justified: UN-01–18 plus adopted INS-05 cover the
increment. Retain incidental/deferred UN-11/17/18 limits and narrow D-01 export.
Do not rename canonical INS/V01/F/T/M IDs merely to fit new slice numbering.

## Complete current ownership proposal

One accountable feature owner per row below; contributors must complete their
own view's errors, policy, cancellation, accessibility and real-DB/browser tests.
An accountable owner cannot defer an earlier mandatory positive to Slice 70.
Master mapping controls if revised before execution; report every delta to 9.

| Unique accountable owner | Matrix rows (each M01–M40 appears once) | Canonical responsibility / mandatory contributions |
| --- | --- | --- |
| 20 | M01–M07, M09–M13, M15, M17–M18, M20, M24, M29, M31–M33, M36, M39 | Record/catalog/admission/references/readiness/policy/queue/security/exact values/export foundation. Text M06→30; origins, readiness, observations M09–13/15/17→30/40/50; edge M18→50; every new view extends common contracts. |
| 30 | M21–M22, M25 | Lexical query, actual route/tokenizer/match/score and High per-result evidence; M21 vector contribution 40, graph contribution 50, combined investigation 60. |
| 40 | M16, M23, M26, M30 | Stored metric-faithful vector UI and cache-only CPU semantic/hybrid; 30 supplies lexical M16 family values, 20 common startup refusal, every serving score retains own meaning. |
| 50 | M08, M19, M27, M38 | Exact edges/directed graph, High source/dependency/revision and functional investigation; 30/40 provide outcome evidence, 70 runs separate High human session. |
| 60 | M28, M40 | Person cross-view workflow and two actual pinned observations; 20–50 provide correct individual views, references and facts. |
| 70 | M14, M34–M35, M37 | All-view keyboard integration, installed lifecycle, core T-01–05 and High human activity; each feature already owns keyboard basics. M02's human contribution is performed here while accountability stays with 20. |

INS mapping: 01/02/03.1–03.2/03.4–03.5/04/05.1–05.4/05.6/06.1–06.3/09.1/09.5→20;
03.3/06.5/07→50; 05.5/08.4 vector→40; 08.1–08.4 lexical→30 with required
vector/graph contributions; 05.7–05.10/09.6–09.7→60; 06.4→70;
09.2→30, 09.3→40, 09.4→50. Shared INS-08.4 accountability is M22→30,
M23→40, rather than competing owners for one undifferentiated claim.
V01-N01/N02/N03/N06/S01/S02/S03→20 with graph provenance/lifecycle contributions
50 and route reuse 30/40; N04/N07→40; N05→30; S04→50 with each view and 60 integration.
X01–07/X10/X11→20 common contract; X08/X09→70 integration, with 20/30/40/50/60
mandatory local enforcement. F-01/02/03/04/05→20 fixture authority; F-06→70
aggregate scale, each feature boundary subset; F-07→60 coordinated person
manifest with independent lexical/vector/adjacency extensions 30/40/50.
T-01–05→70 actual human evaluation; T-04 functional comparison→60.

## Execution, review and closure

1. Review evidence and produce only local draft records/design allocations. Separate
   source fact, inference, proposed change and actual blocker. No web research now.
2. Submit actual changed draft design/contract reasoning to a direct fresh
   gpt-6.1-sol-high design reviewer; correct substantive findings in one coherent
   batch. Direct Terra verifies prose/traceability/source boundaries, not runtime.
3. Send unresolved architecture alignment questions to 4 and adequacy questions
   to 5. Slices 6/7 own audience-doc and build/CI/packaging evidence respectively;
   exchange pointers, then reconcile shared findings once at 9, no duplicate audit.
4. Adopt quality roadmap real-DB, independent numeric/lexical/adjacency oracles,
   property identities/round-trips, targeted fault/cancel/resource/browser/human
   methods in feature allocations. Defer global MC/DC/fuzz/allocation campaigns.
5. During actual execution run relevant Markdown/trace/link checks using existing
   tools, including `./scripts/agent-lint-md.sh` when integrated. No artificial
   RED/GREEN, runtime-green, new harness or full runtime gate for this prose slice.
   Feature behavior later requires its own failing RED run+commit, separate GREEN,
   unchanged tests during fix-to-spec, independent Sol-high code review and Terra.
6. Create a concise status record only on execution: actual work/checks, unresolved
   mandatory blockers, handoff to 4/5/9. COMPLETE means allocated drafts and scoped
   checks done; unavailable mandatory evidence stays unresolved. Rerun only affected
   checks; required final gates remain. Never borrow another checkout's Python venv.
   Integrate/clean only verified slice-owned extra worktrees/branches; preserve
   release checkout/unrelated work. No hook bypass, force push, tags or publication.

## Future execution prompt

For Slice 3, “implement” below means complete the assigned local DRAFT review,
CRUD text proposals and ownership deliverables only. This explicit prework
no-action override forbids applying proposed canonical/source/architecture,
environment or library changes. No TDD/runtime ceremony applies to prose.
Actual design changes receive the requested design review; no review commissions
implementation. Individual proposed changes go to Slice 9 and conditional 10.

```text
Work on and complete Slice 3 for Inspector inspector/dev/0.2.0 (and in a worktree).

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
