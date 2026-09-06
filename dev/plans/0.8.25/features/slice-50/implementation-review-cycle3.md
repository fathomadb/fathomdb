---
title: 0.8.25 Slice 50 implementation review — cycle 3
status: CHANGES_REQUIRED
reviewed_commit: 07218dd6351a93de6cb0b8af26337c01ddc5c4bc
---

# Slice 50 implementation review — cycle 3

Independent review found no P0 issue and returned `CHANGES_REQUIRED` with one
P1 and two P2 findings. The cycle-2 proof matrix was otherwise materially
closed; the focused Rust suite passed 22/22 with properties and strict Clippy.

## Findings

- **P1 — authorization precedence:** `search_with_evidence` rejected relaxed
  existence axes before authenticating the supplied frozen context. A forged
  context could therefore elicit a noncanonical error instead of the required
  indistinguishable evidence refusal.
- **P2 — TypeScript nested context:** evidence request validation bypassed the
  evidence-specific nested field paths and Engine authentication-collapse
  semantics.
- **P2 — closed response types:** Python and TypeScript did not strictly require
  a boolean lifecycle `superseded` value or a non-empty string graph edge
  revision identity.

## Disposition

RED `62a8bec4` makes all three defects executable. GREEN `e741542d` authenticates
before existence-axis refusal, preserves exact recursive TypeScript paths while
leaving semantic authentication to the Engine, and closes both dynamic response
types. That correction also gives test contexts a deterministic `valid_as_of`
only when the test did not explicitly supply one, preventing wall-clock drift
without weakening validity tests.

These corrections require review cycle 4; this cycle is not a completion
verdict.
