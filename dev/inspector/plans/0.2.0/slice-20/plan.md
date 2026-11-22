---
title: Inspector 0.2.0 Slice 20 — coordinated record foundation
date: 2026-11-19
product: FathomDB Inspector
target_release: 0.2.0
status: COMPLETE
desc: Usable record and representation navigation with exact identity, observation and capability facts
---

# Slice 20 — coordinated record foundation

Execution commissioned2026-11-19 after Slice10 closure. Sole writer reuses
`inspector/dev/0.2.0` at `/home/coreyt/projects/fathomdb-worktrees/inspector-0.2.0`,
clean baseline `daa2842c7f9306436245c68b56f03fe0e1ae5272`.
The [scope](../scope.md), [master](../plan.md), [workflow](../slice-workflow.md)
and [design](design.md) govern; unchanged Engine `6edc1155f`, format37.

## Reconciliation and adopted implementation

| Since the draft / allocated input | Disposition and resulting work |
| --- | --- |
| Approved10 (001/002/004–008/010) complete; tools prepared, HLD/matrix corrected, CI includes Inspector | Already done. Reuse owned setup and exact allocation. Ubuntu22.04 installed proof remains70; no prework rerun. |
| 003/011–014 postponed;009 mandatory CPU serving40; single head window ratified010 | Keep settled boundaries. No Engine/SDK/MCP/pins/locks/model work. |
| D3-01/S4-OBS record foundation: handles, class/cursor/revision checks, chunks/tree, policy and cache | Keep. Opaque class-specific handles/per-operation contexts are the extension seam; no speculative universal DTO. |
| Attribute origin metadata lacks action; Back rereads associations/tree and loses focus/expansion | Adjust: origin action, bounded retained workspace/sub-observations, focus restoration. Explicit refresh makes new reads. |
| Declarations currently expose unknown readiness and a dense-runtime string | Adjust: separate storage, scalar readiness, inspection/retrieval eligibility and runtime facts with source/scope. |
| Public status APIs load registry/pending sets and lock beyond Inspector SQL progress budget | Reject their use here. Bounded layout establishes scalar positives; retrieval eligibility/readiness is unknown until qualified30/40 routes. None-session absence is factual, not CPU qualification. |
| Native/allocation/F06 and startup/scale positives not yet qualified on candidate | Keep required execution now. Genuine executor limitations remain blockers, not finished basics deferred70. |

Adopt local20-D1–D4 below as clarifications of existing needs/req/AC, with no
reduced positives. Three focused RED/GREEN increments: factual evidence, exact
origin navigation, retained workspace/keyboard return with late-selection defense.
Reuse existing linked/revision/collision/queued/F06 fixtures. Browser assertions
check saved contexts, expansion, focus and refresh; real backend tests check scopes.

## Result and boundary

Deliver a usable canonical-record workspace: select an exact retained revision,
read its full document/tree and metadata, follow verified source/materialization
links, inspect declaration/materialization/readiness/runtime facts separately,
and return to the original permitted observation by keyboard or pointer. Build
only the common reference/operation contracts needed by later native views.
The record feature must work now; empty tabs and universal DTO scaffolding are
not its result. Query execution/native text evidence belong to 30, stored
vectors and supported semantic/hybrid runtime to 40, interactive graph and
extended provenance/history to 50, pin/compare and complete person transfer to
60, installed/integrated/human qualification to 70.

Historical [old Slice20](../../0.1/slice-20/status.md) retains its dated ACTIVE
state. The [port record](../../0.1/compatibility-0.8.29-port.md) records its scoped
passes and then-pending gates. Current [status](status.md) owns the newly executed
native/allocation/F06/startup/full-source qualifications on the selected unchanged
Engine composition.
No Engine/SDK/MCP/schema/ranker/API/pin changes, model download, retention,
repair, administrative commands, general export or persistent investigation state.

## Prerequisites and source evidence

- Ratified 9 decisions, completed applicable approved 10 items, commissioned
  `inspector/dev/0.2.0` checkout based on final approved planning HEAD including
  port/scope/plans, and checkout-owned tool environment. Missing prerequisites
  block affected work; never borrow another checkout's `.venv`.
- Read [needs](../../../needs.md), [requirements](../../../requirements.md),
  [acceptance](../../../acceptance.md), [HLD](../../../design/high-level-design.md)
  and [single matrix](../../0.1/verification-plan.md). Use the master's current
  ownership: legacy matrix final integration 60 maps to 70; M40 comparison is 60.
- Inspected `src/inspector/src/{storage,session,worker}.rs` and `assets/app.js`:
  canonical/operational and attribute opaque handles, exact cursor checks,
  per-request contexts, associations, bounded chunks/tree and ten-observation
  cache exist. Associations currently expose declaration readiness as unknown;
  this is not an implementation of native text/vector/graph views.
- Inspected real `tests/integration/{records,associations,queued_scope}.rs` and
  `tests/browser/{context,references,association-choice,projection}.mjs`:
  retained revisions/classes, attribute origins, exact revision choices and
  context checks are reusable, not evidence that all new coordination works.
- Public `Engine::read_projections`, `read_projection_status`,
  `read_projection_generation_status`, `read_embedding_readiness` are candidates
  for separately scoped Engine observations. Verify their exact bounds/context
  in `projection_registry.rs`, `projection_generation.rs` and
  [Rust interface](../../../../interfaces/rust.md) before adoption. Aggregate
  readiness is not record readiness or proof of stored vector presence.
- Production `EmbedderChoice` is Default/None; Caller is contained
  `TestEmbedderChoice` (`engine/src/open.rs`). Follow accepted
  [open-policy ADR](../../../../adr/ADR-0.8.28-vector-compatibility-open-policy.md)
  and [embedder identity ADR](../../../../adr/ADR-0.6.0-vector-identity-embedder-owned.md).
  The existing None session supports model-free inspection; it proves no cached
  CPU serving path. Missing mandatory runtime capability is a 9/40 prerequisite,
  not permission to use test-only admission or waive a positive case.

## Adopted needs, requirements and AC changes

These adopted clarifications refine existing canonical requirements/acceptance,
not a competing register. No new user need is required: existing UN-01/04/06/07/08/09/10/14/15/16 cover this
feature. The canonical Inspector documents now include the implemented clarifications;
global `dev/acceptance.md` and all required positives remain unchanged.

| Local item / affected canonical IDs | Observable acceptance and ownership |
| --- | --- |
| 20-D1: clarify record-first anchor; UN-06/07/14, INS-03/04/05.1–.3/.6, INS-09.1/.5/.6; M09–M10/M17/M24/M28 | 20: exact class/cursor/revision stays selected through document → actual scalar materialization → verified origin → Back. Equal text/classes and retained revisions remain separate; zero/many origins stay unresolved/choices. 30/40/50 extend with their real views; 60 completes person workflow. |
| 20-D2: clarify four facts plus runtime; UN-08/15/16, INS-04/06/08, M11–M15 | 20: selected record shows storage/materialization, eligibility, readiness scope and runtime/operation support separately, each with source/context and unknown/refused states. No declaration becomes ready; no aggregate readiness becomes per-record or absence explanation. 30/40 own route-specific positive readiness/queries. |
| 20-D3: extend common observation contract; INS-05.3/.4/.6–.10, INS-X04/X10; M15/M31–M32/M40 | 20: returning retains exact allowed original context or explicit eviction/expiry; refresh is a new observation, later hydration stays separate. Provide typed variants needed by 30–60 without implementing pin/compare in 20. 60 owns positive pin/compare. |
| 20-D4: carry current-format safety/resource evidence; INS-X01–X08/X10/X11, V01-N01/N02/N03/N06, V01-S01/S03; M04/M18/M20/M29/M31–M35/M39 | 20: real queued membership is transaction-fresh and bounded, all eight disclosure policies compose on new paths, inactive/history default positives and legitimate queued display/export work, absent content stays absent. Required original native/allocation/F-06 foundation cases execute on current build; 70 adds installed aggregate qualification. |

## Source feasibility and independent review

The [design](design.md) was checked against actual storage/session/worker/UI and
Engine source. Bounded layout facts were selected instead of unbounded registry/
readiness API calls. Existing admission, handles, sessions and cache are reused.
No external research was needed to settle an implementation choice. Direct
Sol-high design/code reviews and Terra verification completed; the
[status](status.md) records findings, TDD, actual checks and evidence limits.

## Implementation and meaningful tests

1. Preserve existing stable behavior; author real F-02/F-04/F-07 record fixtures
   from intended stored facts, including two equal-name people, retained
   revisions, canonical derived record, scalar materialization, unknown link,
   inactive/history, telemetry and existing ExplicitMaintenance queued content.
   Ordinary Engine writes seed real databases; do not invent storage states.
2. For each new behavior below, write/run its focused failing test, verify the
   intended assertion fails, commit RED with failure evidence, then implement
   separate minimal GREEN. Existing passing regressions need no artificial RED.
   Never change the oracle to hide a wrong implementation or generate goldens.
3. Add only needed typed reference/result/capability adapters; keep exact decimal
   identities, opaque handles and zero/one/many resolution. Test class/session/
   namespace collisions, equal content, missing/removed revision, changed
   generation and separately observed attribute/source hydration. Property-test
   reference variant round-trips and bounded identity/chunk losslessness.
4. Implement bounded factual status/capability responses and their record UI.
   Compare actual public statuses with their declared aggregate scope and selected
   session; validate record facts through qualified current-layout reads. Test
   unconfigured/actual materialized/missing/stale/refused and runtime-unavailable
   facts separately. Required ready/materialized positives cannot be replaced by
   an all-unavailable screen; unavailable dense session does not satisfy 40.
5. Implement anchor/representation/source controls and context-preserving Back,
   explicit refresh, focus return and empty/partial/error/cancelled states.
   Browser tests perform real keyboard/pointer navigation, exact revision choice,
   long Unicode/value copy, eviction, late-response rejection and return after
   cancel. Retain source request/context without equating successive snapshots.
6. Apply D-01 gates before all new labels/content/links/copy/export/cache delivery.
   Recheck authoritative policy/membership per read transaction. Reuse real queue
   withdrawal/budget and atomic-absence regressions, all eight flag combinations
   and overlap tests; no session/generation-only queued membership cache. Preserve
   authentication/Origin/Host/build pairing, cleared worker environment, inert
   markup/URLs and no payload/credential leakage. Add focused malicious-content
   and cancellation/fault regressions for the new controls.
7. Account shared response/work/history budgets across status/association stages;
   never reset per category or preload full registry/database. Retain 200/2 MiB,
   256 KiB chunks, 100k examined, ten/20 MiB observations, one/eight work queue,
   30 s deadlines/quiescence, 250 ms cancel ACK and canonical memory caps. Adapt
   committed probes only when needed; measurement tools remain rerunnable/indexed
   in `src/inspector/tools/README.md`, raw logs in owned scratch.

## Verification, completion and handoff

Use focused lint/type/tests first, then direct fresh Sol-high code review of the
actual diff/TDD and direct Terra (`gpt-6.1-terra`) runtime/keyboard verification.
Run Inspector all-target clippy, ordinary locked offline tests and paired
binary/examples build; run affected existing browser suites plus new flows.
Explicitly execute the four carried ignored native/allocation/scale cases
identified in `tests/{worker,launch}.rs` and tool README on a qualified graphical
executor; ordinary pass/ignored is not their pass. Run startup/scale probes for
this affected foundation, with their real database/cleanup oracles. Missing
mandatory positive environment/test evidence remains blocked, not unavailable.

Apply [quality roadmap](../../../../notes/fathomdb-quality-testing-roadmap.md):
real DB composition, property boundaries, targeted resource/fault/cancel tests
and independent identity facts; defer global MC/DC, generic fuzzers, exhaustive
allocation campaigns and new benchmark systems. Model/FTS/graph numeric oracles
belong to their feature owners. Slice 0 owns formal environment/CPU-only CUDA
policy review; this slice changes no GPU driver/toolkit/support contract.

Complete the required final source gate `./scripts/agent-verify.sh` on the final
code phase using this checkout's environment; run scoped Markdown/docs checks
for changed docs. A specific fix reruns its reproducer/blast radius, not every
unchanged suite; required final gates still apply. Record exact pass/fail/skips.
Create `status.md` only when execution starts; COMPLETE requires assigned usable
feature, reviews, Terra and checks. Give 30/40/50/60 exact reference/context/status
contracts and unresolved source prerequisites; 70 owns additional installed and
human evidence, not unfinished record basics. Reuse release checkout sequentially,
sole writer; reviewers read-only. Integrate verified slice-owned extra worktrees
only if needed, safely clean those only, preserve release/unrelated work. No main
execution, hook bypass, force push, tag or publication.

## Commissioned execution prompt

The commissioning prompt is preserved below as the task contract. Actual
implementation and qualification outcomes are recorded in [status](status.md).

```text
Work on and complete Slice 20 for Inspector inspector/dev/0.2.0 (and in a worktree).

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
