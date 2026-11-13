# Slice 100 bounded design and approved verification scope

## Existing mechanisms versus net-new work

The [census](coverage-matrix.md) and [requirements](requirements.md) are normative
for this candidate's scope. Existing deletion remains one IMMEDIATE transaction;
queue activation and RootDebt join that commit. `physical_erasure::transaction`
owns sanitation/redacted/final receipt transactions and explicitly rolls back
failed COMMIT. Finalization scrubs target-bearing queue/root material before
checkpoint; final receipt joins root/closure retirement after actual TRUNCATE.
Foreground, autonomous worker and offline operator use these shared authorities,
but have different checkpoint callers and scheduling. No source repair is
currently justified by an observed defect.

Format37 checks clean visibility before/inside standalone writers. Erasure's
native transaction is not a `VisibilityCoalescingGuard::arm` interval; ordinary
managed triggers advance visibility, and authenticated vector mapping DELETE
cleanup remains active. Existing visibility guard rollback/admission tests and
span deletion controls are preserved. No migration, new format, suppression
flag or erasure-specific cache is proposed. An abrupt rich-erasure assertion
should read coalescing0 after recovery; it must not fabricate armed state as if
the actual erasure route commits one.

## Fixed fault/state oracle

The following expectations derive from the accepted maintenance contract, not
from the result being tested. They refine the existing85 crash parent without
changing its eight reached-exit/visibility/retry assertions.

| Fault boundary | Committed authority on reopen before retry | Authorized service and terminal expectation |
| --- | --- | --- |
| Before queue commit | No accepted ticket; target/survivor intact. | New acceptance still possible; no invented removal/success. |
| After queue / before delete commit | Queued; UnchangedByThisRequest; exact original selector/cut/scope retained. | Queued guards only; target remains visible subject to ordinary lifecycle view. |
| After delete / before first sanitation commit | Executing/Removed; root+ticket+canonical/projection removal atomic; sanitation prior progress. | Explicit global mutation fence through handle release/reopen; Autonomous retains only certified unrelated-write envelope. |
| After positive sanitation unit / before redacted commit | Sanitizing/Removed; committed sanitation progress, target-bearing root/ticket authority still retained until scrub commits. | No Complete. Preserve exact original obligations and permitted retry. |
| After redacted commit / active checkpoint / before completion commit | Finalizing/Removed; identity-free root/queue; no terminal receipt yet. | Global mutation fence. Raw WAL may still contain target bytes until actual successful TRUNCATE; no false success. |
| After completion commit | One Complete receipt, clean sanitation and no active root/debt; target-bearing queue metadata absent. | Authorized survivor/history intact. Exact retry is idempotent; later new writes are new events, not perpetual cleanliness proof. |

Autonomous progress can race a reopened observation. Use the existing independent
observer/no-follow-up control to prove eventual progress; do not disable that
worker and claim autonomy. Exact intermediate-state qualification should use
persisted ExplicitMaintenance and a separately opened observer before any retry.
Capture physical evidence while Engine/observer ownership remains live, then
close. Closing before the WAL assertion can erase the evidence.

## G100-1: active-checkpoint reachability

Local primary-source inspection only: Cargo.lock pins libsqlite3-sys0.39.1 and
rusqlite0.41.1. Cached SQLite amalgamation SHA256 is
`0c657666d819f261cc4007526ad57e1669de9949df7a8ea847aa284a2bf0ee52` at
`/home/coreyt/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/libsqlite3-sys-0.39.1/sqlite3/sqlite3.c`.
`walCheckpoint` lines69153–69349 show CKPT_START before frame copying,
interrupt checks inside the frame loop, CKPT_DONE after copying, then restart/
TRUNCATE. `OP_Checkpoint` lines104469–104501 calls that work as one VM opcode.
These are inspected source facts, not proof of runtime reachability.

No existing FathomDB hook lies inside that active native call. Busy handlers are
disabled by the online erasure/worker checkpoint path, so a custom busy callback
would change the exercised path. Progress/pre/post/trace callbacks are not assumed
to rendezvous inside frame copying. Calling interrupt before invocation or killing
after a delay is inadequate. CKPT_START/DONE are VFS file-control notifications;
using them as callbacks requires a VFS control, not merely arming a Rust hook.
No new VFS/disk/power campaign or SQLite patch is selected here.

Q1 is selected by seq-372 within the separate seq-371 verification exception;
source review, runtime readiness and parent technical GO still precede execution.
The [maintained protocol](q1-protocol.md) specifies: supervise only an owned disposable explicit-
maintenance child under an existing debugger; break at the pinned native
`walCheckpoint` frame-copy instruction with an erasure checkpoint Rust callstack,
positive pending frames and no native return yet. Record exact inferior PID,
binary/source/debug-symbol identity, breakpoint address/line and stack, then
abruptly terminate that inferior. A launch under the wrong stack, unresolved
symbol, pre-call breakpoint, empty checkpoint, timeout or missing reached witness
is unqualified. An identical release-through control must finish normally.

Reviewed metadata finds GDB15.1 and a retained S20 PIE candidate with native
checkpoint symbols and DWARF mappings for the eligible frame-copy region. Its
source hash matches the reviewed SQLite file; its current exact file is NOT
bound as the executed canonical child by the quiet log. Never borrow that binary.
Future100 needs its own source-qualified child and actual relocated breakpoint,
readable locals/stack and owned actor identities. Metadata and the approved
unconfined executor do not prove runtime ptrace/witness viability. No inferior,
ptrace or fault has run for100. Missing preconditions stop UNQUALIFIED; no timed
kill, broader fault or artificial product RED replaces them.

## G100-2: richer assertions and strict TDD

The proposed small composition would combine the existing85 rich success fixture
and independent survivor-only/history oracles with selected existing transaction
exits, adding direct target span joins and exact pre-retry phase/root/queue checks.
Positive population must establish all four FTS families, retained target and
survivor revisions, dependency rows, non-head vector mapping/span and original
sink ID before fault. Final raw scans occur before close; permitted audit/closure
carve-outs stay. Existing proof validators and expected results are not reused
as the sole independent oracle. Do not multiply every family across every hook.

This is coverage-only work, not a proven behavior repair. Owner seq-371 now
approves exactly the reviewed new child/operator controls and four existing-hook
composition cells as honest passing-on-arrival coverage commits. Seq-372 separately
selects bounded Q1 fault/cleanup on the local unconfined executor. Their exact
scope is in [owner decisions](owner-decisions.md); seq-369 remains20-only.

[Composed oracles](composed-oracles.md) bind all six R100/A100 rows and the four
G2 cells: before-delete rollback, after-delete removal, after-redacted identity
scrub, and one original-sink post-rename replay. Rollback preserves full-fixture
statistics; independent survivor-only ranking applies after committed removal.
Q1 uses the new normal-exit0 child and one witnessed healthy release-through
before its independently witnessed SIGKILL case. Raw main/WAL/SHM are preserved
before any SQLite/Engine open or retry. Existing85 files stay byte-frozen.

No product defect is currently established. Actual defects retain observed
committed runtime RED and separate GREEN; missing hooks/symbols/compilers or
synthetic mutants are not RED. This plan update remains before implementation,
and source review/runtime readiness/parent technical GO remain before faults.

## Repair and dependency boundaries

If existing current selectors expose a real defect, first minimize its accepted
invariant and observed assertion on the actual integrated baseline. Commit that
RED, then make only the necessary source correction in a separate GREEN; keep
the test unchanged and independently verify both SHAs. Source changes to the
shared finalizer require affected explicit/autonomous/offline controls. A genuine
recovery/round-trip-layer change retains existing properties and adds a compact
property only if a new invariant needs it, with human-authored intent.

Actual10 is closed: implementation104dcbcbe92a1eca730d146b99ab17d444df6217 has
fresh independent implementation PASS, with parent closure d7d928f8e reachable at
stable02e30d861beddc8061af0dc22f12106d5e09dd32. Read its completed execution,
terminal review and actual setup/pytest callers. Early `--check-ownership`
precedes installers/hooks/npm; only `--provision` creates the own venv. The
helper builds a noneditable own wheel and validates actual interpreter prefix,
module path/digest, ABI/features/source. The canonical Python suite scopes own PYTHONPATH
to parent/children under the unchanged nonce/source/native receipt gate; installed
production remains separate.18 actual RED/GREEN pairs and214 registrations do
not turn the failed original211 collector into an uninterrupted full-green.

This resolves the old pending10 dependency. Actual source-entry3c34cd19
includes accepted70 e875f360 and20 A3/A5 proof/capacity subset f803a964. The named
erasure/crash/checkpoint inputs remain unchanged from02e30d; whole src and lock
are not unchanged since2f691.70 changed embedding/reasons/native/MCP behavior;
20 added verification controls, while30 added SDK-only lock dependencies. The
current f803 run, rather than blanket old evidence inheritance, qualifies the
then-integrated runtime. Fresh independent review of this in-checkout update remains.
The parent supplied the fresh checkout and passed closed10/resource preflight;
own ownership check passed. Source GO, own native setup and runtime readiness
remain later. Current phase permits only planning/Markdown tooling, no native
tests, debugger execution or faults.

Actual20 f803 canonical execution supplies usable existing-control inference
from the unfiltered serial workspace feature closure and matching quiet Rust
count/child block, including all eight85 crashes. Entire src/lock match f803→
03bdd and source-entry3c34cd19. Quiet output omits phase names/PIDs/Rust binary digests; retain those
provenance limits and independent review. This supports G100-3 reuse without a
duplicate campaign or new hard20 dependency. Accepted qualification assembles
216/216 suites after the original module-policy failure and bounded continuation;
Rust2726pass/67ignored, Python1750pass/36skip and TS629pass/6skip plus15body-skip
markers are not an uninterrupted full-wrapper PASS.20 A3/A5 and RAW capacity
preparation are accepted; A6/whole20 remain held. No20 result fills G100-1/G100-2
observations absent from the unchanged assertions. Later integrated engine,
manifest, lock or caller changes require affected-input review; unintegrated G
work is future work, not a current predecessor.160 owns final composed
qualification. No unconditional all-release rerun rule.
