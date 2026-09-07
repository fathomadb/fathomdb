---
title: 0.8.25 Slice 55 status
status: BLOCKED
slice: 55
updated: 2026-09-06
---

# Slice 55 status

## Current state

Slice 55 is blocked after independent implementation review cycle 13 on
`release/0.8.25` at candidate
`4d3f4ce1f7a3a0b16ed8120115775df7210fda80`. Design review passed at cycle 4.
FIX-12's cancellation-safe projection pause passes review, but review cycle 13
found one P1 projection-runtime startup defect. The owner has authorized
correction and review cycles through 15.

The release-state authority therefore remains unchanged: Slice 55 is not
complete, `next_slice` remains 55, and Slice 60 must not start.

## Blocking review finding

`ProjectionRuntime::new` returns before its dispatcher and two workers report
successful SQLite setup and role registration. Those threads also silently
return on connection or partition-setup errors, so `Engine::open` can report
success for a runtime that will never process projection work. Parallel-report
evidence observed all three roles opening 303–309 ms after an immediate native
inventory timeout.

FIX-12 closed the earlier deadlock:

- a compile RED introduced bounded readiness and cancellation witnesses;
- a channel-backed handle now bounds readiness and worker release, releases on
  drop, and cannot strand the WAL-owning worker; and
- every converted caller, the exact five-BUSY witness, four concurrent witness
  processes, and the canonical unconfined serial workspace gate pass.

The canonical release gate is
`bash scripts/test-rust-workspace.sh --serial`. The
`bash scripts/test-rust-workspace.sh --parallel-report` route is diagnostic and
non-gating under TC-72/TC-74; it terminated normally with no pause-hook timeout
or deadlock. Its bounded startup-race evidence nevertheless proves the P1
product defect above.

## Passed evidence

- Final design review: `design-review-cycle4.md`.
- Latest implementation review: `implementation-review-cycle13.md` (FAIL).
- TDD chronology: `implementation-tdd-chronology.md`.
- Independent Linux focused, performance, wheel, N-API, fast, heavy, all,
  selected-feature, Clippy, and check gates pass.
- Windows wheel, N-API, focused Rust, facade, and CLI routes pass.
- The canonical serial workspace gate passes unconfined.
- Parallel reporting terminates without reproducing the futex deadlock.

The detailed evidence and retained failure traces are in
`verification-fix11-review.md`, `verification-review.md`, and the two
`verification-windows-wheel-failure*.log` files.

## Required next action

Use authorized FIX-13 to preserve deterministic RED coverage for successful
role-complete startup and failed runtime setup, then implement the smallest
bounded role-tagged startup handshake. Any partial startup failure must stop
and join all runtime threads before `Engine::open` returns the existing typed
I/O error. Do not push or start Slice 60 until independent review and
verification pass and Slice 55 is durably closed.
