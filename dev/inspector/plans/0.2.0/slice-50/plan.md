---
title: Inspector 0.2.0 Slice 50 — stored graph and history
status: COMPLETE
date: 2026-11-19
---

# Slice 50 — directed stored graph, provenance and retained history

Execution reuses the release checkout created and entered by Slice0 under the
[owner checkout decision](../slice-0/checkout-decision.md). The owner commissioned
this slice after40 closure; [status](status.md) records actual verification.

Original draft baseline: scope-adoption `609dd85c41fb7980e05faa1132a248d376d8ec89` on
`llm/0.8.29-inspector-additional`, unchanged Engine `6edc1155f`, format 37.
The [adopted scope](../scope.md), [master allocation](../plan.md) and
[shared workflow](../slice-workflow.md) govern. Old 0.1 Slice 50 is reuse input;
old ACTIVE Slice 20 is not evidence that current feature Slice 20 is complete.

## Execution reconciliation (2026-11-20)

Baseline `a34dbe03e2786bbe9cbb4f02f633e63a0d479422`; existing release
worktree reused as its sole writer. Reviewed actual 20/30/40 source and statuses.

| Delta since draft | Disposition |
| --- | --- |
| 20 supplies exact detail/chunks, fresh policy checks and retained Back workspaces | Reuse; extend reference validation to graph/history owners. |
| 30 supplies native lexical requests, payload/rank evidence and exact origins | Preserve original observations when entering graph/history. |
| 40 supplies actual cached CPU hybrid and explicit shipping feature | Preserve `cached-cpu` split, source cache confinement and build storage. |
| No graph/history adapter or SVG exists | Implement here; neither associations nor serving neighbors completes it. |
| Format37 has indexed incident edges but no all-revision logical-ID index | Bound one canonical scan per request (100k rows, 16MiB keys); report unknown resolution on exhaustion, never select latest. |
| Frozen graph expansion can disclose exact selected target/terminal-edge revisions | Add a separate opt-in native observation with public evidence; no private eligibility or resolver copy. |
| Existing source associations omit registered dependency and revision chains | Add bounded class-qualified investigation, including missing targets. |

Existing canonical needs/criteria suffice; the local refinements below are adopted
for this slice. No new acceptance register or Engine change is required. Design
review precedes product changes. TDD is split into stored graph, investigation,
native evidence, and accessible interaction behaviors, with resource/policy
regressions at their respective seams. Full verification runs once at clean final
source, then focused failures only. Runtime/build ownership is serialized.

## Outcome and bounds

Deliver a usable interactive directed stored graph with complete node/edge
inspection, incoming/outgoing/both traversal, kind filters, select/expand/collapse,
exact references and Back. Parallel edges, self edges and cycles remain distinct.
A synchronized relationship list provides equivalent keyboard access; a table
alone fails. Geometry is presentation, never similarity or relation strength.
Deliver bounded source/dependency tracing, class-correct lifecycle facts and
retained canonical revisions, including High INS-07/08 investigation.

- Include full edge-owned body and metadata, endpoints, typed IDs, retained
  revision identity or explicitly absent revision evidence, exact value/chunk
  access, copy and existing narrow disclosure-governed export integration.
- Keep stored graph relations, source attribution, registered dependencies,
  representation origins, supersession and query-time traversal evidence distinct.
- Preserve original text/vector observation and exact selection when entering or
  leaving graph/history. Equal names/content and bare IDs never merge records.
- Exclude inferred similarity edges, graph analytics/layout tooling, global graph
  loading, CRUD, repair, reconstruction, persistent workspaces and general export.
  Engine/SDK/MCP/schema/API/pin changes and copied private algorithms are excluded.

## Prerequisites and evidence

Execute after ratified Slice 9/conditional 10 outcomes and required 20/30/40
features; consume the current master mapping, not legacy owner numbers. Slice 20
owns common exact references, read contexts, policy, resource/session control and
record navigation; 30/40 supply actual lexical/vector observations for F-07;
60 owns two-observation comparison; 70 owns final installed/human qualification.
Feature-local usability, errors, disclosure and tests close here, not at 70.
The future release checkout comes from approved final planning-branch HEAD,
preserving the Inspector port and scope, rather than raw Engine/main.

Read canonical [needs](../../../needs.md), [requirements](../../../requirements.md),
[acceptance](../../../acceptance.md), [architecture](../../../design/high-level-design.md),
[quality roadmap](../../../../notes/fathomdb-quality-testing-roadmap.md), accepted
[exact graph evidence ADR](../../../../adr/ADR-0.8.26-exact-graph-artifact-evidence.md),
[rust contracts](../../../../interfaces/rust.md) and relevant accepted lifecycle,
dependency-closure, governed-write and queued-erasure decisions before design.
Read applicable AGENTS and actual indexed memory entries:
`dont-be-distracted-by-ceremony.md`, `verify-design-against-code-not-just-architecture.md`,
`tdd-red-must-fail-per-finding.md`, `keep-measurement-tools-in-repo.md`.

Inspected source evidence, relative to repository root:

| Source | Observed fact and consequence |
| --- | --- |
| `src/inspector/src/storage.rs`, `policy.rs` | Existing exact detail/chunks, class fields, associations and queued-scope classification are reuse seams; interactive graph/history are not delivered by listing them. |
| `src/rust/crates/fathomdb-engine/src/graph_api.rs` | Public `graph_neighbors` returns `Vec<NodeRecord>`, depth 1–3, cap 50; no complete edge API follows from it. Public `graph_expand` rejects inactive/superseded contexts; serving traversal cannot replace default stored-history browsing. |
| `src/rust/crates/fathomdb-engine/src/search_api.rs`, `evidence.rs` | Public frozen `resolve_graph_evidence` resolves only an actually disclosed selected artifact; internal resolver is private. This is not arbitrary full adjacency/history enumeration. |
| `src/rust/crates/fathomdb-engine/src/dependency_trace.rs`, `dependency_closure.rs` | Public frozen reciprocal `trace_dependency` and keyed `read_dependency_closure` exist; closure status does not enumerate arbitrary revision history or prove a readable source body. |
| `src/rust/crates/fathomdb-schema/src/lib.rs`, engine temporal/read/graph expansion sources | Current canonical/source/dependency layout and distinct node/edge predicates need exact qualification before named stored reads. Test hooks are not production routes. |

## Prep and local draft contract changes

At execution enumerate changes since this baseline: plan/design/scope and master
allocation, predecessor statuses, actual graph/record/session/UI code, source
contracts/schema, assigned findings and fixture facts. Record keep/adjust/reject/
already-done with short evidence/reason rows here; revise to complete, bounded
scope. Revalidate existing tests in full before claiming their assertions.

The following are LOCAL DRAFT refinements only, not a competing acceptance
register or authority to edit canonical `acceptance.md`. Existing needs are
sufficient: UN-06/07/08/10/12/13/14/15/16 already express these outcomes. Apply
approved canonical refinements through the owning workflow; missing required
positive capability blocks completion rather than becoming an unavailable pass.

| Existing IDs | Local refinement and observable check | Owner |
| --- | --- | --- |
| INS-03/04/05, INS-09.4–09.7, V01-S04; UN-06/07/14 | Directed multi-kind SVG plus keyboard list; exact full edge/node details, in/out expansion/collapse, parallel/self/cycle identity, explicit scope/caps, return to original result/revision. | 50, coordination 60 |
| INS-06/07, V01-N02/N03/S01; UN-12/13/15 | Separate provenance completeness/role, recorded dependency, source cardinality, retained supersession and node versus edge time facts; missing/ambiguous/erased targets remain honest. Positive trace and retained-history navigation required. | 50 |
| INS-08, INS-X04; UN-16 | Inspect available actual serving traversal/frozen evidence separately from stored adjacency; report route/context and supported eligibility facts, never infer exclusion from graph absence. High investigation includes an independently known selected artifact and a stored absent target with an undetermined cause. | 50 with 30/40 |
| V01-N01, INS-X01–X11, F-01/02/04/05/06/07 | D01 overlaps on graph/history/trace/evidence/copy/export, bounded/cancelled/partial outcomes, inert content, no mutation commands, keyboard parity and actual owned-work cleanup. | 50; aggregate 70 |

## Design, implementation and verification sequence

1. Finish [design](design.md): qualify named current-layout stored adjacency,
   exact endpoints/revisions, trace mapping and historical reads against public
   semantics. Identify genuinely missing route as a prerequisite for 9/50; do
   not change Engine, copy private eligibility/ranking code or fabricate links.
   Investigate the listed primary-source questions only when needed in execution.
2. Obtain a fresh directly spawned **gpt-6.1-sol / high** design review covering
   requirements/AC, source feasibility, independent oracles and resources; resolve
   material findings before affected implementation. Review actual design changes,
   not repeated unchanged paperwork.
3. TDD small vertical behaviors: exact directed adjacency/full edge detail;
   bounded expansion/filter/collapse; class-specific lifecycle and history;
   recorded provenance/dependency trace; coordinated accessible UI/evidence.
   Each behavior gets its own genuinely failing RED run and committed RED test
   with failing assertion evidence, followed by separate GREEN/refactor. No
   database mocks, generated goldens or test changes to mask wrong behavior.
4. Use real databases populated with supported ordinary/governed APIs. Human
   intent and seed manifest define independent typed adjacency, edge-body and
   revision facts before implementation. Compare source semantics only where
   equivalent; shared Inspector predicates cannot serve as the only oracle.
5. F-01/02/04/07: directed multi-kind in/out, parallel/self/cyclic edges, distinct
   equal-label IDs/classes, long/Unicode edge bodies, NULL/empty/missing values,
   two retained revisions, incomplete/complete provenance, zero/one/many origins,
   recorded dependency/source links, missing endpoints and exact Back context.
   Derive edge temporal boundary and node half-open validity expectations
   independently at explicit instants; no invented edge node-state/t_valid filter.
6. D01: all eight launch flag combinations across every added path, overlapping
   gates, default positive inactive/history, real ExplicitMaintenance frozen queue
   membership without invented logical removal, queued content denial/allowance,
   physically absent content remaining unavailable and per-read freshness.
   Reuse qualified setup; do not create a retention delay or fabricate storage.
7. F-06: 10,000 incident edges within full fixture scale, ≤500 retained graph
   objects, ≤1,000 examined links, depth ≤3; count examined work, not just output.
   Respect 200 items/2 MiB response and 256 KiB chunks, ten observations/20 MiB,
   queue/memory/deadline budgets. Test cancellation before/while queued/executing,
   acknowledgement ≤250 ms, late-result suppression, finite teardown ≤30 s,
   collapse/eviction releasing hidden state, retry, detach and owned handles.
8. Real browser/DOM assertions and keyboard: labeled arrows/kinds, selected edge
   full content, expansion/collapse, focus/Back, truncation/cancel/error versus
   complete empty, inert hostile HTML/URLs and policy-safe links/copy. Exercise
   person → graph → recorded source/history → exact original text/vector context.
   Prepare graph/High investigation cases and rubric for 60/70 human tasks;
   browser output alone cannot prove those human outcomes.
9. Fresh direct **gpt-6.1-sol / high** code review, then direct **gpt-6.1-terra**
   verification of final revision and focused blast radius using actual fixtures,
   UI and resource outcomes. Fix findings with reproducer/adjacent scoped reruns;
   no repeated full regressions without changed broad risk. Run required scoped
   Inspector checks and final full source gate per AGENTS; never borrow a venv.

Quality-roadmap adoption: real DB independent adjacency/body/time oracles,
property invariants for typed-reference round-trip and graph identity/bounds,
selected fault/cancel/late-result/resource branches, browser/keyboard and human
transfer tasks. New evidence is the working format-37 record foundation and
per-transaction queued-scope correction, requiring freshness regressions.
Defer engine fuzzing, global MC/DC and exhaustive allocation campaigns; no
irrelevant benchmark ceremony. Commit any new reusable measurement harness
with purpose/command, index it with the relevant test/experiment; raw logs stay scratch.

## Execution ownership and closure

One directly commissioned slice agent is sole writer in the existing isolated
`inspector/dev/0.2.0` release checkout, sequentially reused. Reviewers are read-only;
extra writers require isolation. Never execute on main or pollute primary checkout.
Write a concise execution `status.md` only then: final scope/revision, substantive
design changes, RED/GREEN/review/verification results and unresolved positives.
COMPLETE requires usable graph/history/High investigation and required checks.
Integrate and clean only slice-owned extra branches/worktrees after verified
integration; preserve release checkout and unrelated work. No skipped hooks,
force push, tag or publication.

## Future execution prompt — do not execute during drafting

```text
Work on and complete Slice 50 for Inspector inspector/dev/0.2.0 (and in a worktree).

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
