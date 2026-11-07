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
