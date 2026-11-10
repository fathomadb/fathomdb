---
title: Slice 5 — verification adequacy proposal
status: PROPOSED
target_release: 0.8.29
slice: 5
---

# Slice 5 — verification adequacy proposal

## Goal, maturity and boundary

Propose whether verification establishes the product outcomes on .29's critical
paths, and identify the smallest useful additions. A need→requirement→AC→test
link is necessary but does not establish an adequate oracle or exercised path.
This prework produces reviewable proposals, not product qualification or fixes.
No test campaign, environment change, canonical acceptance edit, threshold change,
state activation or publication belongs here. Follow the newer
[planning brief](../../planning-brief.md) where older reconciliation text differs.

[Release scope](../../../plan-0.8.29.md) preserves integrated MCP/queue/Block 1,
B2-2/3, partial B2-6 and B2-8 storage; residual reporting remains distinct.
The [quality roadmap](../../../../notes/fathomdb-quality-testing-roadmap.md)
favors owned-boundary faults, post-state checks, resource controls, properties
and reference comparisons. Adopt those selectively; defer general SQLite fuzzing,
all-allocation sweeps and broad protocol campaigns. Add no blanket 100% coverage,
new numerical gate or exhaustive platform/CUDA matrix.

## Sources inspected and limits

- Read the brief, release/reconciliation plans and quality roadmap; sampled
  [needs](../../../../needs.md), [requirements](../../../../requirements.md),
  [acceptance](../../../../acceptance.md) and [test plan](../../../../test-plan.md).
- Read active [architecture purpose/profile](../../../../design/fathomdb-data-plane-architecture-v2.md)
  and inspected [agent-access design inventory](../../../../design/agent-access/README.md).
- Read [engine hints](../../../../../src/rust/crates/fathomdb-engine/src/hints.rs)
  including unit tests, and [MCP bridge](../../../../../src/rust/crates/fathomdb-mcp/src/bridge.rs).
- Read complete MCP [feedback tests](../../../../../src/rust/crates/fathomdb-mcp/tests/feedback.rs)
  and [hint allocation test](../../../../../src/rust/crates/fathomdb-mcp/tests/hints_allocation.rs).
  These provide source evidence, not current runtime passes or complete B2 proof.
- Inspected [verification runner](../../../../../scripts/agent-verify.sh), test-runner
  registration/skip handling and [.28 qualification inventory](../../../0.8.28/features/slice-95/qualification.md).
  Bind historical receipts to their source/artifact/configuration; do not promote
  an old assembled campaign into an uninterrupted current .29 green result.
- Applied verified memory entries `slice-prompt-verify-test-claims.md`,
  `conformance-rewrite-vacuous-green-trap.md`, `0.8.27-slice70-projection-move-lessons.md`,
  `tdd-red-must-fail-per-finding.md` and `full-gate-run-frequency.md`.

## Release-local draft needs, requirements and acceptance

IDs below are proposal-local; canonical AC IDs remain unchanged.

| Need | Requirement | Falsifiable proposal acceptance |
| --- | --- | --- |
| S5-N1: Users receive trustworthy persisted and retrieved state | S5-R1: Trace each selected release outcome through contract, AC, source and test oracle | S5-A1: Every selected outcome has a complete chain or an explicit missing/conflicting link, owner and disposition |
| S5-N2: Agents can act on truthful results and diagnostics | S5-R2: Assess composed MCP/SDK paths, including budgets, hints, paging and coverage | S5-A2: Each critical path names a wrong behavior its proposed tests detect and the observable distinguishing it |
| S5-N3: Qualification matches supported operating claims | S5-R3: Classify evidence by exact source, platform, artifact, feature and failure model | S5-A3: Historical, source-only, skipped, diagnostic and current executed evidence are distinguishable in every row |
| S5-N4: Assurance remains affordable and useful | S5-R4: Rank gaps by user harm, changed seam and oracle strength | S5-A4: Every proposed addition has a bounded method, cost assumption, receiving slice and decision; deferred work has a reason |

## Prep and draft review design

Create `requirements.md`, `design.md` and `traceability.md` beside this plan when
this slice is commissioned; they remain drafts for Slice 9. Start with Slice 3's
contract proposals and Slice 4's architecture findings, then iterate on both.
Read the relevant accepted ADR and complete test before making a load-bearing
claim about its assertion. Search integration tests and in-source `#[cfg(test)]`
modules across engine, SDK, bindings and adapter before declaring a gap.

The design is a compact evidence table: outcome/need, requirement, AC or local
criterion, owning source seam, test/fixture and actual assertion, selection flags,
evidence maturity, plausible undetected defect, risk, cost and receiving slice.
Record absent links explicitly. Separate mapping completeness from adequacy:
ask whether the intended user outcome can still fail while every mapped test passes.

| Critical path / allocation | Adequacy challenge and proposed evidence approach |
| --- | --- |
| Governed write→projection→search; 20/30/40/50 | Snapshot/liveness/filter-before-cap and record identity survive cache/fusion changes; compare real-DB results with independently specified expected IDs/provenance, including write-interleaving and rollback |
| Search→SDK→MCP; 60/70/80/90 | Both formats preserve semantics, budget/paging and truthful hints/spans; inventory native/facade/export distinctions and known/unknown embedding coverage without backfill |
| Queue→deadline/cancel→drain/close; 20/60 | Bridge source retains started work after response abandonment; propose permit/idle/resource post-state witnesses, not error-only assertions |
| Erasure→interruption→reopen; 100 | Survivor/history/projection/WAL obligations under accepted format37 semantics; compare existing tests before adding bounded process-crash cases |
| CUDA fault→drop→exit; 110/120/130/140 | Detection alone cannot prove safe destruction; specify fault permission, outstanding-allocation lifetime and subprocess exit before execution |
| Installed package→open/use/close; 160 | Bind supported existing package obligations to actual consumers and artifacts; source imports and library-only CUDA proof do not establish shipped product support |

Source examples constrain the draft: hints unit tests exercise empty/full/degraded
facts; feedback tests assert query association and noninterference. Neither
establishes all residual B2 behavior. Allocation tests contain BLOCK and notify-only
checks; preserve that distinction. AC-083 remains tracked at 100k with its accepted
targets, not a newly blocking performance gate.

Challenge shared predicates in optimized/reference routes and duplicated parity
literals; specify an independent expected-state oracle or disclose the limitation.
Primary-source research starts with accepted ADRs, implementation, full tests and
runner selectors; consult official SQLite/Rust/Tokio/NVIDIA documentation only for
specific unresolved semantics, with dated citations. No research or host result
is preclaimed. Review fit against real product risks and maintenance cost, then
revise scope before recommending any expensive mutation/fault/coverage campaign.

## Dependencies, tasks and completion

Hard inputs: current source/accepted contracts and the proposal brief. Slice 9
requires this output; draft 3/4/5 exchanges are iterative, not circular approval gates.
Prefer contract mapping before test assessment. Slice 0 supplies proposed venue/tool
availability; 7 supplies runner-selection findings. Their absence is an explicit
unknown, not proof that a test is missing. Serialize shared contract/state edits.

1. Inventory selected outcomes and full test/source mappings; preserve built signals.
2. Challenge each critical-path oracle with a concrete counterexample; classify gaps
   as absent test, weak oracle, selector/skip issue, stale evidence or undecided contract.
3. Rank proposals and assign to existing slices; record conditional hardware/fork/
   artifact prerequisites only for affected checks, not unrelated MCP work.
4. Deliver draft requirements/design/traceability and a short `findings.md` for 9.
   Include rejected/deferred methods, unresolved questions and bounded effort estimates.
5. Obtain independent read-only review of mapping and adequacy before checkpoint.
   Close this prework only when S5-A1–A4 can be checked directly from the drafts.

## Review, validation and decision points

Validate links and scoped Markdown after import; do not run full product gates for
this proposal. Later approved behavior changes require per-behavior failing RED,
GREEN, independent review and relevant checks; final code phases retain required
full gates on owned tooling, with affected reruns and honest skip/failure accounting.
New prerequisites for the next reviewer: settle disputed contract/oracle semantics,
confirm current selectors and artifact attribution, and locate exact venue/fault
permissions before costing execution. Route threshold/support/default changes or
unbounded work to Slice 9; source-only uncertainty remains open rather than passed.
