# Slice100 Q1 child and operator protocol

Reviewed protocol at source-entry3c34cd19, now in approved implementation. Owner seq-371 approves the
reviewed100-only verification exception; seq-372 selects bounded Q1 on the local
unconfined ptrace-capable executor, including bounded reaping of the same actors.
[Exact decisions and technical holds](owner-decisions.md) remain binding. The maintained-plan recheck passed at c1226d1bf and source GO is recorded at
5f8f12071. Independent source review, actor/identity/runtime readiness and parent
technical GO still precede Q1 execution. [Execution](execution.md) distinguishes
source/control results from native witness/qualification;100 closure stays open.
The reviewed scratch packets remain unchanged.

## Actors and subject

One Linux stdlib-Python operator owns one GDB process and exactly one disposable
Rust test child at a time. Each phase has a fresh case directory and nonce under
an eventual parent-assigned /mnt/data task root. A separate observer/retry test
process starts only after the debugger/inferior are gone. No production database,
VM, GPU, model, network, host fault or other session's process is involved.

A future source-qualified test target `slice100_interruption` uses operator and
test-hooks. Its exact build JSON identifies the executable in its own source100
checkout/target; record source SHA/delta, relevant lock/config/features, binary
SHA256/build-id, debugger/native-symbol and SQLite source identities. Symbol-file
identity must match that binary. Reconcile then-integrated G/other source before
implementation; G is currently unintegrated. No borrowed S20 artifacts/toolchain.

The retained S20 candidate9df345e2… proves metadata availability only. Its quiet
canonical log does not bind the exact current file as the executed child. Its
PIE offsets are never runtime addresses. Future Q1 source/binary attribution is
separate from, and adds no binary gate to, reuse of the existing eight controls.

## One normal-success child

New ignored test `q1_child` takes a pre-created exclusive case directory, nonce
and fixture parameters. It neither knows nor branches on healthy/crash mode.
Unknown parameters, nonempty database path or duplicate events fail normally.
No existing85 test or production hook is changed.

1. Open a fresh format37 store with EmbedderChoice::None and persisted
   ExplicitMaintenance. Populate one target node and one survivor logical node
   with three retained revisions, distinct deterministic marker/body values,
   no telemetry or model/provider. Bound total input body bytes to64KiB. Record
   independent raw canonical/history/FTS witnesses and positive target/survivor
   marker occurrences across the raw main-plus-WAL union; neither individual
   file must contain both markers. Fail missing population. No fake pending WAL or
   manual checkpoint-state edit is allowed.
2. Obtain `begin_erasure_maintenance(1000)`, queue SourceAllAtCut(target-source)
   through the handle using a case-specific valid32hex key. Assert Accepted /
   UnchangedByThisRequest and record ticket, cut/scope and survivor snapshot.
3. Flush exactly one `Q1_READY` JSON event (nonce, PID, fixture subject and ticket)
   to stdout. Call test-only `#[inline(never)] q1_ready_after_queue()` with a
   black_box body. This marker exists solely to stop the owned harness before
   completion begins. It is not a production seam or a claimed failing behavior.
4. Call `complete_physical_erasure(ticket)` normally, with no fault hooks armed.
   Only CleanupResult::Complete is healthy success. Release the handle; assert
   current status Complete, target absent, survivor/history exact, root/debt
   retired and coalescing0. While Engine is live, save main/live-WAL raw copies
   and assert target bytes absent, survivor retained. Then close/drop, reopen
   persisted explicit policy, assert same one terminal receipt/no resurrection,
   close/drop and flush one `Q1_DONE` with the nonce. Return from the test normally.

The one-test libtest process must report1pass/0fail and exit0 after close/reopen.
Exit73/99, panic, Incomplete, missing/duplicate DONE, early completion or timeout
is not this healthy control. Existing85 crash_child remains byte-frozen; its
hook73/fallback99 behavior cannot be relabeled as normal Q1 completion.

## Debugger handshake and qualified native witness

GDB is launched with --nx --nh and no inherited user init/command files. Disable
all binary/script auto-loading, debuginfod/network and interactive confirmation
before loading the selected files. Use tokenized MI commands/responses and retain
raw output; do not parse arbitrary prose as a successful stop. No inferior calls,
assignment to locals, synthetic interruption flags or injected SQLite operation.

Before launch, the operator registers itself as the owned-child subreaper and
registers GDB PID/process-group cleanup immediately after spawn. GDB first stops
the new inferior at initial exec/entry, before its test body. Validate the single
inferior PID, executable, start-time and ownership chain, open its pidfd and
register cleanup before allowing fixture execution. No following forks or
external attachment. Then enable only the test ready-marker breakpoint and
continue. It must stop after that inferior's unique matching READY event. At this
stopped marker, resolve and enable the native-copy breakpoint, then resume. This prevents setup/open checkpoints or a READY/enable race from being
mistaken for the selected erasure call. Failure to resolve stops the case.

Current inspected source is libsqlite3-sys0.39.1/sqlite3/sqlite3.c SHA256
0c657666d819f261cc4007526ad57e1669de9949df7a8ea847aa284a2bf0ee52.
The candidate row69275 is the sqlite3OsWrite call inside walCheckpoint, after the
eligible-frame checks and successful WAL read. The preceding read is69271;
CKPT_DONE is69278. Candidate static mappings0x112dba7/0x112db6b are metadata only.
At future source, verify source-path mapping and the actual relocated instruction
inside that function; a source-line breakpoint sliding elsewhere is unqualified.

At the stop, all of these must be readable and true in this same execution:

- Exact announced owned inferior, executable/symbol subject and breakpoint ID;
  current PC resolves inside walCheckpoint at the eligible frame-copy instruction.
- Actual stack includes sqlite3WalCheckpoint and the engine erasure route through
  wal_checkpoint_truncate_once, checkpoint_erasure_at_rest and explicit ticket
  completion. It is not a boot/autonomous/offline/unrelated checkpoint.
- rc==SQLITE_OK, pIter!=NULL, nBackfill<mxSafeFrame, iFrame>nBackfill,
  iFrame<=mxSafeFrame,0<iDbpage<=mxPage and szPage>0; the frame data read succeeded.
  Check eMode==SQLITE_CHECKPOINT_TRUNCATE from the pinned source declaration.
  The active frame has not returned; retain stack/locals/PC before any action.
- READY names this ticket/case; no DONE or Complete outcome occurred beforehand.
  No post-call hook, BUSY report, function-entry stop or sleep is a substitute.

If required locals are optimized out, stack names cannot be resolved, ptrace is
unavailable, source mapping is ambiguous, the stop is wrong/empty or a witness
cannot be retained, classify UNQUALIFIED and stop. Do not patch/rebuild SQLite,
add a VFS/production seam, weaken predicates, relocate venue or substitute timing.
A different build/debug approach would need its own concrete reviewed proposal.

## Matched healthy and interruption cases

Run one healthy case first: after this exact qualified stop, disable the native
breakpoint, continue and require the child's normal success protocol and raw
pre-close proof. No repeated campaign or broad source suite follows by default.
The healthy case establishes the protocol's ordinary successful route only.

Only if that healthy case passed on the identical source/binary/config/fixture
subject may one matched interruption case start. It must obtain its OWN complete
witness. The earlier healthy witness or an earlier stop in this case never grants
a later timed kill. At its qualified stopped frame, and only under seq-372 and parent technical GO, send SIGKILL through the retained pidfd to that exact inferior.
Wait for GDB's child-exit report and reap GDB. No target method or destructor runs
as a substitute for abrupt exit. Missing/changed identity cancels qualification.

## Untouched crash evidence, observer and explicit retry

Once all writer/debugger actors are dead, before any SQLite/Engine open or retry,
copy/hash the complete original main/WAL/SHM and note absent sidecars explicitly.
The operator has exclusive case ownership. Preserve originals and the first
snapshot; no close/checkpoint/WAL rewrite is used to clean the evidence. Raw
bytes may still contain target markers here; absence is required only at Complete.

Use a working copy of that quiescent snapshot for an independent read-only/query-
only SQL observer. Never use immutable=1 on a WAL-backed image, since it can
ignore WAL. SQL state must be the effective main-plus-WAL view. Any SQLite
sidecar recovery affects the working copy only; never the preserved snapshot.
The raw observer does not run Engine, recover_complete_erasures or proof helpers.
For Q1's no-vector fixture, no vector extension query/registration is needed.

Before retry assert Finalizing/Removed, exact target canonical/FTS zero, exact
survivor/history values, coalescing0, identity-free request/root and no Complete
receipt. On a separate retry copy, reopen Engine with persisted explicit policy,
observe the same state before maintenance, assert an unrelated write is fenced
without effects, and assert allowed survivor reads. No autonomous progress is
claimed. Acquire a handle and retry the SAME ticket once; require Complete and
one receipt, exact structural zero and main/live-WAL target absence before close.
Release, close/drop/reopen and assert no resurrection or duplicate receipt.

Malformed WAL/open refusal, changed pre-retry state, lost survivors, false terminal
success or retry failure is preserved as a finding; no operator recovery is used
to turn it into success. A confirmed baseline contract failure is minimized for
real per-finding committed RED then separate GREEN with the oracle unchanged.

## Bounds, abort and ownership

Proposed harness deadlines:45s to READY,30s from READY to qualifying native stop,
30s after healthy release or retry,120s total per case,5s reap grace. These are
harness bounds, not product SLAs. Register all owned PIDs/process groups/pidfds
before blocking. Do not keep an observer snapshot open during checkpoint or
retry; record every observer lifetime so the control cannot pin its own WAL.

On any unavailable/wrong/empty witness, request orderly debugger abort/child
shutdown if available; cleanup never returns QUALIFIED. If bounded cleanup needs
termination, use only registered owned actors and reap them under the explicitly
seq-372 cleanup scope after parent technical GO. An unqualified cleanup kill is
not a Q1 fault witness, RED or product result. Without source review, actual
readiness and parent technical GO, the fault-capable operator must not launch. Never kill by name,
by unvalidated PID, all Rust processes, or another session's process group.
