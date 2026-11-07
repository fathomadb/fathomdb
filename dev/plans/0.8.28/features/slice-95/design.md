---
title: Slice 95 — bounded CI repair and candidate qualification design
status: APPROVED
target_release: 0.8.28
---

# Slice 95 design

## Execution reconciliation

Execution baseline: `6ae55c2e42e5aa447bc9f13265f6bf5a9e97f6e2`.
The [improved plan](plan.md) remains the scope and S95-1–13 acceptance authority.
Changes since its reconciliation baseline `48b23c479` are:

1. Slice 92 completed documentation convergence, all 27 maintained-owner audits,
   pending .28 public-doc truth, source-bound format-36 checking and actual CPU
   consumer examples. Its [status](../slice-92/status.md) bounds reuse; no final
   platform/optimized-artifact qualification was supplied.
2. Three existing plan/board renderers were registered; 13 generated regions
   now agree. Preserve their ownership rather than hand-editing current pointers.
3. The full Slice 92 collection ran all 205 suites: 196 initially passed and
   nine recovered by narrow unchanged rechecks. Prose/state-view corrections and
   disk cleanup are accounted for; there is no uninterrupted full-gate claim.
4. Refreshed gh evidence on 2026-11-04 leaves run 41885604931 at older
   `a2fb4a647`: four native legs failed, Windows native queued, fast/heavy/race
   failed and separate Windows WAL succeeded. None qualifies this candidate.
5. Source review confirms CI1/3/4/6/7, CI5 is fixture isolation, and CI2 is a
   resource failure rather than evidence of a product defect. No runtime repair
   is allocated. The seven repairs below remain needed.
6. The actual host supports live namespaces/ptrace and two RTX 3090s outside
   the sandbox. The checkout has its own pinned venv/Node tools and about 11 GB
   free, but feature builds change profile settings and need a storage budget.
   EU7 corpus is absent here although available in the primary checkout; stage
   byte-verified disposable inputs before timing. Do not borrow another venv.
7. Immutable nonpublishing release dispatch already exempts main ancestry in
   `verify-release-gates.sh`. Version/Axis E and trusted workflow prerequisites
   remain; the pending .28 docs and unchanged .27 manifests are distinct.
   No version cut, main integration or publication is authorized here.

Accept the plan with these source-grounded adjustments. Retain owner deferrals,
ordinary regression coverage and Slice 92's documentation baseline. No new
policy, scheduler, general validator or erasure campaign is needed.

Astra design review APPROVE on 2026-11-04: the bounded repairs, focused tests
and qualification/publication limits are accepted. Resource settings remain
provisional until hosted measurement; spill isolation is invocation-owned.

## Repair design and RED/GREEN

Each behavioral repair gets an observed, staged/committed RED before GREEN.
Keep substantive test assertions fixed during implementation. Explicitly replace
old fixture assumptions that contradict the reviewed intended contract.

| Repair | Smallest intended change | Decisive tests |
| --- | --- | --- |
| CI1 cold setup | Extract canonical pinned Actionlint installation from full bootstrap for reuse. Heavy setup uses Python dev extra, root/TS locked npm installs and existing ShellCheck installer. Provision cheap prerequisites before native compilation; avoid full bootstrap's unrelated Cargo/OS tools. Keep verifier preflight strict. | Extend existing heavy-bootstrap fixture for actual prerequisites, cold/cache-restored execution and correct paths; preserve missing/wrong-tool controls and installer failure propagation. Replace the obsolete rule forbidding needed developer tools explicitly. |
| CI2 parallel resources | Scope test debug=0, incremental=0 and Cargo build jobs=2 to diagnostic job; preserve libtest concurrency. Disable diagnostic target-cache restore/save and cache saving with the pinned action's supported options, retaining dependency restore. Record free bytes/memory/effective settings and target/scratch sizes before/after build/test/upload/post-processing; do not delete runner/shared tool installations. | Existing efficiency/race runner controls prove configuration and environment/arguments with fake Cargo. Hosted run must reach tests and supply real exit/count/log and resource transitions. Chosen settings are provisional until measured. Advisory wrapper reports Cargo failure separately from upload/cache failure. |
| CI3 package exports | Replace inline declaration regexes with one strict installed-package TS consumer importing GraphEvidenceResolveRequestV1 and calling the public method with meaningful typed input. Compile in isolated consumer without source path mappings; use the same proof on Unix/Windows, then retain native calls. | Real modular package must compile; disposable installed package missing only its public export must fail while internal declaration and checkout remain. Retain package/declaration hashes separately from wheel/addon receipts. No source compilation substitutes for this proof. |
| CI4 Cargo color | Pass --color never at cargo-tree command boundary, retaining exact dependency/version/source and forbidden-feature parsing. | CARGO_TERM_COLOR=always legitimate fixture must pass; all wrong package/version/source/feature controls remain decisive. |
| CI5 ShellCheck fixtures | Restrict fixture tool lookup/HOME/cache instead of changing the correct production resolver. Force installer/cache branches independently of host shellcheck. | Wrong/missing binaries fail for named reasons; pinned HOME/wrong PATH and wrong HOME/pinned PATH pass as intended; cache hit/miss/corruption controls cannot be rescued by the host. |
| CI6 real safe diagnostics | Run actual verifier preflight through existing capped-output capture. Add optional invocation-owned spill directory shared by CI producer/collector, preserving local /tmp default. Exclude empty/symlink and foreign-directory logs. Keep primary verify failure distinct from secondary collection/upload errors. | Missing-tool preflight exits nonzero with genuine nonempty redacted evidence; long failure keeps full diagnostics and exit/count/time. Empty/symlink/foreign-only inputs cannot become proof. Existing collect-all and default local behavior stay intact. |
| CI7 manual routing | Validate lowercase full candidate SHA, release ref and equality to dispatched SHA early in hosted changes, before checkout/classification. Expose validated qualification output. Successful manual qualification bypasses diff/lite selection for all eight drivers; manual mode is normal. Retain native precheckout guard and cancellation semantics. | Existing pinned-matcher routing test covers empty-diff/already-merged/manual-lite, malformed/mismatched SHA/nonrelease ref, classifier failure after valid input and cancellation. Invalid manual validation cannot launch fast/heavy qualification; genuine classifier failure retains fast diagnostics. Automatic docs/platform/trusted-lite behavior stays proportional. |

Astra approved the bounded cache treatment (`cache-targets: false`,
`save-if: false`) and temporary Windows executor provisioning. Disabled cache
post-processing is a policy fact, not a measured callback. The
[qualification matrix](qualification.md) enumerates current obligations and the
accepted TC5 AC075 / separate AC073 stress / legacy EU7 diagnostic distinction.
Astra reviewed that applicability matrix and approved it without scope changes.

## Qualification and evidence

After Astra approves implementation and focused checks, freeze executable/source
inputs and commit. Run one broad local verifier and feature-complete round with
live prerequisites, named scratch and explicit resource budget; on failures run
only affected scopes after repair. Parent owns shared release-state edits and
remote gh dispatch; implementation is isolated in one disposable worktree.

Use the existing feature/target matrix and reviewed allowlists, strict live
security, accepted long/performance/EU7 routes, five native-target receipts plus
distinct Windows WAL proof, source-archive Rust consumer and no-private-hook
shipped artifacts. Record real outcomes, unavailable/queued blockers and exact
inputs. Time performance only after competing builds stop. Source Gitleaks and
retained generated-evidence scans are separate, pinned and advisory as before.

Local dry-run and immutable release-workflow dry-run get separate dispositions.
If .28 version/Axis E/provenance preparation remains unauthorized, record that
prerequisite and pending rehearsal; do not substitute an old-version rehearsal.
No tag, registry/Pages publication or publish workflow with dry_run=false occurs.

The final [status](status.md) maps S95-1–13 to commands, artifacts, source/input
identities, reviews and actual results. Report assembled/resumed evidence and
readiness blockers honestly. Forward final verification fixes under the existing
.29 hand-over rule, then clean up only the task implementation worktree/branch.

## CI8 amendment: confirmed-debt worker admission

**Astra amendment review APPROVE on 2026-11-04.** Execution evidence adds one
bounded correctness repair to the seven CI harness changes. Manual candidate
`fb76c8ff936996a69e2b30d03145d7ba1bce5efd`, run41956270895, reached actual heavy
Rust tests after successful cold preflight. Its serial test-rust failed both
source-erasure drain-order cases and the multi-page actuation-receipt erasure
case with physical_admission/service-already-occupied. The parallel diagnostic
also encountered that refusal. These are engine-owned worker races, not shared
fixture databases or process-global service locks.

The first local collection ran205 suites:196 passed,9 failed. The six disk
preflight families recovered with real headroom; the stale pin-owner assertion
and two generated-page checks recovered at06a286876 with focused checks. None
of those corrections explains away the independent hosted Rust failure.
Astra source/design and adversarial contract reviews identify a production
ordering defect; preserve all three original tests unchanged.

The accepted [resumable-cleanup ADR](../../../../adr/ADR-0.8.28-resumable-physical-cleanup.md)
services committed debt and preserves priority against ordinary writers. Slice85
retains Autonomous behavior. The current worker interprets a contended primary
as possible debt, then reserves physical_service_gate before confirming debt.
That speculative occupancy can refuse an otherwise uncontended fresh erase or
excise before its drain-before-freeze path is admitted.

Smallest correction, with no public API/policy/SLA change:

1. Preserve the initial probe and priority/turn acquisition, including when the
   primary is contended. Do not revive starvation by treating contention as no
   work or changing the existing priority control.
2. Acquire the authoritative primary connection and confirm active native debt
   before attempting to reserve the physical service gate.
3. Attempt service acquisition nonblockingly while primary is held. A foreground
   erasure may already own service and wait for primary; on contention promptly
   release primary/turn/priority and yield. Never wait for service under primary.
4. Once acquired, retain the service guard across sanitation, reader drain,
   finalization and blocker recording. Preserve storage/fence/error behavior;
   uncertainty is not successful cleanup. Genuine competing cleanup still
   produces the existing typed Busy/Incomplete outcomes.
5. Keep foreground paths, explicit maintenance, public errors, lock ownership,
   physical proof and shutdown unchanged. Update only the maintained worker
   ordering explanation alongside the implementation.

RED is deterministic real-database coverage: hold a debt-free Engine's primary,
wait with a bounded watchdog for the existing priority atomic, and prove the
worker has not occupied service. Use the existing instance pre-primary erasure
hook plus channels to observe actual foreground admission before releasing
primary and allowing successful erasure. The generic faultpoint is thread-local
and must not be armed on a different thread to pretend to control the worker.
Release and join all actors before assertions so RED cannot hang. Add/retain a
real committed-debt completion/priority control and genuine service-owner refusal.
The existing contended-primary priority test and original three failures remain
unchanged. Stage/commit observed RED, then a separate minimal GREEN.

Scoped validation covers the new tests, all physical_maintenance unit controls,
unchanged erasure_drain_ordering and slice25_fix1_red, idle debt completion,
Autonomous failure/fence/close and explicit admission regressions. Because this
new production phase changes core Autonomous coordination, reassess its blast
radius, regenerate existing docs, and run one final clean broad local/feature
round after it stabilizes. Earlier CI harness evidence stays separately useful;
runtime/artifact evidence must be refreshed for the corrected source. The final
manual CI run must bind the exact reviewed candidate; do not relabel earlier
platform receipts or substitute delta-only PR CI without owner authority.

## CI9: foreground rollback property owns its actor

Candidate run41964943428 at16234813c completed the resource-bounded parallel
report without disk exhaustion but actual Cargo exited101. Its one failed
property observed sanitation family1 after a foreground injected rollback,
where it asserted family0. This is a fixture isolation defect: the thread-local
one-shot hook faults the foreground transaction only. The accepted Autonomous
worker may acquire the released service and advance committed debt before the
property reacquires primary to inspect it. No atomicity or survivor-loss defect
is shown by that observation.

After the property's real seed write, stop/join the existing per-Engine worker
before arming foreground faults. This follows the existing synchronous-blocker
fixture. Preserve all24 cases, fault/outcome/root/family/survivor checks and final
retry. No production code, hook, policy, scheduler, assertion or skip change.
Separate idle-debt, priority and genuine Autonomous contention controls remain
active. Astra approved the actor-isolation design; code approval follows review of
the exact fixture delta.

The original hosted failure is observed RED. Run the corrected property and
whole engine unit library with normal parallel test threading, plus maintenance,
idle-debt and genuine-service controls as GREEN. Review the exact fixture delta.
The completed canonical local run retains its original SHA/outcome; only affected
scopes need local rechecks for this fixture-only correction. Run exact workspace
all-targets check on the final commit. Dispatch fresh full manual CI on that
commit: prior162 platform receipts remain attributed to162, never relabeled.
No evidence-reuse waiver or new gate is introduced. Publication remains held.

## CI10: disposable landing fixtures declare their resource budget

Run41964943428's independent transcript job passed the real hygiene gate and
all but one fixture assertion. Its tiny clean linked-repository positive control
ran default preflight on an8GiB-free executor and failed the unrelated10GiB
workspace-build floor. The first local collection's six ambient-disk fixture
failures are the same blast radius, not additional product failures.

Astra audited all35 preflight calls in the six existing landing, board-currency,
ledger, governed-surface, C1 and transcript fixture helpers. They use disposable
repositories and none tests the default disk floor. Give only these helpers an
explicit1GiB fixture budget via the existing --min-disk-gb option, before caller
arguments so explicit overrides retain precedence. Preserve every assertion,
positive/negative/fail-open control and explicit flag-composition arm. The real
production default and disk predicate remain10GiB and byte-identical.

Observe all six original suites under a fixture-owned8GiB df environment as RED;
after the six localized invocation corrections, run them unchanged under the
same condition as GREEN. Separately require genuine default preflight without
an override to reject8GiB with its named HARD10GiB requirement. No runner cleanup,
new framework, fake database, retry, skip or waiver is introduced. Astra approved
the design and audited scope; code approval follows the exact delta and results.
Include the fixture changes in fresh final exact-candidate CI. Publication held.

## CI11: feature inventory and complete compiler diagnostics

**Astra written amendment review APPROVE on 2026-11-05.**

The first full feature-complete invocation at5f668f330 ran164 tests successfully
with zero test failures,16 ignored and nine findings. Seven configurations
could not compile the same ignored slice50_fresh_open_cost target: it still
accessed removed OpenReport.dense_disabled. One real BGE UTF8 extent test lacked
a valid ignore disposition, and one crash-child feature disposition was stale.
Keep the original exit1 and summary; this is not a passing feature round.

Apply only the accepted contract/inventory corrections. Successful open().unwrap()
already proves admission under seq359, so remove only the obsolete report-field
assertion. Preserve every provider-count, readiness, fingerprint and timing
assertion and the existing opt-in timing ignore. Classify that newly discoverable
single timing test explicitly under the accepted populated-open timing deferral;
do not execute or reopen the deferred experiment. Remove the UTF8 test's ignore
and execute its unchanged body against the real pinned BGE cache. Remove only
the stale feature-only crash-child allowlist row; baseline parent/child behavior
and its explicit child ignore remain unchanged.

The existing listing helper discarded Cargo's JSON stdout, including the full
compiler diagnostic. Preserve complete stdout and stderr in distinct labeled
sections without claiming chronological interleaving; keep parsing, failed-target
accounting and exit semantics unchanged. Add observed RED/GREEN in the existing
test-target unit framework: diagnostic present only on stdout, distinct stderr,
actual listing helper, failure accounting and successful-listing control.

Astra reviews this amendment and the exact implementation. Re-list all seven
affected configurations and account for every newly visible test, including the
explicit deferred timing test. Reuse the164 unaffected passes only after an
explicit relevant-input comparison; run the now-live UTF8 test and existing
feature-runner/coverage validators. Report assembled qualification with the
original failure retained. Refresh exact-candidate manual CI after source and
executable inputs stabilize; forward the corrections into .29 before its final
gate. No production API, threshold, retry, broad exclusion or new framework is
introduced. Publication remains held.

## CI12: preserve subscriber admission below capacity

**Astra concrete design review APPROVE on 2026-11-05.** Exact candidate
bef17bfab run41983987975 passed fast198/198 and every other applicable job,
but heavy6/7 failed Python:1failed,1708passed,39skipped. The unchanged real
write test observed phases [None, finished], missing Started. Preserve its raw
failure and assertions. The first payload was not printed, so attributing None
to a loss warning remains a source-supported explanation, not a captured fact.

The accepted Python subscriber ADR guarantees exact delivery/pairing below
capacity with a live functioning logger. Queue::push instead drops on any
bookkeeping try_lock contention, even when empty. Repair this production
admission defect; do not call ordinary contention overload or relax the test.

Use std::sync::mpsc::sync_channel with the existing4096 capacity and a private
record/Stop envelope. Producers check enabled and try_send without the consumer's
bookkeeping mutex, Python invocation or I/O. Only actual Full counts as overflow;
Disconnected ends delivery. Keep a saturating atomic loss counter. The receiver
and warning/normal-record fairness state remain under consumer bookkeeping.
Avoid prefetch storage that would add another queued record beyond capacity.
Restore deferred loss counts with saturating atomic addition, never overwrite
concurrent losses. Stop is private control traffic and never a diagnostic/loss.

Disable publishes enabled=false before sending Stop, then clears receiver state.
An empty waiting receiver wakes; a full receiver is already ready. Check disabled
after wake and suppress queued delivery. Disconnect or bookkeeping poison ends
delivery consistently without panicking into database callers or retaining an
enabled dead consumer. Never hold bookkeeping across Python. Preserve weak
ownership, profile/error payloads, handler fault/reentry containment, replacement,
the single retiring-worker bound and the already-dequeued callback allowance.
No dependency, public API, lifecycle phase or accepted policy changes.

RED holds the existing bookkeeping mutex with capacity available, pushes Started
and Finished on another actor, and requires producer completion while held.
Release/join before assertions. Current code reports Dropped(2); GREEN delivers
the two FIFO phases without loss. Preserve overflow/fairness/disable oracle
intent and prove disable wakes an idle receiver without waiting its one-second
timeout. Run unchanged real-DB Python subscriber lifecycle/reentry/replacement
coverage against a fresh installed native wheel. Stage/commit RED before GREEN;
Astra reviews exact code and a separate verifier checks outcomes.

The blast radius is Python native subscriber delivery. Refresh its native unit,
installed-wheel/Python runtime and affected feature evidence, then exact-candidate
CI. Explicitly compare unaffected core/Node/measurement inputs before reuse;
do not repeat a cold full local/feature campaign for this module repair. The
already-running .29 gate keeps its original source/outcome; forward the fix
and recheck affected scopes afterward. Correct the maintained binding design's
stale full-or-contended loss claim and state the accepted contract in Python
reference/release guidance. Publication remains held.

## CI13: observe the provider deadline rather than teardown wall time

**Astra design and code review APPROVE on 2026-11-05.** SourceE86fb2924
run41991174061 passed fast198/198, Python and every other applicable job, but
heavy Rust failed one of144 engine units. The close fixture reported207.540814ms
against its200ms bound. Its timestamp precedes releasing the primary mutex,
remaining SQLite disposal, scheduling and channel delivery. That interval does
not measure the configured40ms provider budget. Source still drains SQLite
before joining providers and stores the first deadline with get_or_insert.

Preserve the existing production deadline and Scheduler/worker/owner contracts.
Use private cfg(test) observations of the actual post-SQLite boundary, zero
managed owners, proposed clock/budget, stored deadline and timeout decisions.
No deadline may exist while the real primary transaction is held. First,
concurrent, repeated and Engine Drop attempts select the identical deadline;
already-expired attempts request no additional timed wait. The observer survives
Engine Drop without retaining the Engine. Release the held provider explicitly
after close/Drop observations, with cleanup guards/watchdogs that join actors
on failure. Replace proxy wall-time assertions and the fixed300ms release;
do not raise200ms or change production scheduling/deadline calculation.

Retain the hosted failure as observed RED. The replacement oracle must reject
isolated early-start and renewal mutations; restore source before GREEN.
Focused close/failed-open/provider-retention controls, the normal-parallel engine
unit library and standalone dispatcher controls verify the bounded blast radius.
No full local restart; final exact-candidate CI remains required. S95-6/8 govern
this fixture correction; no new product policy, deadline or API is introduced.

## CI14: witness concurrent progress in the long cursor fixture

**Astra design review APPROVE on 2026-11-05.** Independent review found that the
existing1000-search fixture ignores writer errors and can pass with entirely
empty searches. The earlier execution proves the search path ran, not positive
concurrent-write evidence. Its comment about a compiled-SQL row is stale.

Strengthen this existing test only. Require a successful real commit before the
reads and at least one successful commit during the read phase through explicit
rendezvous. Propagate writer errors and record positive matching-row observations.
Use explicit provider-free open, one logical ID and distinct matching revision
bodies so each iteration can observe advancing committed data. Identical-body
reuse otherwise makes the old row-count check inert after the first barrier.
In this fresh synchronous lexical-only fixture, additionally require each
matching hit.write_cursor <= response.projection_cursor, and observe a hit cursor
beyond the first real successful WriteReceipt cursor. This is not a general
promise for stores with asynchronous dense-projection backlog.

Keep1000 searches, the50us writer throttle,60-second anti-hang watchdog and
original returned-row-count <= projection_cursor invariant. Stop/join the writer
on all failure paths without a second panic during unwinding. Remove the stale
comment; no new framework, product API or timing SLA.

Demonstrate the original fixture's vacuous pass under a disabled/invalid writer,
then the strengthened fixture's rejection under the same mutation. Also freeze
later response cursors at the first real receipt: the advancing-hit oracle must
reject that stale-cursor mutation even when the row-count check cannot. Restore
the real writer and run the actual long body plus focused cursor controls. Report
safe progress/advancing-hit counts. The row-count comparison remains a lower
bound; the additional hit-cursor witness is scoped to this provider-free fixture.
S95-6/9 require meaningful concurrent evidence; no unrelated local regression
restart is needed. Astra approved this refinement after rejecting the weaker
positive-activity-only version.

## CI15: classify the registered TC5 fidelity receipt

**Astra design review APPROVE on 2026-11-05.** Use the existing classification
schema/API and closed amendment reasons. The immutable record's missing plan and
untyped external summary locators each receive one exact-run/record-hash-bound
amendment. The retrospective plan and sidecar bind eight scalar metrics and
explicitly exclude 30 identity/control leaves; preserve original record, config,
metrics and index bytes. No schema, validator or historical policy is broadened.

Classify actual private `tc5_vector_stage` storage, corpus/gold and deterministic
recall/bootstrap work as data-plane components. The public-operation taxonomy has
no label for this private path; empty call paths/witnesses are truthful. Bind the
measured `5f668f330` source and original artifacts directly, rather than invent a
public `Engine.search` or external-retrieval witness. The companion derivation
states retrospective timing and fidelity-only limits, including the changed
post-run database hash of unknown cause. Normal API idempotence, portable tree
validation, existing 38 tests and independent immutable-byte/hash checks verify
this correction after the observed lint RED. AC011 and publication holds remain.

## CI16: bound the write-throughput diagnosis

**Astra diagnostic design review APPROVE on 2026-11-05.** The owner authorized
focused diagnosis and repair with the original thresholds retained. No arithmetic
artifact is established: even dividing 59,986 commits by exactly 60 seconds falls
below 1,000/s. The final complete commit legitimately extends elapsed time.

Use at most two diagnostic runs before reassessing the design. Phase 1 adds only
temporary aggregate timers around the synchronous write phases, using existing
physical metrics after the unchanged warmup and the same AC011a body. Capture
original elapsed/rate before metric serialization. Only successful foreground
per-write phase counts should equal measured commits; worker/shared stages and
nested durations are separate observations. Actual tx.commit belongs inside its
commit timer. Preserve guards, lock/error/RAII lifetimes and worker activity.
Instrumentation overhead is unknown and cannot establish a saving of 0.0348%.

Phase 2, if needed, isolates the dominant phase's actual SQL preparation,
repreparation and execution. Existing trigger toggles can expire prepared
statements, so cached preparation is not automatically a reuse improvement.
The existing row-trigger fallback is not an approved shortcut: even a single
Node write can make several mutations, whereas the accepted managed transaction
performs one checked nonce-bearing visibility advance.

**Astra repair design APPROVE on 2026-11-07.** The second bounded diagnostic
confirmed 1,000 initial endpoint-statement preparations and zero executions for
1,000 successful anonymous Node writes. This proves unused work in that sample;
it does not establish production speedup or the cause of the historical miss.
The retained probe and its rerun instructions live in
`dev/experiments/ac011-endpoint-prepare/`; raw output remains ignored.

Restore uninstrumented source, then mechanically extract the existing
endpoint-count block at its unchanged post-batch/pre-retention call site. A real
SQLite authorizer scoped to helper calls denies canonical-node reads: Node-only
counting must still return zero, while an Edge control must propagate denial and
count two missing endpoints after denial is removed. A real Engine write receipt
and lexical-search control, plus existing edge/mixed/supersession checks, preserve
observable behavior. Observe and retain the failing Node-only test before GREEN.

The repair prepares the same statement lazily on the first effective Edge after
the existing in-batch supersession skip and reuses it for that helper call. Keep
final post-batch endpoint visibility, logical-ID supersession, independent endpoint
checks and saturation unchanged. No transaction, trigger, nonce, cursor, worker,
guard or durability change. Thresholds and the canonical qualifier stay unchanged.
