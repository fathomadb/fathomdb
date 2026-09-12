---
title: FathomDB 0.8.26 Slice 7 — provisional approved-prework implementation
status: PROVISIONAL DRAFT
---

# Slice 7 plan — provisional approved-prework implementation

> Slice 6 must replace this plan after interactive HITL decisions and
> independent review. It authorizes no work in its current form.

## Purpose

Implement only preparation items approved in Slice 6 so feature slices begin
from a clean, verified, contract-consistent repository.

## Candidate work classes

- environment or project-infrastructure corrections;
- individually approved dependency or pin changes;
- approved cruft disposition;
- accepted needs, requirements, acceptance criteria, ADR, interface, or
  architecture documentation changes; and
- approved verification-infrastructure repairs that are not feature behavior.

Feature implementation for Slices 10–50 is prohibited here.

## Execution discipline

1. Copy the exact Slice 6 rulings into an approved-item checklist.
2. For each behavioral change, commit a failing test before implementation.
3. Preserve one writer per checkout and isolate any concurrent writer in a
   separate worktree.
4. Update public contracts and successor ADRs in the same change as any public
   surface they govern.
5. Run focused checks, then `./scripts/agent-verify.sh`; run the broader gate if
   the approved work touches packaging, documentation build, or release logic.
6. Obtain independent implementation review and close findings before Slice 7
   completion.

## Exit criteria

Every approved item is implemented and evidenced, every postponed/rejected
item remains untouched, the repository gate is green or an exact externally
owned platform gate is recorded, and the feature ladder may begin at Slice 10.
