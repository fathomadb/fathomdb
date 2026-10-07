---
title: Slice 135 projection commit recovery diagnostic
status: INTERIM_ROBUSTNESS_RESULT
target_release: 0.8.27
---

# Projection commit recovery diagnostic — 2026-10-07

The six existing TC-91 real-database tests passed once through Cargo and ten
more times by direct execution of the same test binary. This is an additional
**candidate-only** system robustness and exception-handling result; it does not
close the [full Phase 1 matrix](../../phase1-protocol-draft.md). The tests
exercise one-shot faults through debug-only hooks and keep the product's real
SQLite and projection worker in the loop.

## Identity and command

- Exact product source: `b2ac8081e79a6e626d01f5f97331be331f1cc16d`.
  The Slice 135 branch was at `a76731af0d3c3a212255521c8404476a2a440061`
  for this run; `git diff --quiet b2ac8081 HEAD -- src/rust src/python src/ts
  Cargo.lock` returned zero.
- `Cargo.lock` SHA-256:
  `9e9d7b5e82184a0bddfbe96de28fa0615ef4443fc84639b271c1cff29600ccfe`.
  The retained [test source](test-source.rs) SHA-256 is
  `49236d66b3c3c9d98bd076dea47711ed0e9a0b48503c40088e643c50d1f92cb0`.
  The built test binary SHA-256 was
  `e9181b6850ced826f364554d6798fb719d813af187e3aa2b2b6d4030c51b3c74`.
- Rust/Cargo 1.95.0, Linux 7.0.0-38-generic x86_64. Cargo used the worktree's
  cached debug test target; this is a behavior and resource observation, not
  release-mode latency evidence.

```sh
CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 \
  timeout 300s /usr/bin/time -v \
  cargo test --locked --offline -p fathomdb-engine \
  --test tc91_projection_commit_hardening -- --nocapture --test-threads=1
```

The [raw Cargo output](test.log) reports **6 passed, 0 failed** in 0.59 s
after a 0.41 s incremental compile. Its two printed panics are intentionally
injected and recovered; the test process exited zero. [GNU Time](resource.txt)
reported 1.05 s wall time for the command, 233,360 KiB peak RSS, 89 major
faults, zero child swap events, and exit zero. The test databases are fresh
temporary real SQLite files for each case.

## Assertions actually exercised

| Fault or schedule | Existing test assertion |
| --- | --- |
| Forced SQLite busy at projection commit | Caller write survives; worker error event has `SQLITE_BUSY`; a second embed/dispatch produces a vector; projection failure count remains zero. |
| Non-SQLite storage error at commit | Worker error event is `StorageError`, not mislabeled SQLite; durable pending work is redispatched to a vector. |
| Subscriber panic while reporting failed commit | Worker cleanup still redispatches, reaches bounded `drain`, and produces the vector. |
| Embedder panic plus failed panic-terminal commit | The failed terminal does not become durable; normal redispatch produces a vector. |
| Failure as mean-vector threshold is crossed | Rollback does not consume or publish the pin twice; exactly one pin event appears after redispatch. |
| Stop wins before failed-commit cleanup | Reopen reconstructs pending work from the durable database and reaches vector-ready state with zero terminal failure count. |

All ten direct repetitions ran the same SHA-256 test binary with
`--nocapture --test-threads=1` and a ten-second subprocess timeout each.
The [attempt ledger](repeats.json) records every exit, elapsed time and raw
log hash; [repeat logs](repeat-01.log) through `repeat-10.log` are retained.
Each repetition reports six passes, with subprocess elapsed time 588.65–602.23
ms. Repetition checks this deterministic schedule for flakiness; it does not
sample randomized schedules or simulate a crash inside the commit.
An [independent audit](audit.json) rehashed all ten logs, checked the six
named test outputs and run summaries, and rejected a changed pass count.

These tests use debug-only injection hooks. The retained test source is the
state oracle; the runner output establishes that its assertions completed but
does not independently inspect SQLite tables. The run did not capture
before/after environment snapshots, a production-build run, provider timeout,
disk-full/permission failure, cancellation under load, interrupted erasure,
or a power-loss model. The larger robustness matrix remains open. The next
projection fault receipt should retain pending/committed state at the fault
point and after reopen and include a negative control that a missing vector
fails the oracle.
