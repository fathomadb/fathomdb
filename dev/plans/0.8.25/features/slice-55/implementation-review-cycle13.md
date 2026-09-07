---
title: 0.8.25 Slice 55 independent implementation review — cycle 13
status: FAIL
---

# Slice 55 independent implementation review — cycle 13

## Review target

- Candidate HEAD: `4d3f4ce1f7a3a0b16ed8120115775df7210fda80`
- Baseline: `52a350c732cc84f775b0ffd06a456f01aca65c69`
- Branch: `release/0.8.25`
- Worktree: clean; 160 commits ahead of the remote branch

The independent reviewer made no repository or Git changes.

## Verdict

**FAIL.** FIX-12 closes the projection-transaction pause deadlock, but one P1
runtime-startup finding remains and requires FIX-13.

## P1 — successful engine open does not establish runtime startup

`ProjectionRuntime::new` spawns the projection dispatcher and two workers and
returns immediately. The dispatcher silently returns when its SQLite runtime
connection cannot open. Each worker silently returns when either its runtime
connection cannot open or vector-partition setup fails. `Engine::open` accepts
that unchecked runtime as successfully started.

The parallel reporter supplied direct race evidence: the immediate native
inventory probe timed out and reported all three runtime roles missing; their
`opened` events appeared only 303–309 ms later. A real setup error would be
worse than this bounded scheduling delay: engine open would still report
success, but projection work could never settle. This violates the locked
engine-open contract, which includes scheduler startup.

FIX-13 must make projection-runtime startup acknowledged and fallible. The
dispatcher and both workers must report their exact role and successful
connection setup, worker partition setup, managed registration, and WAL
registration before `Engine::open` returns. Failure, timeout, disconnection,
or a missing or duplicate role must stop and join the partial runtime and map
through the existing `EngineOpenError::Io` surface. Increasing the 250 ms
diagnostic timeout is not a correction.

Required RED coverage:

- preserve the immediate complete-native-inventory failure as successful-
  startup evidence;
- require deterministic invalid-path or injected runtime-setup failure to
  make `ProjectionRuntime::new` return `Err`; and
- require successful construction to return only after the exact dispatcher
  and two worker roles are registered and immediately queryable.

## FIX-12 review

The FIX-12 correction itself passes review:

- `decacd05b3847aabc73013bb13720ee20f34bbea` is a genuine compile RED;
- GREEN `747c66f5b073414467c9290737f3666f5793a9bb` does not edit the frozen
  tests;
- one-shot readiness and release are bounded and cancellation-safe;
- explicit `release()` and handle drop are idempotent and best-effort;
- the worker's release wait is bounded to 30 seconds;
- all four existing callers use the bounded handle;
- the exact five-BUSY, owning-role, autocommit, native-inventory, and
  post-release sampler assertions remain intact; and
- focused cancellation tests, every converted caller, the exact WAL witness,
  and four concurrent witness processes pass.

The Slice 55 plan correctly names `scripts/test-rust-workspace.sh --serial` as
the canonical gate and `--parallel-report` as a non-gating diagnostic under
TC-72/TC-74. The unconfined serial gate passed. Parallel reporting terminated
without a pause-hook timeout or deadlock; its startup race is independently
actionable even though the reporter itself is non-gating.
