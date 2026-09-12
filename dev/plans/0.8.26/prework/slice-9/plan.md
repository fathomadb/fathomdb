---
title: FathomDB 0.8.26 Slice 9 — provisional approved-prework implementation
status: PROVISIONAL DRAFT
---

# Slice 9 plan — provisional approved-prework implementation

## Slice-complete workflow

This plan adopts the full [lean slice execution contract](../../slice-execution-contract.md):
delta reconciliation, need/requirement/acceptance completion, design review,
RED/GREEN implementation where behavioral, code review, independent
verification, status, and any temporary worktree cleanup.

> Slice 8 must replace this plan after interactive HITL decisions and
> independent review. It authorizes no work in its current form.

## Purpose

Implement only feature-independent preparation items approved and allocated to
Slice 9 by Slice 8 so feature work begins from a clean, verified,
contract-consistent repository.

## Candidate work classes

- environment or project-infrastructure corrections;
- individually approved dependency or pin changes;
- approved cruft disposition;
- accepted needs, requirements, acceptance criteria, ADR, interface, or
  architecture documentation changes;
- approved verification-infrastructure repairs that are not feature behavior;
- recurring local build, preflight, or verification corrections from Slice 6;
  and
- feature-independent CI/CD validation, packaging scaffold, diagnostic, or
  supply-chain corrections from Slice 7.

Feature implementation for Slices 10–50 and any Slice 6–7 item allocated after
Slice 10 are prohibited here.

## Execution discipline

1. Copy the exact Slice 8 rulings and Slice 9 placements into an approved-item
   checklist.
2. For each behavioral change, commit a failing test before implementation.
3. Preserve one writer per checkout and isolate any concurrent writer in a
   separate worktree.
4. Update public contracts and successor ADRs in the same change as any public
   surface they govern.
5. Test build/CI corrections with the narrowest non-publishing reproducer, then
   the affected typed verb or workflow validator; do not weaken a gate.
6. Run focused checks, then `./scripts/agent-verify.sh`; run the broader gate if
   approved work touches packaging, documentation build, or release logic.
7. Obtain independent implementation review and close findings before Slice 9
   completion.

## Exit criteria

Every Slice 9 item is implemented and evidenced, every later/postponed/rejected
item remains untouched and durably allocated, the repository gate is green or
an exact externally owned platform gate is recorded, and the feature ladder
may begin at Slice 10.
