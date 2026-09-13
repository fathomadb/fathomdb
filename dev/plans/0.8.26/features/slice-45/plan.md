---
title: FathomDB 0.8.26 Slice 45 — architecture documentation convergence
status: DRAFT
---

# Slice 45 plan — architecture documentation convergence

## Purpose and placement

Make the maintained architecture documentation current, coherent, navigable,
and correct after Slice 40 fixes the as-built product shape and before Slice 50
asserts release readiness. This is documentation convergence, not permission to
change product behavior or rewrite historical decisions.

## Slice-complete workflow

1. Enumerate changes since this draft, completed Slice 9–40 work, assigned
   requirements/acceptance criteria, allocated documentation items, accepted
   ADRs, public interfaces, and the as-built crate/module/schema topology.
2. Inventory architecture-bearing documents, identify their authority and
   lifecycle, and classify each as keep, update, deprecate-in-place,
   archive-in-place, or delete. Do not act on uncertain ownership.
3. Evaluate the inventory and adjust this plan so it is complete but not
   overbuilt. Add or adjust slice-local needs, requirements, and acceptance
   criteria before editing maintained documents.
4. Update the architecture design and navigation, then obtain review from an
   independent read-only design-review subagent and resolve findings before
   implementation. A contradiction with an accepted ADR requires a successor
   ADR; a code/contract defect returns to its owning feature slice.
5. For any new validation behavior, use TDD RED/GREEN and obtain independent
   code review. Pure prose/link corrections use the mechanical-change
   exception with explicit source-of-truth evidence.
6. Use an independent agent to verify correctness, authority ordering, links,
   Markdown, and the affected documentation build. Run
   `./scripts/agent-verify.sh` before completion; add broader matrices only if
   validation code or product artifacts change.
7. Write `status.md` with the inventory, disposition, review evidence,
   unresolved contradictions, and exact checks. Merge and clean up any
   temporary branch/worktree used by the slice.

## Requirements and acceptance

- One clear current architecture entry point identifies current versus
  historical architecture and links to the authoritative ADR, interface, and
  subsystem-design layers.
- Maintained architecture matches the 0.8.26 as-built crate/module boundaries,
  write/read/concurrency paths, fresh-database boundary, frozen evidence,
  operator boundary, and atomic derived-edge behavior.
- Accepted and superseded ADR relationships are accurate; accepted decisions
  are not silently changed in prose.
- Historical architecture remains historical and gains a verified current
  successor pointer when needed; it is not rewritten as if it described
  0.8.26.
- Duplicate current authorities are consolidated by ownership and links, not
  by deleting unique rationale.
- Every cited path and anchor resolves, documentation gates pass, and no
  product, dependency, schema, packaging, or publication change occurs.

## Stop gates

Stop and route the issue to the owning feature or HITL on an architecture/code
contradiction, uncertain authority, proposed decision change without a
successor ADR, destructive historical move, public-contract change, or scope
large enough to require product implementation.
