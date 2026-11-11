# Slice 100 bounded design and policy-held extension

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

Q1 is a bounded **unselected** alternative for independent review and the specific
policy/fault-control disposition: supervise only an owned disposable explicit-
maintenance child under an existing debugger; break at the pinned native
`walCheckpoint` frame-copy instruction with an erasure checkpoint Rust callstack,
positive pending frames and no native return yet. Record exact inferior PID,
binary/source/debug-symbol identity, breakpoint address/line and stack, then
abruptly terminate that inferior. A launch under the wrong stack, unresolved
symbol, pre-call breakpoint, empty checkpoint, timeout or missing reached witness
is unqualified. An identical release-through control must finish normally.

`/usr/bin/gdb` exists by read-only path lookup; its version, usable native symbols,
ptrace permission and source-location fidelity are unverified. No debugger or
fault ran. Q1 neither promises a viable control nor authorizes a new fault venue.
If its preconditions cannot be demonstrated under the permitted owned-child
model, leave G100-1 open for specific disposition. Do not substitute a timed kill,
broader host fault or a new test seam whose absence is called product RED.

## G100-2: richer assertions and strict TDD

The proposed small composition would combine the existing85 rich success fixture
and independent survivor-only/history oracles with selected existing transaction
exits, adding direct target span joins and exact pre-retry phase/root/queue checks.
Positive population must establish all four FTS families, retained target and
survivor revisions, dependency rows, non-head vector mapping/span and original
sink ID before fault. Final raw scans occur before close; permitted audit/closure
carve-outs stay. Existing proof validators and expected results are not reused
as the sole independent oracle. Do not multiply every family across every hook.

This would likely be passing-on-arrival coverage, not a proven behavior repair.
The current implementation plan therefore **defers** new G100-2 tests. Q1 and any
new passing qualification harness are likewise policy-held. Concrete choices
for the parent to present if qualification remains mandatory are:

- Explicitly allow only the reviewed G100-1/G100-2 passing coverage additions,
  with honest coverage-only commits and independent negative/healthy controls;
  real defect repairs still require observed per-finding RED→separate GREEN.
- Keep strict discipline unchanged, add no passing coverage or artificial source
  change, and explicitly defer/dispose those unqualified cells while retaining
  the already-covered transaction proof and its precise limits.

Neither alternative is recorded as an owner decision. Slice20's unanswered
race23/T2 question cannot choose for100. Missing-hook compilation or a synthetic
mutant failure is not evidence that the actual baseline violates the contract.
The default under unchanged rules is no new passing-on-arrival test/source work.

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

This resolves the old pending10 dependency. Current clean parent/release2f6913c
includes independently reviewed20 Linux subset657edeb. Its delta owns evidence
rendezvous only; erasure/crash/checkpoint inputs remain unchanged from02e30d.
Recorded actual10 design recheck passed; fresh independent review of this20
evidence update remains. Future source entry still requires parent's fresh
worktree/GO, closed10 preflight, then-actual-base reread and own setup/resources.
No setup, native tests or faults are authorized by this planning update.

Actual20 canonical source-bound execution supplies usable existing-control
inference from the unfiltered workspace feature closure and matching quiet Rust
count/child blocks, including all eight85 crashes. Entire src/lock match collector
6eee→final657→current2f691. Quiet output omits names/PIDs/Rust binary digests;
retain those provenance limits and final independent review. This supports
G100-3 reuse without a duplicate campaign or new hard20 dependency. Collector
212pass/2fail plus affected corrections remains assembled qualification. No20
result fills G100-1/G100-2 observations absent from the unchanged assertions.
Later40/50/70/110/90 engine changes require explicit affected-receipt invalidation;
160 owns final composed qualification. No unconditional all-release rerun rule.
