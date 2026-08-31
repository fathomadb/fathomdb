---
title: 0.8.24 Slice 3 — product-document and architecture draft plan
status: DRAFT
target_release: 0.8.24
---

# Slice 3 — product-document and architecture draft plan

## Purpose

Review the product-document changes implied by Slices 0–2 and the proposed
feature ladder, then write explicit draft create/read/update/delete (CRUD)
proposals for User Needs, Requirements, Acceptance Criteria, and architecture.
Every draft receives one primary destination. This slice produces reviewable
proposals only; it does not alter an authoritative contract.

## Required inputs

- [0.8.24 plan of record](../../plan-0.8.24.md), including R24-1 through
  R24-7 and the proposed feature allocations.
- Slice 0 findings and [owner decision brief](slice-0-decision-brief.md).
- Slice 1 [design review](slice-1-design-review.md) and
  [library sweep](slice-1-library-sweep.md), including temporary planning
  labels N24-1 through N24-5 and R24/AC24-8 through -12.
- Slice 2 [design review](slice-2-design-review.md) and
  [cruft review](slice-2-cruft-review.md), including temporary planning labels
  N24-6 through N24-8 and R24/AC24-13 through -15.
- `dev/needs.md`, `dev/requirements.md`, `dev/acceptance.md`,
  `dev/test-plan.md`, `dev/architecture.md`, the accepted ADR index, and the
  public interface documents.

## Scope

1. Review every existing 0.8.24 draft need, requirement, and acceptance signal
   for necessity, observability, duplication, and correct allocation.
2. Approve, reject, or adjust each draft. Add a draft only when an in-scope user
   outcome or release invariant has no adequate existing statement.
3. Propose exact CRUD operations against the canonical product documents,
   identifying the target document and section without applying the operation.
4. Review `dev/architecture.md`, applicable signed ADRs, release architecture,
   binding architecture, and packaging architecture for required draft CRUD.
5. Allocate every retained product or architecture draft to Slice 7, one
   feature slice (10, 20, 30, 40, 50, 60, or 70), or postponement.
   When a row belongs to a feature slice, add it directly to that feature's
   initial draft-plan input in `plan-0.8.24.md`; never convert it into Slice 7
   implementation work.

## Non-goals and no-implementation boundary

- Do not edit `dev/needs.md`, `dev/requirements.md`, `dev/acceptance.md`,
  `dev/test-plan.md`, `dev/architecture.md`, an ADR, or an interface document.
- Do not mint authoritative `NEED-*`, `REQ-*`, or `AC-*` identifiers. Existing
  `N24-*`, `R24-*`, and `AC24-*` names remain temporary planning labels.
- Do not change code, tests, workflows, dependencies, runners, registries,
  package metadata, release state, or the feature ladder.
- Do not resolve a Slice 0 owner decision by inference. A conditional draft
  stays conditional and names the decision on which it depends.
- Do not treat an architecture proposal as evidence that its mechanism exists
  in the shipped code.

## Work plan

### 3.1 Review the current draft set

Build a single register containing R24-1 through R24-15, N24-1 through N24-8,
their paired acceptance signals, their sources, and their current allocations.
For each row:

1. identify the applicable existing canonical need and requirement, if any;
2. classify the draft as approve, adjust, reject, merge with another draft, or
   postpone;
3. state whether the draft is user-observable, maintainer-only, or an
   implementation constraint; and
4. state what evidence would make the draft falsifiable.

Implementation constraints that do not express a user or release-process
outcome belong in design or architecture, not in canonical requirements.

### 3.2 Draft product-document CRUD

For every retained draft, write the proposed canonical operation:

| Field | Required content |
| --- | --- |
| Document | `dev/needs.md`, `dev/requirements.md`, or `dev/acceptance.md` |
| Operation | Create, update, supersede, retain, or no change |
| Target | Existing identifier/section, or a temporary new-item label |
| Draft text | One outcome-oriented need, one falsifiable requirement, or one observable criterion |
| Trace | Need → requirement → criterion → proposed verification owner |
| Rationale | Why the current authoritative text is sufficient or insufficient |
| Allocation | Slice 7, 10, 20, 30, 40, 50, 60, 70, or postponed |

Deletion is proposed only for a true contradiction or obsolete duplicate, and
must name the retained replacement. The locked acceptance document is not
changed in this slice; the output may recommend a later owner-governed update.

### 3.3 Draft architecture CRUD

Review the architecture implications of:

- separate Tegra public-package identity and selection;
- remote Windows CUDA build, artifact provenance, and SDK surface;
- CPU artifact preservation and publisher retry safety;
- benchmark-directed engine integration;
- Windows Python-SDK WAL evidence and attribution; and
- current CI ownership and release-branch integration.

Each architecture proposal must name its exact target record, operation,
current rule, proposed rule, reason, downstream contracts, and owning slice.
When a signed ADR owns the decision, propose a successor or amendment rather
than silently contradicting it.

### 3.4 Allocate retained drafts

Use these primary allocation rules:

| Subject | Primary destination |
| --- | ---: |
| Accepted prework maintenance or navigation repair | 7 |
| Main CI interface gap | 10 |
| Benchmark-supported engine change | 20 |
| Tegra public distribution | 30 |
| Windows x64 CUDA distribution | 40 |
| Windows Python-SDK WAL evidence or attributed fix | 50 |
| Installed-package smokes, CPU preservation, publisher idempotency | 60 |
| Release integration/evidence assembly | 70 |
| Insufficiently understood, out-of-scope, or unowned work | Postponed |

One row has one primary destination. Dependencies may be listed separately and
do not create duplicate implementation ownership.

## Deliverables

1. `slice-3-product-doc-drafts.md` — disposition register, proposed canonical
   CRUD, traceability, and allocation for every retained or rejected draft.
2. `slice-3-architecture-drafts.md` — architecture CRUD proposals and explicit
   no-change findings, each with its owning slice and decision prerequisites.
3. An update to the overall plan only if the review changes a proposed slice
   allocation, including the prework-to-feature register for every retained
   Slice 10–70 row; no generated release-state region is created or edited.

## Completion and verification

Slice 3 is complete when:

- every R24-1 through R24-15 and N24-1 through N24-8 input has a recorded
  disposition, including merged/rejected items;
- every retained need traces to a falsifiable requirement and acceptance
  signal, or explicitly records the missing layer;
- every product and architecture proposal has exactly one primary allocation;
- every retained Slice 10–70 allocation has been copied to that feature's
  initial-plan input rather than left only in a prework record;
- every cited path exists and every architecture assertion distinguishes
  current authority, evidence, inference, and net-new proposal;
- canonical contracts, source, tests, workflows, dependencies, and release
  state are unchanged; and
- scoped Markdown lint and `git diff --check` pass for the planning records.

No build, package, hardware, hosted workflow, or full repository verification
run is justified by this planning-only slice.
