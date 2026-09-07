---
title: 0.8.25 Slice 55 independent implementation review — cycle 14
status: FAIL
---

# Slice 55 independent implementation review — cycle 14

## Review target

- Candidate HEAD: `2864c90fdf17f4a525d90730fa8b85d32bee04ad`
- Baseline: `2d6836a5bc4e4be40aa1d921568786c4b7f1f41c`
- Branch: `release/0.8.25`
- Worktree: clean; 164 commits ahead of the remote branch

The independent reviewer made no repository or Git changes.

## Verdict

**FAIL.** FIX-13 makes runtime setup fallible and role-complete, but its
one-phase acknowledgement can accept a role that exits immediately after
reporting success. One P1 startup-protocol finding requires FIX-14.

## P1 — setup acknowledgement does not prove a live service loop

Each runtime role sends its only successful startup report before entering its
normal wait or work loop. `await_startup` accepts the three expected reports
and returns success without a parent-issued service probe, a second-phase
acknowledgement, sender-disconnection or extra-message validation, or a handle
liveness check. A role can therefore send the expected identity and exit
immediately while `Engine::open` still reports success.

The existing success fixture exercises only the normal path and cannot expose
this false acceptance. The deterministic RED is an `ExitAfterReport` fault for
one worker: send the ordinary successful setup report, then exit. The current
constructor returns `Ok` from the queued message set even though the worker is
gone and an immediate native inventory cannot complete.

FIX-14 must add a second service-readiness phase under the same absolute 30 s
startup deadline. After setup, keep each role alive behind parent-controlled
coordination, issue and collect an exact role-tagged service acknowledgement,
and only then release roles into their normal loops. Missing, duplicate,
unexpected, disconnected, or exited roles must fail, stop, notify, and join
the entire partial runtime. Merely adding a handle-liveness sample or extending
the timeout is insufficient.

## Items that pass review

- `648fb3f1cfef8fac7e6e454e3853dc8caa156a0c` is a genuine compile RED.
- GREEN `2143c2632c2ba353cab694b6c8b1807dcbb1414c` does not edit the frozen
  startup tests.
- Runtime construction is fallible and maps to the existing
  `EngineOpenError::Io` surface without an ADR or public error expansion.
- Setup reports cover the exact unique dispatcher and two worker roles under
  one 30 s deadline.
- Connection setup, worker vector-partition setup, managed registration, and
  WAL registration precede each setup report.
- Observed spawn, setup, timeout, disconnection, missing, and duplicate-role
  failures stop, notify, and join partial runtime threads boundedly.
- The deterministic test seam is restricted to `cfg(test)`.
- FIX-12 cancellation and WAL-attribution behavior is unchanged and green.
- Canonical unconfined serial and parallel-report workspace runs both pass and
  terminate normally.

Focused startup, pause, WAL, non-test release-check, formatting, Clippy, and
diff checks passed during review.
