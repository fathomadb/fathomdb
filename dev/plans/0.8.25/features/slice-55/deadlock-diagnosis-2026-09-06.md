---
title: 0.8.25 Slice 55 bounded deadlock diagnosis — 2026-09-06
status: PASS
---

# Slice 55 bounded deadlock diagnosis — 2026-09-06

An independent read-only verifier investigated the nine old hung
`fathomdb-engine` test processes recorded in
`recovery-checkpoint-2026-09-06.md`. All diagnosis used exact clean commit
`0e574227bf621aa9e4b94955cfc7af71c44f55d8`. No source, test, documentation,
or Git state changed during the investigation.

## Bounded commands and results

The verifier compiled only the two implicated test binaries before executing
them directly under external `timeout` watchdogs. Every test process ran with
`RUST_BACKTRACE=full`; `/usr/bin/time` captured elapsed duration and exit
status.

```text
cargo test -p fathomdb-engine --lib --no-run
  PASS; compile 13.45 s
  target/debug/deps/fathomdb_engine-6f2435238551b759

timeout --signal=TERM --kill-after=10s 180s \
  target/debug/deps/fathomdb_engine-6f2435238551b759 \
  tests::wal_attribution_projection_worker_typed_refusal_then_post_release_sampler_is_recorded \
  --exact --nocapture
  PASS; 1/1; exit 0; 0.19 s

cargo test -p fathomdb-engine --test slice45_pagination --no-run
  PASS; compile 5.94 s
  target/debug/deps/slice45_pagination-cb5452257b133c7d

timeout --signal=TERM --kill-after=10s 180s \
  target/debug/deps/slice45_pagination-cb5452257b133c7d \
  operational_page_snapshot_linearizes_before_concurrent_replacement \
  --exact --nocapture
  PASS; 1/1; exit 0; 0.08 s

timeout --signal=TERM --kill-after=10s 300s \
  target/debug/deps/fathomdb_engine-6f2435238551b759 \
  tests::wal_attribution_ --nocapture --test-threads=1
  PASS; 17/17; exit 0; 1.73 s

timeout --signal=TERM --kill-after=10s 300s \
  target/debug/deps/slice45_pagination-cb5452257b133c7d \
  --nocapture --test-threads=1
  PASS; 13/13; exit 0; 1.50 s

timeout --signal=TERM --kill-after=10s 600s \
  target/debug/deps/fathomdb_engine-6f2435238551b759 \
  --nocapture --test-threads=1
  PASS; 51/51; exit 0; 7.70 s
```

No watchdog fired. The final process scan found no Cargo, engine-test, or
Slice 45 test process. The final temporary-file scan found no matching
`wal-attribution-projection.sqlite.lock` or `state-race.sqlite.lock` orphan.
The worktree remained clean.

The diagnostic rebuild consumed approximately 2.0 GB. Available space after
the run was 191,593,484,288 bytes; `target/` occupied approximately 1.1 GB.

## Findings

The current commit does not reproduce a deterministic single-test,
test-family, engine-library, or Slice 45 pagination deadlock under serial
execution. No product deadlock correction is justified by this result.

The WAL attribution test retains a latent harness liveness hazard. Its main
thread waits on an unbounded two-party barrier in
`wal_attribution_projection_worker_typed_refusal_then_post_release_sampler_is_recorded`.
The worker arrives only after beginning an immediate transaction and passing
the closure guard, then waits on a second unbounded release barrier. A worker
failure before the first rendezvous, or a main-thread panic between
rendezvous, can conceal the original failure as a permanent harness wait.
This is an evidenced hazard, not a reproduced explanation for the killed
processes.

The Slice 45 tests share a process-global one-shot after-validation hook. Commit
`f4b8b68e015242a3b760d3287142e0cad95cdf87` added `RACE_HOOK_LOCK` serialization
for the three Slice 45 users on 2026-09-05. The preceding source lacked that
serialization, so hook theft is a plausible historical mechanism for an old
binary. The killed binary hashes and commits were not retained, so that
attribution remains an inference.

An open `<database>.lock` file identifies an `Engine` that remained open; it
does not prove a thread was waiting on that file lock. Likewise, threads parked
in `futex_do_wait` are compatible with barriers, condition variables, and idle
workers. Without user-space stacks or retained last-test output, the old WAL
attribution hangs cannot be assigned to a product lock-ordering defect.

## Release consequence

The earlier full gate remains failed because of `ENOSPC` and the stale
worktree Python extension; this bounded diagnosis does not convert that run to
green. Broader verification may resume at the current clean candidate, using
the repository's serial Rust route plus an external watchdog and retained
last-test output. If a current test hangs, capture its exact binary, commit,
last output, process tree, user-space stacks, and lock inventory before
termination, then return any correction through committed RED/GREEN and
implementation re-review.
