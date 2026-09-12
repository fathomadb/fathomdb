---
title: FathomDB / Memex 0.6.0 collaboration plan
status: DRAFT
date: 2026-09-12
branch: collaboration/memex-0.6.0
baseline: a563362d7f653202c77b3c9a842db50f4eae2e6d
---

# FathomDB / Memex 0.6.0 collaboration plan

## Outcome

Turn the Memex review of FathomDB 0.8.25 into three independently decidable
slices:

1. Slice 10 makes frozen and evidence-bearing search understandable from the
   public Python documentation.
2. Slice 20 designs an exact, eligibility-bound evidence lookup for a graph
   target without body re-search or private database access.
3. Slice 30 determines the smallest safe extension that lets one governed
   actuation transaction include derived semantic edges.

This package is planning and design work. It does not authorize product-code,
schema, publication, or Memex-repository changes.

The cross-slice and post-0.8.25 portfolio is ranked in
[`memex-needs-prioritization-scaffold.md`](memex-needs-prioritization-scaffold.md).
That scaffold distinguishes FathomDB work from Memex adoption and orders
candidate work by value, stability risk, and effort.

## Baseline findings

- FathomDB is not node-only. Its accepted graph model is one ontology-neutral
  binary property graph with first-class nodes and edges, including temporal
  fact-on-edge memory.
- The 0.8.25 actuation gap is narrow: `ActuationOperationV1` has canonical-node,
  derived-node, dependency-registration, and lifecycle-transition operations,
  but no edge operation.
- `GraphTargetV1` identifies a returned node by logical ID and write cursor but
  does not expose immutable artifact-revision identity or evidence authority.
- Evidence references are minted only by `search_with_evidence`; exact evidence
  resolution cannot currently begin from a graph target.
- The public Python reference on the collaboration baseline lists the frozen
  and evidence methods, but it does not yet teach their comparative semantics,
  end-to-end usage, or complete retry and security rules.

## Slice ladder

| Slice | Deliverable | Dependency | Exit decision |
| --- | --- | --- | --- |
| 10 | Public frozen/evidence documentation and examples | None | A Python consumer can choose and safely compose the APIs without reading internal interfaces. |
| 20 | Exact graph-target evidence design | 10 terminology | The public identity, authorization, failure, and compatibility contracts are implementation-ready. |
| 30 | Edge-actuation decision plan | Existing graph and actuation contracts | Benefits, alternatives, compatibility hazards, and implementation risks support an explicit API/versioning decision. |

Slice numbers express ordering within this collaboration package; they do not
amend any published FathomDB release ladder.

## Cross-slice invariants

- Frozen authority remains reproduce-or-fail, database-local, and
  eligibility-bound; it is not a retained snapshot lease.
- Evidence identifiers and references are not authorization capabilities.
- Authorization, lifecycle, validity, erasure, supersession, and dependency
  closure are rechecked at resolution.
- A graph target's source evidence does not prove the complete traversal path.
- Memex must not use body re-search, raw SQLite, a shadow identity index, or a
  second non-atomic journal to fill these gaps.
- FathomDB supplies storage, identity, transaction, and evidence mechanisms;
  Memex retains semantic policy for relations such as `supports`, `refutes`,
  and `corrects`.

## Completion evidence

Each slice must name the governing ADR/interface anchors, record unresolved
decisions explicitly, and pass the repository Markdown and documentation
checks. Any later implementation must follow test-first delivery and preserve
cross-SDK parity.
