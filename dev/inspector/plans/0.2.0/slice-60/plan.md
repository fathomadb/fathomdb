---
title: Inspector 0.2.0 Slice 60 — two observations and coordinated investigation
date: 2026-11-19
product: FathomDB Inspector
target_release: 0.2.0
status: COMPLETE
desc: Bounded in-session comparison and exact cross-view person navigation
---

# Slice 60 — two observations and coordinated investigation

Future execution reuses the release checkout created and entered by Slice 0 under
the [owner checkout decision](../slice-0/checkout-decision.md). Recording that
ruling performs no slice execution or checkout creation now.

## Authority, baseline and outcome

This is the completed Slice 60 plan. The owner-adopted
[scope](../scope.md), current [master allocation](../plan.md) and
[shared workflow](../slice-workflow.md) govern execution. Historical `0.1/slice-60`
combined integration and final qualification; its final installed/support/docs
qualification now belongs to Slice 70. Earlier views must arrive usable and tested.

Inspected 2026-11-19 at scope-adoption `609dd85c41fb7980e05faa1132a248d376d8ec89`,
branch `llm/0.8.29-inspector-additional`, based on unchanged Engine `6edc1155f`
(`dev/0.8.29`, package 0.8.28, current format 37). Imported record-browser port:
`551625462` with remediation `826ac0afc`; these are historical evidence, not new passes.
Future `inspector/dev/0.2.0` must retain the final approved planning-branch HEAD,
including the port/scope/plans, rather than branch from raw Engine/main.

Deliver two pinned actual observations, with honest typed membership/rank/own
metric comparison, and a usable person investigation: full record/metadata →
indexed-text match → vector windows/neighbors/serving → stored directed graph
and complete edge → exact origin/revision → original observation. Equal names,
retained revisions, unknown links and multiple origins must remain distinguishable.

In: pin/replace/unpin A and B; bounded union/intersection presentation; each side's
query or seed, filters/options/limits/population/read contexts; available actual
rank/value; exact navigation/Back; comparison errors, cancellation, disclosure,
cache/resource behavior and keyboard/pointer interaction.
Out: shared score conversion, relevance judgments, fabricated omission causes or
shared snapshots, saved experiments/history/bookmarks, automatic reruns, generic
export, new Engine/SDK/MCP/schema/ranker APIs, richer facets or graph analytics.
The existing conditional narrow erased-content export is preserved, not broadened.

## Prerequisites and inspected evidence

- Ratified Slice 9 decisions and the individually authorized Slice 10 results;
  then required Slices 20/30/40/50 complete, including their own positive views,
  errors/cancel/accessibility and real-database/browser checks. Legacy ACTIVE 20
  is not evidence that new 20 is complete. Consume the current master mapping.
- Slice 40 must qualify a supported **public Engine cache-only CPU admission and
  semantic/hybrid query route**, plus model-free stored-neighborhood inspection.
  Production `EmbedderChoice` is Default/None; Caller is `TestEmbedderChoice`
  behind test/evaluation containment. A standalone cache loader does not prove
  production admission. Missing mandatory positive remains a Slice 9/40 blocker;
  do not bypass with test Caller, copy ranking, change Engine or pass unavailable.
- Consume 40's qualified supported head-window/profile/metric/origin contract and
  actual cache-only CPU Engine serving evidence. S9-010 defers the within-source
  multiple-window positive; exact metadata, revisions, no fabricated windows and
  every other person/compare criterion remain. Missing cached Engine serving
  stays a blocker.
- Read actual [worker operations](../../../../../src/inspector/src/worker.rs),
  [reference/read implementation](../../../../../src/inspector/src/storage.rs),
  [session](../../../../../src/inspector/src/session.rs),
  [browser state](../../../../../src/inspector/assets/app.js) and
  [request ownership](../../../../../src/inspector/src/server.rs). At draft time the
  worker had record operations only; the reconciliation below supersedes that baseline.
  Browser `observations`/`evict`/`invalidate`/`api` already cap retention and reject
  obsolete responses; `storage::context` supplies per-read context, not a shared
  query snapshot. All new comparison state below is net-new Inspector work.
- Read [HLD](../../../design/high-level-design.md), the Inspector
  [requirements](../../../requirements.md), [acceptance](../../../acceptance.md),
  [needs](../../../needs.md), legacy single verification matrix and
  [Rust interface](../../../../interfaces/rust.md); verify callable/cfg contracts
  against current source, not stale HLD helper→Caller or absent-snippet claims.
  Accepted [provider admission](../../../../adr/ADR-0.8.28-vector-compatibility-open-policy.md),
  [format 35](../../../../adr/ADR-0.8.28-store-format-35.md),
  [exact graph evidence](../../../../adr/ADR-0.8.26-exact-graph-artifact-evidence.md)
  and [explicit erasure](../../../../adr/ADR-0.8.28-explicit-erasure-maintenance.md)
  constrain reuse; frozen graph evidence is not an ordinary stored-graph snapshot.

## Execution preparation and local specification delta

Before code, enumerate plan/source/feature/allocated-item changes since this draft,
including predecessor statuses, current APIs and policy/cache controls. Give each
keep/adjust/reject/already-done a short source/test reason; revise this plan/design
and resolve actual prerequisite gaps. Do not rerun settled scope approval.

These are LOCAL DRAFT refinements, not a second acceptance register or permission
to edit global `acceptance.md`. INS-05.7–05.10 and M40 are already adopted; no new
need/requirement ID is necessary. Promote only approved clarifications to existing
Inspector documents/matrix through its current owner.

| Existing IDs / need | Slice-local check/refinement | Owner |
| --- | --- | --- |
| UN-08/14/16; INS-05.7/M40 | Pin two actual bounded results without reexecution; display their own typed IDs, revisions, route, request, population, limits and read evidence. Graph rows have membership only. | 60 |
| INS-05.8; INS-03.4/5; INS-09.6 | Independently seeded overlap/disjoint, equal-text/class/namespace collisions, separate revisions and zero/one/many origins; exact Back/context restoration, no text-based joins. | 60 integration; 20/30/40/50 native routes |
| INS-05.9; INS-06/08; INS-X02/04/06 | A-only means not returned in B's bounded observation, not absent from database; missing values/ranks stay unknown. Gate membership/labels as well as content. | 60 |
| INS-05.10; INS-X03/08/10 | Pin/replace/unpin/evict/refuse/expire and late-response tests share ten observations/20 MiB; no extra pinned budget or retained payload after invalidation. Keyboard facts/actions equal pointer. | 60 |
| INS-04/07/09; F-07; T-03/04/05 | Person record→real text/vector/graph→exact origins, edge content/lifecycle/readiness and original context; usable errors stay in each feature. | 60 workflow; 70 additional human/installed qualification |

## Design review and implementation sequence

1. Finish [design](design.md), source-backed capability table and fixture/oracle
   setup. Directly spawn a fresh **gpt-6.1-sol-high** design reviewer for actual
   additions/changes. Address material identity/disclosure/resource findings
   before affected code; no unrelated architecture rewrite or research ceremony.
2. For each behavior, commit its genuinely failing RED test with the failing
   assertion/result in the commit body; implement separate GREEN, then refactor.
   Keep test intent unchanged during fixes; no generated goldens or database mocks.
3. Extend the existing observation envelope/registry and pin state using immutable
   observation references; implement explicit replace/unpin, bounded comparison
   construction and method-specific labels. No duplicate payload cache or long-
   lived read transaction. Recheck current disclosure before new emissions/actions.
4. Implement identity-safe compare rows and exact origin choices/navigation;
   integrate the person workflow using completed native view routes and shared
   selection/Back. Preserve each observed result independently from later detail.
5. Complete comparison empty/partial/refused/unsupported/unknown states, cancellation,
   focus/eviction announcements and late-result clearing. Fix native-view defects
   with their owners before completion; do not park them in Slice 70.
6. Fresh direct **gpt-6.1-sol-high** code review of the substantive final diff;
   resolve findings with focused RED/GREEN and scoped reruns. Direct **Terra**
   (`gpt-6.1-terra`) independently verifies actual browser keyboard/pointer flows,
   numeric/identity facts and failure/budget cases; source review is not UI proof.

## Concrete verification and quality adoption

Use real disposable F-02/F-04/F-06/F-07 databases and committed existing harnesses.
Independently author expected typed IDs/revisions, origin links, adjacency and
lexical facts before looking at Inspector output. Use hand-calculable stored
vectors/distances and declared populations for numerical oracles, actual backend
serving evidence for semantic/hybrid values; do not invent an independent fused
rank oracle by copying Engine ranking. Ties preserve observed order, not a new
semantic preference. Compare preserved values to actual captured route output.

- Positive overlapping and disjoint results, same-text distinct IDs, cross-class/
  namespace collisions, different retained revisions, repeated physical vector
  entries and zero/one/many origins; canonical correlations never erase row identity.
- Different queries/seeds/filters/limits/read times and populations; capped top-K,
  graph membership without ranks, unavailable metrics, policy-filtered/unknown
  omissions. A route limit is not a population count; no absent rank zero.
- Small property invariants for typed-key equality, grouping, bounded union rows,
  retain/release accounting and opaque reference round-trips, grounded in those
  human-authored cases. Include exact Unicode/large IDs and numbers without coercion.
- All eight D01 flags, overlapping inactive/erasure/telemetry, real queued retained
  and physically absent cases; policy change invalidates pin/compare/history/DOM
  payloads and queued late responses. Membership and labels cannot leak withheld IDs.
- Ten observations/20 MiB shared accounting at boundaries, replacement/unpin,
  graph collapse, eviction/focus recovery, repeated compare/select/Back/cancel,
  detach/session expiry and no implicit query reexecution. Resource and queue
  controls remain the canonical envelope, including 200 items/2 MiB per response,
  256 KiB chunks, one costly request/eight waiting and 250 ms cancel acknowledgement.
- Browser T-03/T-04 rehearsal and T-05 unknown-cause case: inspect full biography,
  real indexed match, supported vector windows/neighbors/serving, directed edge
  content, origin choices and return. Keyboard must expose the same facts.
  Human participant evidence remains human; automate no claim of human success.

Apply the [quality roadmap](../../../../notes/fathomdb-quality-testing-roadmap.md):
independent real-data oracles, boundary/property invariants and targeted fault,
late-result/resource/cleanup cases. Defer generic Engine fuzzing, global MC/DC,
exhaustive allocation campaigns and extra platform research. Any new reusable
measurement harness belongs in `src/inspector/tools`/tests with a rerunnable command
and README entry; raw logs alone may be scratch. No scratch-only AC tooling.
Run relevant Inspector Rust/browser checks, then the required final source gate
`./scripts/agent-verify.sh` once on the final code phase; use checkout-owned `.venv`,
report missing preflight rather than borrow one. Markdown uses scoped validators.
Scoped passes are not workspace/installed green; Slice 70 owns its additional gates.

## Completion and execution ownership

One directly commissioned slice agent is sole writer in the existing release
worktree/branch, sequentially reused. Additional worktree only for necessary
concurrent writing, never primary checkout; review/verification agents are read-only.
Write concise `status.md` only during execution: final revision, completed scope,
actual checks, remaining blockers and next owner. Mandatory positive failure or
missing supported route means blocked/incomplete, never unavailable-pass.
Integrate verified slice-owned extra work, then clean only its own extra resources;
preserve release worktree and unrelated work. No skipped hooks, force push, tags,
registry publication, or Steward/Orchestrator. Execution follows this feature plan;
the prompt below is future commissioning text and is not executed by this draft.

## Execution reconciliation — 2026-11-20

Baseline `668bf73e995b70197df14cfe22008a46e6f79878`, existing release checkout.
Predecessors20/30/40/50 are complete; their canonical source, status and real
browser tests replace the draft's record-only capability claim. Keep adopted
INS-05.7–10/M40 and M28; no new IDs or acceptance register. Ratified9/10 scope,
Engine6edc1155f/format37 and cached-cpu shipping capability remain unchanged.

- **Already done:** native indexed/body/property/BM25F text, exact vector windows,
  stored L2, actual cached CPU hybrid, directed stored/native graph, source,
  dependency and retained history routes, per-read contexts and exact Back/focus.
- **Adjust:** comparison consumes these actual envelopes. Graph response endpoint
  keys are local and cannot join observations. Canonical class/cursor and known
  revision, vector row/profile/cursor and typed Engine result entries are separate
  identities. Origin links correlate but never deduplicate entries.
- **Keep/new:** explicit A/B pin/replace/unpin, own original request/context/value,
  bounded union/intersection, honest missing-side labels, native return navigation,
  policy revalidation, stale/late/budget clearing and keyboard interaction.
- **Adjust resources:** keep one registry and existing ten/20MiB limits. Retain
  original request in each registry observation, pin only registry IDs. Charge
  comparison DOM/index/control storage to that owner's budget, page at50 rows,
  release transient projection/DOM on replace/unpin/eviction/invalidation/leave.
  No extra service comparison response or duplicate payload store.
- **Reject:** persistent experiment/export, query rerun, shared score conversion,
  shared snapshot, guessed omission cause, Engine/API/pin/lock changes and new
  all-revision index.70 retains installed/all-view/unseen-human qualification.

Implement after fresh direct Sol-high design review: (1) typed pure adapters and
paged join with independent unit/property cases, (2) shared registry pin owner and
browser controls with actual DB RED/GREEN, (3) policy/late/budget transitions and
integrated person fixture/browser proof. Each behavior has an intended assertion
RED commit then minimal GREEN. Fresh Sol-high code review and Terra runtime check
follow the coherent batch. Focused checks during work; one final clean full gate.

## Future execution prompt

```text
Work on and complete Slice 60 for Inspector inspector/dev/0.2.0 (and in a worktree).

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
