---
title: Inspector 0.2.0 Slice 9 — consolidation and human disposition
date: 2026-11-19
product: FathomDB Inspector
target_release: 0.2.0
status: COMPLETE
desc: Reconcile prework evidence, obtain item decisions and ratify conditional Slice 10
---

# Slice 9 — consolidation and human disposition

## Original planning mandate and baseline

Turn actual completed 0–8 review outputs into one understood, de-duplicated list,
obtain real interactive item decisions, and prepare/review/refine the resulting
[conditional Slice 10](../slice-10/plan.md) for final human ratification. The
[scope](../scope.md) is already adopted. This draft neither runs that human
session nor approves any item. No implementation bundle is predetermined.

Inspected 2026-11-19: planning branch `llm/0.8.29-inspector-additional`, adoption
`609dd85c41`, unchanged Engine base `6edc1155f`, packages 0.8.28/format 37.
Inspector 0.1.0 is an unpublished record browser; [port evidence](../../0.1/compatibility-0.8.29-port.md)
does not establish full-workspace green. The missing checkout-owned `.venv` and
unrun explicit long/native cases remain evidence limits, not waived gates.
Follow [master allocation](../plan.md), [workflow](../slice-workflow.md) and
[design](design.md). Historic final 60 maps to new 70; comparison M40 maps to new
60. Old Slice 20 ACTIVE and old 15/16 completion are not new execution evidence.

In: consolidate proposals, assess source evidence/ownership/dependencies, conduct
human decisions, update conditional 10 and its draft local contracts, independent
review, bounded FIX cycles, actual ratification and concise status. Out: applying
0–8 proposals; installing/upgrading/cleaning source; feature implementation;
Engine/SDK/MCP/schema/ranker/API/pin changes; silently weakening adopted positives.

## Prerequisites and reconciliation

A directly commissioned slice agent is the sole writer. Reuse the release
worktree created and entered by Slice 0 under the
[owner decision](../slice-0/checkout-decision.md). Its base preserves the approved
final planning HEAD and port/scope/plans, not bare main/dev/0.8.29. Other
environment/infra changes remain proposals until actual item-level decisions
and final ratification here; conditional 10 reuses the checkout for its approved set.

Require actual reports/local drafts and review dispositions from 0–7 plus the
explicit reserved/no-work outcome of 8. Missing required input stays missing;
8 contributes no invented finding. Read every source finding and its limits.
Before scoring, enumerate changes since this draft in plans, source/interfaces,
assigned functions, proposed allocations and predecessor evidence. Give each a
keep/adjust/reject/already-done disposition with reason/source; revise this plan
and 10 where needed. Already-done requires current Git/source evidence and does
not fabricate a check pass. Do not re-ask adoption of unchanged scope.

Read [needs](../../../needs.md), [requirements](../../../requirements.md),
[acceptance](../../../acceptance.md), [HLD](../../../design/high-level-design.md),
[single matrix](../../0.1/verification-plan.md), applicable ADRs/interfaces and
actual indexed memory entries. Reconcile drafted contracts locally; do not edit
the global acceptance register or create a second Inspector acceptance matrix.

## Consolidated item table and scoring

At execution maintain one table in this plan; retain original report IDs/links.
One decision unit has one outcome, owner, acceptance and independently selectable
boundary. Merge duplicate claims by root cause and remedy, preserving all source
IDs and disagreements. Split bundled remedies or ownership boundaries; link
prerequisite/dependent rows. Counts must reconcile to every 0–8 source item,
including rejected duplicates, already-done findings and deliberately empty 8.

| Field | Required value |
| --- | --- |
| Item / aliases / evidence | Stable `S9-NNN`, all source IDs, source locations, observed baseline/date and evidence limits |
| Proposed outcome / ownership | Concrete change, Inspector-owned vs external owner, affected files/contracts and later slice |
| Understanding | `unknown` (claim lacks evidence), `partial` (some mechanism/impact unresolved), `established` (source-backed outcome/dependencies understood) |
| Risk | `low` (local/reversible), `medium` (cross-view/build impact), `high` (policy/lifecycle/release/public boundary), or `unknown`; short reason |
| Effort | `small` (one local change), `medium` (several modules/checks), `large` (cross-cutting qualification), or `unknown`; assumptions and verification cost |
| Recommendation | include / postpone / obtain evidence, with benefit, delay consequence and dependency closure; never a numerical auto-approval |
| Contract / acceptance | Existing need/INS/V01/M IDs, observable result/check, failure or rollback handling |
| Human disposition | Initially `UNDECIDED`; later actual include/postpone/reject/defer-evidence, rationale, conditions and response reference |

## Actual consolidated intake — 2026-11-19

Source `aef5b2eac19dbcf5850713081ccd427896909eea`, release checkout
`inspector/dev/0.2.0`. Slices 0–7 now have actual reports/local drafts and 8 has
its reserved status. Keep the adopted scope and feature ownership, adjust old
source/executor assumptions, reject synthetic runtime completion. Independent [Sol-high and Terra reviews](prework-verification.md) passed
with explicit source/document-gate limits. Actual final response 3 ratifies
S10-DRAFT-2 at 2604013b7; all 14 dispositions below now reflect that final batch.
This table replaces pre-execution intake anchors; it is the single item authority.

| Item / aliases / evidence | Outcome and owner | Understanding; risk; effort | Recommendation / dependency / observable acceptance | Human disposition |
| --- | --- | --- | --- | --- |
| S9-001 / S0-ENV, S7-A; [0](../slice-0/review.md), [7](../slice-7/findings.md) | Checkout-owned Python/pip/pinned developer tools and isolated native wheel install, no editable/shared venv; Inspector setup in conditional 10. | partial; medium native isolation; medium | Include bounded setup plan. Review exact wheel/tool commands first; own-path imports and verifier preflight must work, no sibling rebinding. Failed setup stops affected gates; remove only owned partial setup if authorized. | INCLUDE — USER final batch, 2026-11-19, response 3 |
| S9-002 / S0-ABI, S7-D; [0](../slice-0/review.md), [7](../slice-7/findings.md) | Choose Ubuntu22.04-compatible x86_64 builder and installed-test executor; Inspector setup, 70 qualifies. | partial executor unknown; high support; medium/unknown | Include executor selection/preparation or explicitly allocate to 70; never waive ABI target. Acceptance: exact builder/browser route, required ELF symbols and real installed 22.04 all-view/offline/close. Current Docker/Podman executables exist, no suitable image/executor qualification yet. | INCLUDE — USER final batch, 2026-11-19, response 3 |
| S9-003 / S0-TOOLS; [0](../slice-0/review.md), [1](../slice-1/review.md) | Shared TS Node25 contract/CI versus current supported Node26 reconciliation; separately owned SDK/tooling scope. Keep Inspector browser pair provenance. | established mismatch; medium shared contract; medium | Postpone SDK declaration/CI-runtime change outside Inspector; separate commission if wanted. Inspector harness uses separate root/browser tooling. Current scripts expressly skip TS when src/ts/node_modules is absent; this is no demonstrated mandatory Inspector gate blocker. Affected SDK compatibility remains a separate maintenance risk. | POSTPONE — USER final batch, 2026-11-19, response 3 |
| S9-004 / D2-D6-NAV, D3-01/06; [2](../slice-2/proposal.md), [3](../slice-3/draft-changes.md), [6](../slice-6/findings.md) | Correct current Inspector navigation/allocation labels in needs, requirements, acceptance, design/review indices and product-area DOC-INDEX; Inspector docs, conditional 10. | established; low local prose; small | Include precise current pointers/60 versus70 labels, preserve historical paths/status/anchors. Verify links and unchanged criteria/evidence. No source/API behavior. |INCLUDE — USER, 2026-11-19, response 1 below; final ratification response 3 |
| S9-005 / D2-D6-HLD, D3-02/07, D4-01/02; [4](../slice-4/review.md), [6](../slice-6/findings.md) | Correct maintained HLD current snippet/Caller/window facts; Inspector architecture docs, conditional 10. | established false route; medium architecture; small | Include factual replacements only. Public route bodies/cfg validate text; cached serving stays a required unresolved positive; multiple-window disposition follows response 2 without unproven support claims. Do not replace stale claims with unproven design. |INCLUDE — USER, 2026-11-19, response 1 below; final ratification response 3 |
| S9-006 / S3-MAP, D3-09; [3](../slice-3/draft-changes.md), [5](../slice-5/review.md) | Reconcile existing single matrix current owner/stage cells to master mapping; conditional 10. | established 40 unique rows; low prose; small | Include. Preserve every M01–40 method/criterion/fixture/historical result; comparison60/final70, local contributions explicit. No second matrix. |INCLUDE — USER, 2026-11-19, response 1 below; final ratification response 3 |
| S9-007 / S5-METHOD, S7-G; [5](../slice-5/review.md), [7](../slice-7/findings.md) | Clarify matrix-linked evidence scope: synthetic pause versus native, bootstrap/SIGTERM versus installed window close, samples versus peaks; preserve queued-positive regression. Inspector verification docs, conditional 10. | established limits; low prose; small | Include text-only method clarification; no new gate/test campaign. Required actual feature/native/installed positives remain owned20–70, existing executed states unchanged. |INCLUDE — USER, 2026-11-19, response 1 below; final ratification response 3 |
| S9-008 / S7-C; [7](../slice-7/findings.md) | Existing CI path filters/fixtures include Inspector-only source/assets/package with proportional job mapping; shared CI harness. | established source gap; medium routing; medium | Propose narrow Inspector-only filter/fixture integration in final exact-set ratification; no general CI redesign. Failing routing fixture first, actionlint/focused fixture then required final gate for executable change; no new required status/merge queue. | INCLUDE — USER final batch, 2026-11-19, response 3 |
| S9-009 / S4-CACHE, D3-03, D4-02, M30; [4](../slice-4/review.md), [5](../slice-5/review.md) | Resolve supported cache-only CPU Default/None Engine admission/serving design; Inspector 40 evidence or separately scoped Engine prerequisite. | partial route feasibility; high mandatory positive; unknown | Keep bounded evidence in existing Inspector 40 scope; no Engine commission; no Caller/test/private injection. Acceptance: actual Engine positive under denied network/unchanged cache, missing/partial/corrupt negatives with no fetch/repair. Keep 40/70 blocked until qualified. | RETAIN mandatory bounded proof in existing 40 — USER final batch, response 3; outside 10 |
| S9-010 / S4-WINDOW, D3-03, M17/26/F07; [4](../slice-4/review.md), [5](../slice-5/review.md) | Qualify supported single head windows in 0.2.0; defer within-source multiple-window positive through explicit scope/AC/method/roadmap amendment in 10. No Engine change. | established source conflict; high scope consequence; medium document reconciliation | USER chose scope reduction, preserving exact metadata/origins/metric/CPU semantic+hybrid/FTS/graph/all other criteria. Exact local replacement draft is in 10; canonical application waits10 commission. |INCLUDE — USER, 2026-11-19, response 2 below; final ratification response 3 |
| S9-011 / S1-POSTURE; [1](../slice-1/review.md) | Current full-tree advisory/alert/feature-closure review; shared dependency owner, preserve paused Dependabot. | partial fresh coverage; medium security uncertainty; medium | Obtain separately scoped current audit if selected; no tool install/resolver/pin update here. Acceptance exact current advisory data and reachable closure across ecosystems. Four checked GHSAs/stale cargo audit are insufficient universal clean. | POSTPONE — USER final batch, 2026-11-19, response 3 |
| S9-012 / S1-MCP; [1](../slice-1/review.md) | Refer rmcp 3.5.1 affected-version/HTTP-feature question to external MCP owner. | partial reachability; medium external; small evidence | Postpone Inspector work; separate owning review if desired. Confirm actual HTTP client feature closure before any 3.9 upgrade/mitigation decision. Inspector does not link MCP. | POSTPONE — USER final batch, 2026-11-19, response 3 |
| S9-013 / S1-SQL; [1](../slice-1/review.md) | Optional rusqlite 0.43 upstream fixes review; external Engine/link boundary. Retain 0.41.1/libsqlite 0.39.1 now. | partial full reachability; medium native; medium | Postpone routine bump; no matching affected API consumer found in examined source; locked SQLite outside checked advisory range. If owning review finds reachability, separate Engine compatibility/security commission. | POSTPONE — USER final batch, 2026-11-19, response 3 |
| S9-014 / S0-CUDA; [0](../slice-0/review.md) | Optional further current usage/peer compiler-fix relevance evidence; Engine policy ownership. Bounded delegated review performed, adoption trend UNASSESSED, ratings/actions unchanged. | partial data access; low Inspector impact/unknown Engine; unknown | Postpone extra Inspector work. No CPU-only exemption/full procedure-complete claim; no mandatory unchanged-review approval invented. Any actual rating/action change requires owner ratification in owning scope. | POSTPONE — USER final batch, 2026-11-19, response 3 |

No action item is manufactured for reserved 8. S0-BOOTSTRAP-01 is already done
under its existing owner ruling. KEEP-REDIRECT/INPUTS/HISTORY and all other
corpus keep-pending-owner groups remain retained; no delete/archive candidate.
S4-GRAPH/OBS, D4-03–08, D3-01–05/08 feature detail and S7-VERSION are ordinary
adopted 20/30/40/50/60/70 work, not new prework scope or reapproval questions.
S7-B/E/F have no current Inspector code remedy justified: retain preflight,
security separation and external registry ownership. S1 pin/bot RETAIN outcomes
preserve standing decisions. These mappings account for every 0–8 proposal
without silently promoting a feature or duplicating a remedy.

Proposed interactive batches: documentation S9-004–007; setup/routing
S9-001/002/003/008; mandatory feasibility S9-009/010; maintenance referrals
S9-011–014. Recommendations are advisory, never selected defaults. Actual
responses and rationale/conditions will be recorded against exact IDs; no
Slice 10 approved set exists yet. After decisions, derive only selected
Inspector-owned10 work, independently review/verify, then obtain final actual
human ratification of that exact revised plan and residual dependencies.

## Actual human responses

Response 1, owner via root, 2026-11-19:

> yes, include S9-004-007 in Slice 10. This doesn’t need my input.

S9-004/005/006/007 are INCLUDE by USER. This selects text-only corrections,
not their implementation or final plan ratification. New steering delegates
routine in-scope documentation/planning choices: do not request fourteen
mechanical yes/no votes. Setup 001 and builder selection 002 may be drafted as
agent recommendations for final plan ratification, not fabricated individual
human votes. External maintenance 011–014 can be recommended postponed within
existing scope, with decider labeled agent recommendation. Only material
scope/dependency/shared-owner choices need further questions.

Response 2, owner via root, 2026-11-19, selected:

> Qualify single head windows; defer the multiple-window positive case

The UI's “Recommended” suffix was a label, not part of the ruling. The question
explicitly disclosed the scope reduction: within-source multiple-window person
case is deferred; exact stored window metadata, origins, metric, cached CPU
semantic/hybrid, FTS, graph and every other criterion remain. S9-010 direction
is INCLUDE by USER. No Engine work is commissioned. The canonical application
is an exact local draft for 10, not performed in 9; after 10 applies the ratified
amendment, multiwindow is no longer a 0.2.0 positive blocker. S9-009's real
cache-only CPU Engine positive is still mandatory and unqualified.

Before response 3, agent-proposed final set: 001/002/008 plus explicit USER 004–007/010; proposed
postpone 003/011–014; 009 remains bounded proof owned 40, no Engine commission.
Actor labels in the table distinguish recommendation from human vote. Routine
choices follow the owner's delegation and will be approved only by final
ratification of the exact resulting plan/set. No further mechanical item votes. Response3 below ratifies the whole exact set
and dispositions; these were not earlier individual human votes.

## Final human ratification — response 3

On 2026-11-19 the USER selected the option to **ratify S10-DRAFT-2 and close
Slice 9**. The UI's “Recommended” suffix was a label, not an extra condition.
The question pinned reviewed commit
`2604013b736ad655ac5dfa0aa32c1b67208ec63b` and this exact packet:

- **INCLUDE in 10:** S9-001, S9-002, S9-004, S9-005, S9-006, S9-007,
  S9-008 and S9-010.
- **POSTPONE:** S9-003 and S9-011–014; no external SDK/MCP/Engine/pin/CUDA work.
- **S9-009 stays mandatory in existing 40:** bounded supported cache-only CPU
  Engine semantic/hybrid proof, still unqualified; no Engine commission.
- **S9-002 in 10 is local executor-route selection only**; actual preparation,
  artifact ABI and installed all-view/offline/close qualification remain 70.
- **S9-010** is the reviewed single-head-window scope/F-07/method/roadmap
  amendment; exact metadata/origins/metric/CPU/FTS/graph/other criteria survive.
- No Engine API/schema/pin change, paid infrastructure or Slice 10 implementation
  is authorized by this ratification.

This final batch supplies actual human disposition for the former agent-proposed
rows; it does not fabricate prior individual votes. Responses 1/2 retain their
original item-direction provenance. Both independent targeted FIX-1 rechecks
passed at the pinned commit; there were no remaining material findings and
only one FIX cycle. Final ratification closes Slice 9's decision workflow only.
The exact reviewed 10 documents are unchanged by this closure metadata commit.
Canonical scope/AC/method amendments remain unapplied until separately
commissioned10. Source/model/native/installed/human positives remain unqualified.

## Local draft contract and architectural changes

| Affected authority | Slice-local proposal/check | Owner |
| --- | --- | --- |
| All INS/V01/X/F/T and High; UN-01–16 as allocated | Every finding maps to preserved obligation or explicitly outside scope; no silent acceptance deletion | 9 drafts; 10 only ratified edits; features 20–70 |
| V01-N07/S02; INS-09.3; M26/30 | Record callable/cfg/source feasibility and unknown cache-only positive path; test-only admission is not production reuse | 4/9 investigation disposition; 40 positive implementation |
| INS-05.7–05.10/M40; legacy final-owner references | Draft owner correction to 60 comparison / 70 final qualification, maintaining one matrix | 9 selection; 10 if approved |
| No new product need/requirement inherently added | This decision workflow adds no product feature or alternate AC; any proposed CRUD needs explicit item evidence and ruling | 9 |

Source [open.rs](../../../../../src/rust/crates/fathomdb-engine/src/open.rs) exposes
production Default/None and cfg-contained `TestEmbedderChoice::Caller`, consistent
with the [accepted open-policy ADR](../../../../adr/ADR-0.8.28-vector-compatibility-open-policy.md).
[search_api.rs](../../../../../src/rust/crates/fathomdb-engine/src/search_api.rs)
has public frozen/text/snippet/evidence routes; old absence claims need correction.
These facts establish neither complete cache-only serving nor all runtime bounds.
Read public contracts/cfg, private storage facts and test-only routes separately;
never copy ranker code or invent provenance. Future primary-source research is
only for unresolved report questions (supported cache-only loading/admission,
exact packaging/tool contract); use official source/docs and record applicability.
No network research or runtime sweep is performed by this draft.

## Interactive decision session and conditional 10

1. Present a compact evidence/dependency summary, unresolved unknowns and proposed
   decision batches (about 3–5 related items). State risk/effort reasons and scope.
2. Ask the human for include/postpone/reject/defer-evidence per exact item ID;
   permit explicit batch decisions only when every listed ID and condition is
   unambiguous. Clarify ambiguous answers before recording them. Silence, elapsed
   time, a UI default, reviewer recommendation or prior scope adoption is no vote.
3. Record the actual response reference, rationale/conditions and changed selection.
   Unknown work may get an explicitly bounded evidence-only task; it cannot become
   an unbounded approved implementation. Ask follow-ups for prerequisite conflicts.
4. Compute approved dependency closure. Never auto-include an unapproved prerequisite.
   If a mandatory positive depends on rejected/postponed/unknown/external work,
   identify affected slices as blocked; seek a separate scoped ruling/commission.
   Inspector approval cannot authorize another owner's Engine/SDK/MCP change.
5. Derive 10 from only explicitly included Inspector-owned work: each ID has concrete
   files/outcome, dependencies, local contract edits, acceptance/checks, failure
   handling and order. Distinguish feature-owned follow-ups from prework remedies.
6. Directly commission fresh Sol-high (`gpt-6.1-sol`, high) design/draft review of
   selected 10 and closure; code review applies only to later actual code. Resolve
   findings in at most FIX-1…FIX-4, each recording changes/residuals and focused
   recheck. Reconsider after two repeats of the same failure; four is a ceiling,
   not four mandatory rounds. Changed/new items require explicit human decisions.
7. Direct Terra (`gpt-6.1-terra`) verification checks coverage, decisions, source
   limits and selected-10 correspondence. Present final plan, selection delta and
   residuals; obtain actual human ratification of that exact version/ID set.
   After four unresolved cycles stop, expose blockers and request human direction;
   no self-approval, automatic continuation or unreviewed scope substitution.

An explicitly ratified empty set makes 10 a no-change outcome, not authority to
implement recommendations later. Missing mandatory prerequisites still block
20–70 even when 9's decision record is complete. Without final ratification 9
is waiting/blocked and 10 cannot start. Do not fabricate today's session/results.

## Verification, completion and cleanup

Prose-only deliverable: source/link/traceability checks, item accounting, decision
accuracy and `./scripts/agent-lint-md.sh`; no artificial RED/GREEN or runtime-green
claim. No full regression solely for documents. Quality-roadmap adoption is real
DB/independent lexical/numeric/adjacency oracles, properties, targeted faults,
resources/cancel, browser/keyboard/human and installed checks allocated to actual
code owners. Defer global MC/DC/fuzz/allocation campaigns and unrelated ceremony.
New source evidence is format 37 and real queued-scope budget/cache freshness.
Any later approved behavior change in 10 needs its own failing RED commit,
separate GREEN, independent code review and scoped plus required final gates.

Create status only during execution: actual baseline, coverage/decisions, final
10 revision, reviews/FIX residuals, executed checks/skips, downstream blockers.
COMPLETE requires real ratification and assigned document checks; it does not
claim downstream feasibility/implementation complete. Preserve release checkout;
integrate/clean only verified slice-owned extra worktrees/branches, under normal
safe authorization. No main execution/merge, hook bypass, force-push, tag or publish.

## Future execution prompt

Slice 9 implementation means the consolidation, real human session and ratified
conditional 10 documents only. It applies no proposed prework changes; the generic
last sentence below cannot authorize them. Reviews/verification assess documents,
not a fabricated source implementation. Execute this prompt only when commissioned.

```text
Work on and complete Slice 9 for Inspector inspector/dev/0.2.0 (and in a worktree).

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
