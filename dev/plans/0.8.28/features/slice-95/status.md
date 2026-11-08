---
title: FathomDB 0.8.28 Slice 95 — execution status
status: IN_PROGRESS
---

# Slice 95 status

**IN_PROGRESS. Publication held.** Latest clean candidate
`10ca015034f6c55c5c2193a75e1eb4056d5bb948` measured **932.474344 commits/s** against
the unchanged **1,000 minimum** (55,950 commits over 60.001650826 seconds).
The canonical helper exited 101 before AC011b. Separate verification confirms
source/binary/profile identity and applicable before/after/enclosing readiness.
Previous valid non-passes were 999.651557/s and 827.267649/s; none is waived.

Two narrow repairs are implemented with observed RED, frozen GREEN, Astra design
and code approval and separate verification: lazy unused-endpoint preparation and
compiled open-state statement reuse with fresh reads. Both are integrated into
.28 and forwarded to .29. Their correctness is established; passing throughput
is not. After these two repairs, stop further implementation or measurement for
an explicit reviewed reassessment. Do not infer a causal speedup from separate
qualification runs or weaken durability/thresholds. Final local code-phase gate
and fresh exact-candidate hosted qualification remain pending.

The [plan](plan.md), [design](design.md), [qualification matrix](qualification.md)
and [source-bound evidence summary](qualification-evidence.json) record the
scope, acceptance, identities and limitations. Slice 92's completed documentation
baseline is consumed; later repair deltas have scoped documentation checks.

## Implemented and reviewed

Astra approved design and code reviews for CI1–15. Observed RED/GREEN covers
cold prerequisite setup, bounded parallel builds/cache behavior, installed
modular TypeScript contracts, deterministic dependency/tool fixtures, retained
failure diagnostics and manual qualification routing. Genuine hosted failures
also exposed and repaired confirmed-debt service admission (CI8) and
below-capacity Python subscriber delivery (CI12). Existing product thresholds,
public APIs, queue capacity and actual saturation controls remain intact.

CI9/10 isolate foreground actors and disposable-repository disk fixtures.
CI11 corrects stale feature expectations and retains complete compiler output.
CI13 verifies actual provider-close deadline decisions; early-start and renewal
mutations fail. CI14 proves nonempty concurrent lexical reads with successful
advancing writes, propagates writer errors and rejects a stale-response-cursor
mutation. These last two repairs change test instrumentation/fixtures only.

Separate verification confirms final code `3a05a45cb`: 10 close/provider
controls, 1,000 cursor searches with 239 writes/238 during reads/999 advancing
hits/zero violations, target accounting (356 targets/36 feature-only), generated
contracts and 10 unchanged Python subscriber tests using a freshly verified
native module. Earlier deterministic tests and mutation evidence remain retained.

## Qualification and remaining requirements

| Requirement | Current evidence and disposition |
| --- | --- |
| S95-1/2/3/9 CI coverage and robustness | Inventory and focused controls pass. Hosted cold setup reaches real tests; bounded parallel Rust evidence has zero failures and healthy measured headroom. Actual test results remain separate from advisory wrapper success. |
| S95-4 installed consumers | Linux Python/Node examples, strict positive/negative TypeScript consumers, external Rust source archive and immutable CLI doctor pass. Candidate-bound Windows configuration supplements and the earlier five-platform collector pass on their recorded sources. All five prior-code native platforms and the collector pass; the CI16 repair requires fresh candidate CI. No registry-resolved .28 claim. |
| S95-5 documentation | Slice 92 baseline retained; affected interface/generator/lifecycle/surface/feature/target checks pass. The classification correction passes the portable validator and 38 tests; scoped Markdown validation passes. No repeated full documentation audit. |
| S95-6 broad and feature checks | One clean .28 canonical round passed 205/205 at `16234813c`, actual child exit 0; outer session 143 has unknown cause and is recorded separately. Later repairs have focused checks and fresh hosted verification. Feature assembly: 180 planned, 165 passed, 0 failed, 15 reviewed ignored. No uninterrupted full-local pass is claimed on the later source. |
| S95-6 performance and reliability | **Current candidate AC011a FAIL at 932.474344/s, unwaived.** AC011b 307.292/s, AC012 1/3 ms, AC013 55/61 ms, seven AC081 hard limits, 60-second AC021 and AC073 stress 47 ms pass. Existing AC081 soft warnings remain. Diagnostic tracing is nonqualifying and establishes no cause. |
| S95-6 accepted fidelity | Real TC5: 7,667 docs/100 queries/1,000 bootstrap, recall .958, CI [.938,.974], measured source `5f668f330`. Private vector-stage fidelity only; post-run database hash changed for unknown reason. EU7's retired recall assertion separately exits 101; its overall run is not called PASS. |
| S95-7 hosted CI | Run 42005653176 on `3a05a45cb` is **not qualified**: fast lint rejects the missing TC5 classification, so 198 cheap suites did not run. All five native targets/collector, five size cells, parallel Rust, strict security, default-provider and distinct Windows WAL pass. Heavy also passes 7/7 in 1,503 seconds. Corrected records candidate `6479399fe` passes local fast 198/198, lint/type/security 0/0/0. Corrected-source hosted fast remains pending; no overall CI-green claim. Earlier failures and successful run 41971020280 retain their actual source attribution. |
| S95-8 handoff | .29 has an uninterrupted 205/205 pass at `6674cf79d`, followed by reviewed forwards through `2803eb7f1` with affected native/logging/close/cursor checks passing. Reviewed metadata forward `d68de5db5` imports all nine final records exactly, with zero executable delta and passing scoped validators. All prior-phase task worktrees/branches were removed. Later records-only forwards preserve exact imported bytes and unchanged executable inputs. |
| S95-10 evidence safety | Pinned tracked-source Gitleaks passes with zero leaks on `6479399fe`. Separate scan covers 1,466 retained files/224.6 MB, including the full diagnostic trace: one match is independently verified as a source-file digest. Actual scanner exit 1 and the finding are retained. A separate added/changed-evidence supplement scans 56 files/3.77 MB with zero findings/errors/drift; no suppression or relabeling of the original result. |
| S95-12 confirmed-debt admission | Astra-approved CI8 RED/GREEN and independent 27 real-DB controls pass; the original hosted failures pass unchanged. Genuine competing-service refusal and true-debt progress are preserved. |
| S95-13 subscriber admission | Astra-approved CI12 deterministic native RED/GREEN, 36 PyO3 units, fresh shipping-wheel 10/10 subscriber tests and five examples pass. Final-code Python CI and affected feature/native refresh pass; capacity 4096 and full-only loss remain. |
| S95-11 rehearsal | Local preflight records .28 target versus unbumped .27 manifests. Immutable release dry-run remains pending version/Axis E/trusted workflow preparation. Its existing main-ancestry exemption is preserved. No version cut, main merge, tag, registry/Pages publication or release dispatch occurred. |

## Current follow-up and cleanup

Astra approved a narrow retrospective TC5 classification: preserve immutable
record/config/metrics/index bytes, add a data-plane plan/sidecar and the existing
exact-run/hash-bound plan and missing-kind artifact amendments. No preregistration,
public-search witness or new exemption is invented. Correction `af7e570ff` has
Astra design/code approval, 38 passing classification tests and independent
portable validation with all immutable bytes preserved. Clean `6479399fe` passes the affected local fast collection (198/198),
with live security 0/0/0. Retain explicit unchanged-input reuse for executable
evidence; hosted fast refresh remains pending until the final acceptance
candidate is settled. Do not add a new workflow or replay the full native matrix
solely for these records-only changes.

All worktrees/branches from CI1–15 were removed after clean/merged checks.
The standing Windows VM is restored off with byte-identical original persistent
configuration, original runner hash/service/policies unchanged, and task runner
registrations, folder and disposable disk removed. Exactly 13 matching staged
corpus files and the task-only feature target are removed; source corpus and
shared/user caches remain intact.

The previous phase preserved the checkout-owned debug cache on external cold
storage. On 2026-11-07 that volume is unmounted, leaving the original debug-cache
symlink unresolved; its contents are not freshly inspected. Do not discard data
or change the mount. Current verification will use a fresh checkout-owned Cargo
target directory. Prior cleanup/relocation claims describe their recorded phase.

CI16 records the authorized continuation: at most two diagnostic runs, followed
by review of an evidenced mechanism and focused TDD before any production repair.
Diagnostic runs cannot qualify throughput; no infrastructure or acceptance knob
is changed. The replacement continuation worktree and branch are now removed after verified
integration into both .28 and .29. Prior cleanup records remain accurate
for the completed phase.

D28-09/10/11 and populated-open deferrals remain. Slice 95 cannot close while the
required AC011a non-pass lacks a pass or explicit owner-authorized disposition.
Publication authorization remains separate and held.

## Diagnostic continuation on 2026-11-07

The historical Phase 1 result was 68,998 commits at a rounded 1,149.955/s.
Commit processing and the transaction body
were the largest observed phases; measured lock waits were small in that run.
The original temporary probe files and raw logs are no longer available, so that
summary is a historical transcription, not a fresh independently verified result.
Its instrumented rate does not qualify AC011a or explain the earlier miss.

The next bounded diagnostic uses 1,000 unchanged node writes to count/time the
exact unused edge-endpoint statement preparation and distinguish execution from
preparation/repreparation. The probe must be committed, rerunnable and indexed;
new raw output uses persistent ignored storage. A fresh isolated worktree at
`/home/coreyt/projects/fathomdb-worktrees/slice95-ac011-repair` replaces the missing
temporary worktree. Review an evidenced minimal repair and deterministic RED
before implementation; then use unchanged AC011a/b acceptance. Publication held.

Phase 2 independently confirmed 1,000 successful writes/rows and 1,000 initial
endpoint-query preparations with zero executions or automatic repreparations.
Astra approved lazy preparation on the first effective Edge and the scoped
real-SQLite RED design. Uninstrumented source was restored before production TDD. The Node-only test
failed on forbidden SQLite access at RED `592b64510`, then passed unchanged at
GREEN `ec7b7f014`. The implementation prepares only on the first effective Edge.
Astra code review found no material defect; 36 scoped controls and Clippy pass.
A separate verifier reran the new unit and all eight dangling-edge tests, and
confirmed the canonical qualifier remains byte-identical. This establishes avoidable work, not the cause of the old miss
or a throughput acceptance pass.

The [retained SQL probe](../../../../experiments/ac011-endpoint-prepare/README.md)
is committed and rerunnable. No diagnostic output is throughput acceptance. The
next steps are unchanged AC011a/b on the clean reviewed candidate, one final
canonical code-phase gate and fresh hosted CI, followed by the .29 forward and
cleanup. No new feature, durability setting or publication permission is involved.

The unchanged canonical qualification on `13c6470b5` failed AC011a at 827.268/s;
no source, binary or release-profile mismatch was found. Existing before/after
readiness checks remain applicable (no recorded competing workload or swap).
Whole-job CPU use was about 61%; that suggests waiting but does not identify its
cause. No rerun into green occurred, and AC011b was not reached.

Astra reassessed the diagnostic budget and approved one further release-mode
foreground wall/thread-CPU attribution run. It preserves the 5s/60s workload and
all guards, worker, trigger, synchronization and checkpoint behavior. Investigate
a wait stage only if it explains at least half the foreground non-CPU time and
at least 10% of write wall time; otherwise report an inconclusive result.
Production changes and additional runs require review of that evidence.

The release wall/thread-CPU diagnostic completed with matching success/stage
counts. Its signed non-CPU residual was -397.121ms, so the preregistered verdict
remains inconclusive. Review found nested wall/CPU clock boundaries can produce
that residual; the result is not silently corrected. Conservative analysis makes
the commit span a strong read-only lead, not proof of a SQLite or fsync cause.
The instrumented 1,011.349/s rate is not acceptance. Active instrumentation is
removed, and the retained tool remains rerunnable.

Source review confirms SQLite's default 1,000-frame auto-checkpoint can execute
synchronously from COMMIT under NORMAL mode. Astra approved one bounded
fsync/fdatasync-only trace of the unchanged canonical binary, with seccomp filtering,
120s watchdog and 32MiB raw-output cap. Whole-test foreground DB/WAL sync spans
>=5s justify only further read-only checkpoint/storage investigation. Startup,
warmup and teardown remain included; no measured-window percentage or policy
change is inferred. The tracer capability preflight passed.

The bounded sync trace independently verified 19.537003s of whole-test foreground
DB/WAL sync spans (2,282 DB and 4,562 WAL calls), with no syscall error, missing
identity, cap hit, timeout or fallback warning. This meets only the reviewed
read-only follow-up criterion; traced throughput is not acceptance. Source review
found normal checkpoint/restart synchronization, not a removable duplicate sync.

**[session] Durability contract decision pending.** The accepted
[durability ADR](../../../../adr/ADR-0.6.0-durability-fsync-policy.md) requires
NORMAL, <=100ms final-commit loss on power cut and zero OS-crash commit loss.
Frame-only checkpointing does not establish the time bound during low/idle
traffic. The process-kill fixture preserves host caches, and the separate
OS-crash test remains explicitly ignored for a missing VM fixture. These are
contract/evidence gaps, not permission to enlarge the checkpoint window. No
existing ruling waiving these durability promises was found in the live state
or ledger review.

The owner has been asked to select an explicitly reviewed durability direction:
preserve the strongest acknowledgement guarantees with suitable synchronization
and qualification, or explicitly adopt NORMAL with bounded loss for machine
failures and implement/validate its cadence. The 1,000/s threshold and publication
hold remain unchanged in either direction. No answer, waiver, checkpoint or
synchronization change is inferred. The current reviewed SQL repair and retained
tools may be integrated independently; policy changes and final acceptance wait.

The reviewed repair, diagnostic tools and current records are merged into .29 at
`6f67219e7d70286f5a073b5651663734efd1d8a9`. Separate verification passes the Node
unit (1), dangling-edge suite (8) and release-state views, with a clean tree and
unchanged canonical performance tests. The replacement task worktree and branch
are removed; raw evidence remains in the canonical .28 checkout. Integration is
complete for this repair; throughput acceptance and final qualification remain open.

## Metadata read repair and current reassessment

Astra approved statement-only reuse in `physical_erasure::load<T>` under the
existing S95-6 performance requirement. RED `f718be50bc7df9b875236bed208a700107ad47b4`
failed all three real-SQLite compilation assertions; GREEN
`10ca015034f6c55c5c2193a75e1eb4056d5bb948` passes those tests unchanged. Current
UPDATE/DELETE values, corrupt JSON, invalid typed state, missing-table recovery,
actual trigger off/on invalidation and stable-state reuse are covered. Every
existing read and fence, SQL/key, error mapping and durability setting remains.

Astra code review approved the exact clean GREEN commit. Separate verification
passes 14 scoped controls, including all three new regressions, erasure units,
rollback/visibility and queue/physical-fence cases; scoped Clippy passes. Raw logs
are preserved under canonical `target/slice95-ac011-evidence/metadata-reuse/`.

One unchanged canonical qualification on that clean commit then failed at
932.474344/s. Its exact counts, binary, source bundle, optimized profile, actual
exit and applicable readiness were independently recomputed. AC011b was not
reached. This is not acceptance or proof of causal improvement over 827.268/s.
The two-repair retry boundary now requires source/evidence/design reassessment
before any further implementation or measurement. Existing policy remains in
force; the pending durability question does not block policy-neutral work.

The metadata repair is independently verified on .29 merge
`b56083f16eeae80cbeef79dfef21cda55d5d429c`: identical incoming loader source,
14 focused controls passing, clean tree and unchanged qualifier. Its task
worktree and merged branch are removed, with all raw task logs preserved first.

Astra's read-only reassessment does not justify a third micro-repair. Trigger
configuration invalidates SQLite prepared statements, including FTS5 internals;
required dual-index commit flushing/merging is another plausible CPU cost.
Neither is quantified by the current sync trace. Astra proposed one bounded
user-CPU sampling investigation of the exact failed binary, preserving launch
session/autogroup and the unchanged workload; it is not acceptance. Its permission
preflight (`perf 7.0.14`, `perf_event_paranoid=4`) failed: no supported events and
performance-monitoring access denied. No benchmark profile ran or kernel setting
changed. A session question now asks the owner to authorize narrowly scoped
privileged profiling or choose a separately reviewed instrumentation design.
Until resolved, do not bypass permissions or start a fallback/profile/repair.
