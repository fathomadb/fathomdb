# Slice 100 bounded evidence-reconciliation design

## Obligation and disposition map

This proposal distinguishes an observed assertion gap from an accepted new gate.
The owner closed physical-erasure feature/optimization effort except defects;
existing guarantees and regression protections remain. No new fault, debugger
alternative, harness, test or source work is selected.

| Authority / obligation | Evidence discharged within its recorded limits | Concrete residual |
| --- | --- | --- |
| Original [D28-09](../../../0.8.28-draft-scope.md): interruption inside a transaction **or** WAL checkpoint. | Eight85 exits include before-delete inside IMMEDIATE;82 also interrupts real sanitation writes before COMMIT. Bounded actual20 Astra PASS accepted historical execution inference. Fresh v1 design PASS accepts the disjunctive interpretation. | G100-1 active-native-checkpoint interruption remains unproven and optional. No both-boundaries mandate or debugger control is selected. |
| D28-09 named real-DB interruption, truthful reopened state and repeatable recovery. |85 requires eight exit73 outcomes under5s watchdog, target/survivor visibility, ticket logical state, after-completion Complete and retry-to-Complete.82 adds raw canonical/debt/rollback/continuation and original-sink replay. | Quiet retained execution lacks named phase/PID/binary-digest trace. Exact phase/root scrub at every hook is not an established additional gate. |
| Retained R29-3P-16/O3-16 survivor/history post-state. | Existing interrupted tests check survivor presence/count;82/85 non-crash controls independently preserve full history. | **G100-2H:** unchanged survivor historical versions after interruption are not directly asserted. Separate non-crash history proof does not discharge it. |
| Retained O3-16 projection/span post-state. | Direct span cleanup and rich projection/vector success controls exist separately from actual crash tests. | **G100-2P:** direct projection/span state after crash/reopen is not asserted. Logical target absence is insufficient. |
| Retained O3-16 physical-erasure post-state. | Raw main/live-WAL success witness exists;83 post-crash main scan runs after close;85 retries to Complete with logical absence. | **G100-2W:** independent main-plus-live-WAL absence after post-crash completion is not asserted before final close. A Complete enum does not discharge it. |
| Accepted maintenance/format37 and defects-only closure. | Current source retains atomic authority/checkpoint-before-receipt ordering; no demonstrated violation. | These three retained evidence predicates remain unmet, not product defects. Every-family/every-hook or all-in-one conjunction is not an established gate; optional extra assertions cannot obscure the three concrete gaps. |

Proposed current outcome remains documentation-only reconciliation. Whole100/
D28-09 stays unclosed because G100-2H/2P/2W remain unmet; labeling a larger
conjunction optional does not dispose of them. The [minimal coverage-only
proposal](test-plan.md#minimal-coverage-only-proposal) uses two existing transaction
boundaries plus a healthy control to address exactly these predicates. It likely
passes on correct baseline code, so it requires a narrow exception to the current
passing-on-arrival coverage prohibition before test implementation. No exception
is granted/requested from the user here; parent first obtains exact revised-design
and fresh final documentary reviews, then presents any concrete policy choice.
A policy-approved deferral would also need explicit disposition; none is recorded.
Real defects still require observed per-finding RED and separate GREEN.

## Practical architecture at the qualified current parent

At `1099978cd7d7aef6e745cfc850ddb8f9b57050d3`, queue acceptance uses an owned
IMMEDIATE transaction with clean visibility admission; its before-commit hook
follows durable request writes. `activate_in` stores Executing/Removed inside
the deletion transaction. `purge_inner`/`excise_source_inner` atomically remove
canonical and registered projections, enqueue original redaction obligations,
measure closure and install RootDebt before their before-delete hook and COMMIT.
A process exit there tests real rollback of FathomDB composition.

`physical_erasure::transaction` owns BEGIN IMMEDIATE, clean-state admission,
COMMIT and explicit rollback after operation or COMMIT failure. `sanitize_unit`
performs the real FTS merge and progress store in that transaction before its
faultpoint. `prepare_redacted` clears selector/scope and root retry/cursor/vector/
denial material atomically. `finalize` joins root/closure retirement and terminal
receipt publication in one transaction; its caller first requires actual
successful TRUNCATE. A before-completion crash is after checkpoint, never inside it.

Foreground finalization, the autonomous worker and offline recovery share these
authorities while using distinct checkpoint callers. Online and worker paths
suppress SQLite's busy handler during TRUNCATE and retain durable incomplete
state on refusal. Offline completion validates current authority and finishes
activated debt only; it never activates Queued. Explicit-mode reopen validates
persisted policy/debt without starting the autonomous worker. Autonomous open
starts its Engine-owned worker; the independent observer control waits for debt
retirement without follow-up Engine calls. The worker and retry have distinct
proofs; disabling the worker cannot establish autonomous progress.

Format37 admission requires the clean coalescing marker. Erasure deletion and
standalone sanitation/finalization do not arm `VisibilityCoalescingGuard`;
ordinary visibility triggers and authenticated vector-span mapping DELETE
cleanup remain active. Registry row erasure also removes orphan spans directly.
No alternate erasure cache, suppression flag, schema or migration is needed.

The maintained [engine](../../../../design/engine.md),
[production design](../../../../design/explicit-maintenance-erasure/production-design.md)
and accepted ADRs agree with these mechanisms. Their checkpoint-boundary language
is consistent with pre/post-checkpoint crash controls; it does not establish
termination inside native frame copying. No global architecture contradiction
was found, so this local attribution is the maintained change.

## Current-parent delta and evidence validity

The qualified ancestry is10 implementation104dcbc/closure d7d928,20 Linux subset
657edeb,60 formats/direct-filter/budget5271c1b,70 COMPLETEe875f360 and30 P runtime
2fd7725/final documentation0d8be5b. Original failed collectors, affected resumptions
and skips remain assembled evidence; whole20 A3/A5+WindowsA6,60 F11 and30 G/D/M/F
remain open. Isolated30 G/D49be773 is not integrated and supplies no100 evidence.

The source/lock equality formerly stated at2f6913c is historical. Since that
parent, engine production changes are `embedding.rs`, `hints.rs` and
`search_types.rs`: absent/timeout degradation, Closing propagation and supported
remediation; the engine manifest adds the70 test target.30 adds an SDK measurement
example/dependencies and two registry entries;60 changes MCP formats/filter/budget
and bindings. Cargo.lock changes SDK dependency references, not the pinned
rusqlite0.41.1/libsqlite3-sys0.39.1 versions. These facts are selective attribution,
not a claim of an identical whole binary or all transitive inputs.

The task receipt `receipts/source-census.json` pins22 named erasure/recovery/test
and Rust-caller files byte-identical to clean20 collector6eee2e9 and current HEAD.
It includes all nine integration test files in S1–S8/S9, core erasure/queue/
maintenance/visibility/registry/WAL modules and canonical Rust caller. Existing
search/reader hook ownership changed in20; that prior bounded review accepted
its execution inference.70 query reporting changes can affect rich route
observations, so C12 remains evidence of the named historical control rather
than fresh100 current-native execution. No whole-src/lock equality is claimed.

The retained20 Rust log is65288bytes/2390lines, SHA256
`aacbdb427f6a809d34ef046fe38ebebf5b8e01256f21e32a2d5e338d53bbd2a2`.
Its1611–1629 block contains2tests, eight child launches,1pass/1ignored/0fail.
Canonical serial workspace command has no selector/exclusion; CLI operator plus
engine self-dev test-hooks select85. This matches unchanged85's parent/child
structure. It is command/source/order inference, already boundedly reviewed;
quiet output omits phase names/PIDs/Rust binary digests. The collector remains
212pass/2fail plus reviewed affected corrections, not uninterrupted green.
No rerun is commissioned to improve those receipts in this documentation task.

## Fault model, proof limits and repair boundary

Process exit73 is controlled process-crash evidence, not OS/power/torn-write proof.
G100-1 has no witness inside active native checkpoint frame-copy/TRUNCATE work.
BUSY, before/after-call observation and checkpoint-to-receipt crash cannot fill
that cell. No debugger/VFS/timed-kill control is selected or required by this
candidate's interpretation of the original disjunction.

G100-2H/2P/2W are retained unproven predicates, not demonstrated missing erasure
logic. Separate success/refusal proof does not fill their interrupted-history,
post-crash projection/span or post-retry live-WAL observations. An exhaustive
combined matrix is optional. Existing82/83 child waits have no parent watchdog;
85 has5s. Keep that harness limit and historical failures/skips explicit.

If later accepted-invariant failure is actually demonstrated, minimize it at
that actual baseline, commit genuine RED with observed assertion/body evidence,
then separate minimal GREEN preserving the oracle. Passing-on-arrival tests,
mutants and absent-hook compile failures cannot supply product RED. A source
repair requires its own review, resources and affected explicit/autonomous/offline
checks; no such implementation is authorized or necessary for this proposal.
