---
title: 0.8.25 Slice 60 independent verification — cycle 4
status: FAIL
candidate: f6d351c797807775d13938295afa0a274d6e75cb
---

# Slice 60 independent verification — cycle 4

## Verdict

**FAIL.** The exact AC-059b correction passes, and the release-wide AC-013
failure is explicitly owned by Slice 75. The required terminating parallel
workspace reporter nevertheless exposes a separate scheduling-sensitive
Slice 55 WAL-attribution oracle failure. The reporter terminates normally
rather than deadlocking, but its nonzero result must be corrected and reviewed
before Slice 60 closes.

## AC-059b correction

The unchanged long cursor-race command passed all 1,000 iterations with zero
violations in 28.76 seconds under an external 180-second timeout:

```text
AGENT_LONG=1 cargo test -p fathomdb-engine \
  --test cursor_read_after_write \
  projection_cursor_bounds_observed_row_count \
  -- --exact --nocapture --test-threads=1
```

## Release-wide AC-013 signal

The exact unconfined `./scripts/check.sh` route passed AC-059b and then reported
one failure in `perf_gates`:

```text
AC013_NUMBERS n=10000 samples=1000 seed_ms=10264 p50_ms=919 p99_ms=991
AC-013 failed: p50=919.459309ms > budget 80ms at n=10000
test result: FAILED. 9 passed; 1 failed; 6 ignored; finished in 5235.28s
```

After preserving that exact diagnostic, the remaining no-fail-fast workspace
tail was intentionally stopped instead of spending another long verification
window behind a known failure. The resulting command exit 130 is not claimed
as a complete CI pass. The handoff and Slice 75 plan explicitly assign AC-013
classification to Slice 75 through three alternating isolated release-mode
candidate/baseline repetitions. An independent read-only scope audit confirms
that this release-wide signal does not backfill or reopen Slice 60.

## Parallel reporter RED

The exact unconfined diagnostic was:

```text
timeout --foreground 7200 \
  ./scripts/test-rust-workspace.sh --parallel-report
```

It completed in approximately seven minutes with exit 101. Exactly one target
failed; the no-fail-fast tail otherwise completed:

```text
tests::wal_attribution_projection_worker_typed_refusal_then_post_release_sampler_is_recorded
assertion `left == right` failed
  left: 2
 right: 1
error: 1 target failed:
    `-p fathomdb-engine --lib`
```

The retained role timeline showed the projection worker inside its deliberately
paused write transaction while the dispatcher legitimately acquired a second
read snapshot:

```text
projection_dispatcher:0 phase=snapshot_acquired elapsed_ms=149
projection_worker:0 phase=transaction_opened elapsed_ms=149
```

The test's exact-one-active-role assertion therefore rejects an allowed
parallel runtime overlap before it reaches the intended typed-erasure oracle.
The correction must retain all production attribution, the paused worker,
every typed busy checkpoint attempt, cancellation-safe release, and the
post-release native-state sampler. It may only replace exclusivity with an
exact presence requirement for the paused projection worker across the active
snapshot and busy checkpoint records.

## State

The reporter left no running Cargo or test process. The durable worktree was
tracked-clean at the exact candidate before this record. No package production,
registry, tag, publication, merge, or release action occurred.
