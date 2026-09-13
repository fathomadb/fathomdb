---
title: FathomDB 0.8.26 Slice 46 — technical design documentation convergence
status: DRAFT
---

# Slice 46 plan — technical design documentation convergence

## Purpose and placement

Make the existing technical design documentation current, well organized, and
correct after Slice 45 establishes the architecture hierarchy and before Slice
50 verifies the integrated release. This slice owns design-document lifecycle
and navigation, not product redesign or historical rewriting.

## Slice-complete workflow

1. Enumerate changes since this draft, Slice 45's architecture inventory,
   completed Slice 9–40 work, assigned functions, allocated draft items,
   accepted ADRs/interfaces, and the as-built implementation and tests.
2. Inventory `dev/design/` and any maintained technical-design documents
   elsewhere. Identify owner, audience, authority, release relevance, inbound
   links, and current/historical status; propose keep, update,
   deprecate-in-place, archive-in-place, or delete per document.
3. Evaluate the inventory and update/approve/reject/narrow this plan. Complete
   slice-local needs, requirements, and acceptance criteria without turning
   the slice into a historical-document rewrite.
4. Update the technical-documentation design, index, lifecycle labels,
   successor pointers, and maintained topic designs. Obtain review from an
   independent read-only design-review subagent and resolve findings before
   implementation.
5. Use TDD RED/GREEN for any index, link, or documentation validator changes
   and obtain independent code review. Use explicit source-of-truth comparison
   for mechanical prose/metadata corrections.
6. Use an independent agent to verify organization, correctness, links,
   authority, Markdown, and the affected documentation build. Run
   `./scripts/agent-verify.sh` before completion; add broader matrices only when
   tooling changes justify them.
7. Write `status.md` with the disposition matrix, changed current documents,
   preserved historical records, reviews, checks, and unresolved findings.
   Merge and clean up any temporary branch/worktree used by the slice.

## Requirements and acceptance

- `dev/design/README.md` and the authoritative document index accurately
  distinguish maintained topic designs, reference material, experiments, and
  historical slice records.
- Every maintained topic design agrees with Slice 45 architecture, accepted
  ADRs, interfaces, requirements, and verified 0.8.26 behavior.
- The 0.8.26 frozen evidence, graph evidence, operator integrity, actuation,
  persistence, lifecycle/erasure, bindings, performance, and release designs
  have clear current owners and successor relationships.
- Historical documents are preserved with accurate lifecycle labels and
  successor pointers; unique rationale is not lost and stale text is not
  presented as current guidance.
- Redundant current documents are consolidated through a single owner and
  links. Destructive deletion requires exact no-authority/no-inbound-link/no-
  unique-rationale proof.
- All affected links, anchors, indexes, Markdown checks, and documentation
  builds pass without product or publication changes.

## Stop gates

Stop and return to the owning slice or HITL on a product/design contradiction,
unclear canonical owner, required public-contract or ADR change, destructive
loss of historical evidence, broad path migration, or implementation work.
