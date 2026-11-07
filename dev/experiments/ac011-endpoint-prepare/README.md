# AC-011 endpoint preparation diagnostic

This retained Phase 2 tool measures the exact dangling-edge endpoint statement
prepared by `write_commit.rs`. It is diagnostic evidence only: it has no
throughput acceptance assertion and cannot replace the unchanged AC-011 gate.
The Slice 95 CI16 design and Astra's concrete review authorize one isolated run
with the default parameters below. Further runs require reassessment.

`probe.patch` temporarily adds a test-only thread-local aggregate around the
existing `tx.prepare` call, without changing SQL, control flow, statement lifetime,
transaction/visibility guards, worker activity, triggers, caching or durability.
The timer ends immediately after successful preparation. Counters from that exact
statement distinguish initial preparation from SQLite `RePrepare`, `Run` and
`VmStep`; a separate counter records calls to `exists`. Only the foreground
test thread collects these values. Preparation is counted before transaction
completion; the sample requires every measured write to succeed.

The ignored unit test opens a fresh real Engine with the same default runtime
and tempfile storage as `perf_gates::ac_011a_write_throughput_1kb`. It repeats the
same anonymous Active Node (`kind=throughput`, `source_id=test:ac011`, 1,024 `x`
bytes, other optional fields absent), warms up for five seconds, then measures
1,000 successful commits. It captures elapsed time before JSON serialization and
reads canonical row counts outside the measured interval. Normal workers remain
active. Instrumentation cost is unknown; these timings do not prove a production
speedup or the cause of the earlier throughput miss.

| Environment parameter | Default | Meaning |
| --- | --- | --- |
| `AC011_DIAGNOSTIC_COMMITS` | `1000` | Positive measured commit count |
| `AC011_DIAGNOSTIC_PAYLOAD_BYTES` | `1024` | Positive Node body byte count |
| `AC011_DIAGNOSTIC_WARMUP_MS` | `5000` | Warmup duration in milliseconds |

The patch targets the production source at retained tool commit
`9f112e6dc93165c5ba54d7edd60f3aade395f37e`. The later repair extracts its
counting block; reproduce this historical diagnostic from an isolated checkout
of that tool commit after the normal repository worktree preflight.

Run from the assigned worktree. Keep its own target directory and build with two
jobs offline; do not borrow a Python environment or another checkout's target.
Compilation is permitted before reserving the isolated execution slot:

```bash
git apply dev/experiments/ac011-endpoint-prepare/probe.patch
CARGO_TARGET_DIR="$PWD/target" CARGO_BUILD_JOBS=2 cargo test --offline \
  -p fathomdb-engine --features test-hooks --lib \
  write_commit::endpoint_prepare_diagnostic::phase2_endpoint_prepare_diagnostic \
  --no-run
```

After Astra reviews the concrete patch and the parent grants the isolated slot,
run the exact compiled test executable printed by Cargo. Record its full path,
source/patch commit, debug test profile, features (`test-hooks`), `rustc --version`,
`cargo --version`, storage path, command, exit and complete raw output. Raw logs
belong in the persistent ignored Slice 95 evidence directory specified by the
parent; capture actual exit status rather than a trailing command's status.

```bash
AC011_DIAGNOSTIC_COMMITS=1000 AC011_DIAGNOSTIC_PAYLOAD_BYTES=1024 \
  AC011_DIAGNOSTIC_WARMUP_MS=5000 /absolute/path/printed/by/cargo \
  write_commit::endpoint_prepare_diagnostic::phase2_endpoint_prepare_diagnostic \
  --ignored --exact --nocapture --test-threads=1
```

The `AC011_ENDPOINT_PREPARE` JSON line reports configured parameters, fixture path,
warmup commits, measured elapsed nanoseconds, initial preparations and their
aggregate nanoseconds, explicit `exists` calls, statement repreparations/runs/VM
steps, and canonical row delta. The default Node-only sample asserts 1,000 initial
preparations, 1,000 added canonical rows, and zero endpoint calls/repreparations/
runs/VM steps. It also requires zero dangling endpoints on every write receipt.
These assertions check the measurement's attribution; they are not a production
correctness RED or throughput qualification.

Restore instrumentation before production TDD. The committed patch stays here so
the probe remains reproducible:

```bash
git apply --reverse dev/experiments/ac011-endpoint-prepare/probe.patch
```

## Release foreground wall/thread-CPU attribution

`release-wait.patch` is a separately reviewed diagnostic after the unchanged
canonical AC-011a failed on candidate
`13c6470b5b5700c61c68ce8d10257551fbd2c9ca`. It targets that candidate's production
source. Keep the original endpoint probe above as historical evidence; its
1,000-write debug sample does not characterize this release-mode miss.

The new patch adds an ignored integration sampler to `perf_gates.rs`, temporary
hidden Linux/test-hooks start/snapshot methods, and a private per-thread timing
collector. The engine library builds with `cfg(not(test))`, matching canonical
qualification. No unit-test sampler is used. No global physical metrics or SQL
trace is enabled. Worker scheduling, all existing calls, lock/guard/error/drop
paths, transaction protocol, NORMAL synchronization, checkpoint policy and
acceptance thresholds remain unchanged.

The collector reads `Instant` and checked libc `CLOCK_THREAD_CPUTIME_ID` in the
same order for each span (wall then CPU at start; CPU then wall at end). Only the
foreground thread is armed. A sample merges only when `Engine::write` returns a
successful receipt. The overlapping `Engine::write` reference includes the
existing operation guard's destruction; it is not added to exclusive stage sums.

| Exclusive stage | Existing boundary |
| --- | --- |
| `turn_wait` | `writer_turn()` acquisition |
| `writer_mutex_wait` | writer `connection.lock()` acquisition |
| `pretransaction` | post-lock guards, validation, enrolment/job collection and cursor metadata, ending before `commit_batch` |
| `begin` | actual `transaction_with_behavior(Immediate)` |
| `trigger_check_disable` | canonical trigger predicate and trigger suppression |
| `apply_body` | `apply_batch_in_transaction` |
| `visibility_advance_restore` | visibility advance, then a separate interval for restore and guard destruction |
| `commit` | actual `tx.commit()` |

Each stage reports successful sample counts, interval counts, wall nanoseconds,
thread-CPU nanoseconds and signed wall-minus-CPU nanoseconds. The visibility stage
has two disjoint intervals but one sample per successful measured commit; other
stages have one interval. Canonical Node writes must exercise every stage. The
report also includes reference totals, exclusive sums, signed uncovered wall,
CPU and non-CPU residuals, and a flag for negative residuals. No value is clamped.
Uncovered time includes lifecycle/initial checks, canonical-batch classification,
combinator gaps, receipt work, notification, remaining guard destruction and
instrumentation outside the measured spans.

The integration sampler retains the canonical anonymous Node, five-second
warmup and 60-second measured loop, including the final complete commit. It
captures original elapsed time before snapshot/report serialization. Defaults are
parameterized for rerunnability; this authorized single run uses exactly:

| Environment parameter | Value |
| --- | --- |
| `AC011_WAIT_WARMUP_MS` | `5000` |
| `AC011_WAIT_MEASUREMENT_MS` | `60000` |
| `AC011_WAIT_PAYLOAD_BYTES` | `1024` |

```bash
git apply dev/experiments/ac011-endpoint-prepare/release-wait.patch
CARGO_TARGET_DIR="$PWD/target" CARGO_BUILD_JOBS=2 cargo test --offline --release \
  -p fathomdb-engine --features test-hooks --test perf_gates \
  ac_011_release_foreground_wall_cpu_diagnostic --no-run
```

After concrete-patch review and the parent's isolated-slot grant, execute the
printed release integration executable once. Retain complete raw stdout/stderr,
actual exit, command, source/patch identity, toolchain/features/profile and fixture
path under the parent's persistent ignored evidence directory. No generated
results or receipts are committed.

```bash
AC011_WAIT_WARMUP_MS=5000 AC011_WAIT_MEASUREMENT_MS=60000 \
  AC011_WAIT_PAYLOAD_BYTES=1024 /absolute/release/executable/printed/by/cargo \
  ac_011_release_foreground_wall_cpu_diagnostic \
  --ignored --exact --nocapture --test-threads=1
```

`AC011_RELEASE_WAIT` contains the JSON report. Counts must reconcile with
successful measured commits. A stage qualifies for further focused investigation
only if its signed non-CPU total is both at least 50% of reference foreground
non-CPU time and at least 10% of reference write wall time. Examine signed
residuals and instrumentation overhead before attribution; incoherent counts,
material negative residuals, or no qualifying concentration mean inconclusive.
Stop after this one run. Commit concentration alone does not prove fsync or
checkpoint causation. No baseline comparison, syscall trace, automatic retry,
production repair or throughput acceptance claim follows from this diagnostic.

Restore all active instrumentation before any later production repair:

```bash
git apply --reverse dev/experiments/ac011-endpoint-prepare/release-wait.patch
```
