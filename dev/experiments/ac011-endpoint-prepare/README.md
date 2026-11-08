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

## Bounded canonical sync-only trace

`sync_trace.py` retains the separately Astra-reviewed sync-only follow-up.
`test_sync_trace.py` uses small hand-computed traces for parser math, foreground
identity, worker separation and unfinished/resumed accounting. Neither tool
changes or rebuilds the canonical engine. The parent selects the unmodified
canonical executable, retains its source/binary identity separately, and owns
this sole workload execution slot.

The runner requests strace `--seccomp-bpf -q -f -ttt -T -yy` with only
`fsync,fdatasync,prctl` selected. Root's strace 6.8 preflight confirmed
`seccomp filter enabled`; retain that preflight alongside the actual evidence.
The tracer writes through a passed pipe descriptor. A bounded writer retains
at most **33,554,432 bytes (32 MiB)** of raw trace; reaching that cap or the
**120-second watchdog** terminates then kills the owned process group and marks
the result inconclusive. A two-second termination grace may follow the watchdog.
No file-size resource limit is applied to SQLite. Tracee stdout/stderr go directly
to task files so pipe draining cannot deadlock on their output.

Run parser tests without tracing:

```bash
python3 -m unittest discover -s dev/experiments/ac011-endpoint-prepare \
  -p test_sync_trace.py
```

Only after Astra's static approval and root's isolated-slot grant, run once:

```bash
AGENT_LONG=1 FATHOMDB_TIER1_WRITE_PERF=1 python3 \
  dev/experiments/ac011-endpoint-prepare/sync_trace.py run \
  --output-prefix /persistent/ignored/evidence/canonical-sync \
  --binary /absolute/unmodified/canonical/perf_gates-executable -- \
  ac_011a_write_throughput_1kb --exact --nocapture --test-threads=1
```

`--max-bytes` and `--timeout-seconds` expose those default bounds for reuse;
this authorization uses the exact defaults. `--fixture-basename` defaults to the
canonical `AC-011a_write_throughput.sqlite`. The output prefix supplies `.trace`,
`.stdout`, `.stderr`, `.metadata.json` and `.report.json`; all belong in persistent
ignored storage, never source control. Metadata retains the actual tracer/tracee
exit, command, elapsed time and any cap/timeout/setup stop reason. Exit 101 from a
completed canonical test is retained as its benchmark failure, not a tracer error;
the runner propagates it when attribution is otherwise conclusive. Exit 2 means
inconclusive analysis. No wrapper exit is a throughput acceptance verdict.

Re-analyze retained evidence without running any command:

```bash
python3 dev/experiments/ac011-endpoint-prepare/sync_trace.py analyze \
  --output-prefix /persistent/ignored/evidence/canonical-sync
```

The analyzer finds the foreground TID through successful
`PR_SET_NAME("ac_011a_write_t")`, identifies the unique resolved fixture DB/WAL
stem, and separates every other TID. It groups resolved paths as DB, WAL,
directory or other, reporting calls, successes, errors and exact decimal duration
sums/maxima. Unfinished/resumed pairs count once; unmatched records remain explicit
incompleteness. Sync errors, unresolved TID/FD/fixture identity, parser loss,
truncation, watchdog or tracer diagnostics/fallback make the result inconclusive.

This is **whole-test scope**, including startup, five-second warmup, 60-second
measurement and teardown. Foreground fixture DB/WAL observed sync sum **≥5.0 s**
permits read-only checkpoint/storage investigation as material observed spans.
Below that criterion means only that material spans were not established here;
it proves nothing about locks or historical runs. No measured-window percentage,
gain, fsync/checkpoint cause, production change or acceptance claim follows.

## User-owned benchmark with attached CPU sampling

`cpu_profile.py`, `test_cpu_profile.py` and `test_cpu_profile_cleanup.py` retain
this separately approved CPU profile. Tests cover hand-computed attribution,
thresholds, terminal cleanup/exit accounting and FIFO ordering with
only normal-user `/bin/true`. The worker never runs the canonical profile or a
privileged smoke. Root owns the sole reviewed workload slot and desktop approval.

The runner launches `/bin/sh -c 'kill -STOP "$$"; exec "$@"'` as a new process
group in the existing session, observes STOP and checks unchanged UID, SID and
autogroup. The benchmark executes as the normal user; only perf is privileged:

```text
pkexec --disable-internal-agent /usr/bin/timeout --signal=INT --kill-after=2 120 \
  /usr/bin/perf record -D -1 --control=fifo:CTL,ACK -e cpu-clock:u -F 99 \
  --call-graph dwarf,8192 --max-size=256M --no-buildid-cache -p PID -o -
```

User-owned nonblocking FIFOs enable events, require `ack` and only then send
SIGCONT. Inherited events cover later benchmark threads. No new session,
`preexec_fn`, kernel/capability/sudo configuration or root-owned database is used.
Authentication precedes the privileged timeout: while approval is pending, the
benchmark PID stays stopped/reserved until the collector is terminal. Cancellation
queues FIFO `stop`; desktop authorization may need user action to finish/cancel.
The runner never signals a root PID. Actual benchmark execution has a 120-second
watchdog; perf has its separate privileged 120-second timeout and two-second kill
backstop. Authentication wait is not claimed to be inside either collection bound.

Perf streams into a normal-user writer with a hard **268,435,456-byte (256 MiB)**
cap, including headers. On benchmark completion/failure, cap or watchdog, FIFO
`stop` ends collection and the pipe drains/flushed output is retained. Cap,
collector/export failure or incomplete output is inconclusive. `DEBUGINFOD_URLS`
is cleared; recording avoids build-ID-cache writes and export uses a task-owned
build-ID directory. Metadata includes exact commands, perf version, identity,
actual benchmark/collector exits, stop reason and authentication-inclusive elapsed
time. All raw output and JSON belong under an ignored evidence prefix.

After review, root runs the unchanged canonical executable/environment once:

```bash
AGENT_LONG=1 FATHOMDB_TIER1_WRITE_PERF=1 python3 \
  dev/experiments/ac011-endpoint-prepare/cpu_profile.py record \
  --output-prefix /persistent/ignored/evidence/cpu-profile --binary BINARY -- \
  ac_011a_write_throughput_1kb --exact --nocapture --test-threads=1
python3 dev/experiments/ac011-endpoint-prepare/cpu_profile.py analyze \
  --output-prefix /persistent/ignored/evidence/cpu-profile
```

The currently supplied executable is the canonical .28
`target/release/deps/perf_gates-3ffa5706c34cc98f`, SHA-256
`bc61c376e060309450cdfdc01d9db5cbf45c33ab87f6939deb750312d135e2a4`;
root retains its source/environment binding separately. No Rust build is needed.
Benchmark exit 101 remains its actual test result, not a profiling setup failure.

Analysis exports explicit `comm,pid,tid,time,event,ip,sym,dso` fields with demangled
full call chains and lost-event records. The exact `ac_011a_write_t` comm identifies
one numeric foreground TID; all other TIDs remain separate. Finite source-backed
anchor lists live in the script: the innermost compilation, FTS flush/merge or
engine-guard anchor assigns at most one family. An unknown frame before an anchor
makes the sample unclassified; an unknown outer frame does not erase an already
known anchor. Raw leaves, chains, selected anchors and unknown/lost/unwind flags
remain available for audit.

Require **≥95% known foreground leaves**, without claiming complete unwinds. A
family must have **≥20% of all foreground user-CPU samples AND ≥200 samples** to
support a focused design only. Workers never enter that denominator. Explicit
loss/unwind/export/truncation warnings make the result inconclusive. Whole-test
sampling is not throughput acceptance, measured-window attribution or proof of a
speedup. No retry, policy change or production repair follows automatically.

```bash
python3 -m unittest discover -s dev/experiments/ac011-endpoint-prepare \
  -p 'test_cpu_profile*.py'
```

## Process-local offline libdwfl bias correction

`dwfl_bias.c` is the separately Astra-approved, narrowly configured interposer
for re-exporting the existing recording. It changes only `dwfl_report_elf` calls
whose `file_name` exactly equals `FATHOMDB_DWFL_TARGET`, whose base equals
`FATHOMDB_DWFL_BAD_BASE`, and whose `add_p_vaddr` is false. First registrations
and fallbacks forward every remaining argument and native return value unchanged
through `dlsym(RTLD_NEXT, ...)`.
For these exact matches only, `fd==-1` permits lookup with real
`dwfl_addrmodule(dwfl, corrected_base)` and `dwfl_module_info`. Reuse requires a
native module whose start equals the corrected base, whose range is nonempty,
and whose mainfile exactly equals the target. The interposer returns that existing
native handle instead of re-registering during an active DWARF walk. Wrong/missing
metadata falls back to the normal corrected report. Nonnegative FDs always reach
the real reporter, preserving its ownership/consumption behavior. Neither native
lookup API nor any sample address is intercepted or falsified. Their ABI comes
from the primary [module-info](https://www.sourceware.org/elfutils/coverage/libdwfl/dwfl_module_info.c.gcov.html)
and [address-lookup](https://www.sourceware.org/elfutils/coverage/libdwfl/dwfl_addrmodule.c.gcov.html)
implementations.

No filename normalization, ELF mutation, installed-tool patch, global preload,
new recording or workload is involved. The original inconclusive export remains
retained; corrected output must use separate files.

`dwfl_bias_abi.h` supplies only native ELF64 opaque declarations and the signature
from the [primary elfutils implementation](https://www.sourceware.org/elfutils/coverage/libdwfl/dwfl_report_elf.c.gcov.html),
lines 293–295. System `<elf.h>` supplies `Elf64_Addr`; `<stdbool.h>` supplies `bool`.
No elfutils development package is required. `test_dwfl_bias.c` builds a tiny fake
next library and driver; `test_dwfl_bias.py` exercises the actual preload ABI,
exact/near/other filenames, wrong/already-correct bases, true `add_p_vaddr`, all
other arguments, returned pointer and two audit records. The additional
`test_dwfl_bias_reuse.c` / `test_dwfl_bias_reuse.py` fixture verifies duplicate
native-handle reuse and missing/wrong filename/start/range/FD fallbacks. The seven
original controls use fd77 to keep exercising the native reporter. Compiler defaults are
`cc -std=c11 -Wall -Wextra -Werror`; `CC` and `DWFL_BIAS_TEST_OUTPUT` configure reuse.

```bash
python3 -m unittest discover -s dev/experiments/ac011-endpoint-prepare \
  -p 'test_dwfl_bias*.py'
mkdir -p "$BIAS_OUT"
cc -std=c11 -Wall -Wextra -Werror -fPIC -shared \
  dev/experiments/ac011-endpoint-prepare/dwfl_bias.c -ldl \
  -o "$BIAS_OUT/libdwfl_bias.so"
```

Use an ignored user-owned `BIAS_OUT` and the exact recorded `PROFILE_BINARY`
filename. Root owns the reviewed offline export from unchanged `RETAINED_DATA`:

```bash
FATHOMDB_DWFL_TARGET="$PROFILE_BINARY" \
FATHOMDB_DWFL_BAD_BASE=0x59018f058000 \
FATHOMDB_DWFL_CORRECT_BASE=0x59018f057000 \
FATHOMDB_DWFL_AUDIT="$BIAS_OUT/corrected.audit" \
LD_PRELOAD="$BIAS_OUT/libdwfl_bias.so" DEBUGINFOD_URLS= \
  /usr/bin/perf --buildid-dir "$BIAS_OUT/buildids" script --demangle \
  --show-lost-events -F comm,pid,tid,time,event,ip,sym,dso -i "$RETAINED_DATA" \
  > "$BIAS_OUT/corrected.script" 2> "$BIAS_OUT/corrected.stderr"
```

The example bases describe this retained record only; both are configurable env
values, not compiled-in addresses. Target filename comparison is exact. Use a
fresh audit file per export: every corrected call appends its process-local
total count, action (`registered` or `reused`), per-action count, before/after
bases and filename. `registered` counts forwarded native-registration attempts:
the audit precedes the real call and does not assert native success. `reused`
counts confirmed existing native handles returned without re-reporting.
Audit open/write/close or symbol-resolution
failure terminates explicitly; corrections are never logged to decoder stderr,
and actual decoder warnings remain untouched. Generated `.so`, fake ABI binaries,
exports, audit files and test logs stay ignored under `target/`.

Root/independent verification must audit representative corrected callers against
retained stack bytes and CFI before aggregating. Existing classification rules,
95%/20%/200 gates and original inconclusive results stay unchanged. This tool
repairs only the configured offline module-bias input; passing its fixture is
not proof of recovered unwinds, workload improvement or acceptance.

### Standard independent offline decoder

Samply's published perf importer uses Framehop, an independent unwinder:
[released importer source](https://github.com/mstange/samply/blob/samply-v0.13.1/samply/src/import/perf.rs).
A similar ELF load-bias failure is discussed in the
[elfutils maintainer thread](https://sourceware.org/pipermail/elfutils-devel/2023q1/005935.html).
These sources justify independent offline decoding, not another recording.

Install pinned `samply` 0.13.1 from crates.io into an ignored private tool directory
with `cargo install --locked --version 0.13.1 --root TOOL_ROOT samply` (supply a
separate `--target-dir`). Convert a copy of pipe-mode data using plain
`perf inject -i ORIGINAL.data -o FILEMODE.data`, without JIT/itrace options.
Require complete raw sample-record preservation before using the converted file.
Then run the standard command locally:

```bash
DEBUGINFOD_URLS= samply import --save-only --unstable-presymbolicate   --output independent.json FILEMODE.data
```

Keep task-owned XDG cache/config directories. No server, browser, upload, new
recording or database rebuild is needed. Use sample rows, not flamegraph widths;
no thread reuse, recursion folding or filtering. Inspect raw timestamps, numeric
TIDs, module-relative PCs and library build IDs. The 0.13.1 importer drops its
internal truncation marker, so import success is not complete-unwind evidence.
Retain any architecture warning and verify actual ELF/register architecture.
Pair raw samples and independently validate recovered callers against CFI/stack
bytes. Preserve original exports and all unchanged attribution/acceptance limits.
