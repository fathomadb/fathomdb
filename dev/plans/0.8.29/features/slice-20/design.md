# Slice 20 design

## Existing substrate and exact delta

[Queue admission](../../../../design/agent-access/queue-admission.md) test23
explicitly records the uncovered Ready→pre-enqueue race. Current `Registry`
captures `Next::Admit { load, epoch, changed }` under its mutex, unlocks, and
calls `admit`. Free permits bypass `QueueTicket`; queued calls increment before
`after_arrival_check`. After a permit, `admit` rechecks shutdown, phase and epoch;
stale attempts return None to the outer open-wait loop. `Load` survives reopen;
permit ownership already defines in-flight work. Idle/LRU drains require both
in-flight0 and queued0, so draining an already-queued ordinary waiter is not a
valid fixture. Shutdown24 covers its different reachable transition.

Delta: test-only observation of the existing gap and of stale rejection/engine
entry. Preserve runtime branches and error/deadline semantics unless a genuine
behavior failure is reproduced. No reader cancellation or retry estimate.

Engine `evidence_linearization_hooks` has two global `OneShotHook` values;
`arm` overwrites the singleton, any engine can consume it, and `fire` currently
invokes the closure while its mutex guard may remain alive. The graph-evidence
resolver already has an engine-owned hook; frozen-validation and reader-search
controls are already scoped. Delta: replace only sidecar/resolver ownership and
bounded rendezvous use, retaining their exact transaction positions.

`open.rs` startup/visibility/admission test hooks also use one global Option each
and compare raw registered path to `canonical_path` at firing. Noncanonical or
Windows verbatim identities can differ; concurrent arms can overwrite. This is
a source-derived hypothesis requiring deterministic current reproduction. TC57
and Slice40 versioned-parent helpers already configure before raw SQLite open;
Slice40 WAL fixture opens raw SQLite only in its explicitly separate child.

## Evidence rendezvous ownership

Prefer an Engine-owned test-only hook set, enabled only by `test-hooks`/unit
cfg. Keep the two existing free hook names and require an explicit `&Engine` owner,
as the existing frozen hook already does; update all callers atomically. Do not
keep old-signature compatibility shims. The existing test-seam inventory paths
remain valid; its default/refusal and explicit-feature consumer check still runs.
A hook has distinct armed/reached/released state and one owner; take the closure
out of its mutex before calling it. No registry/ownership mutex is held across
a rendezvous. An arm must not silently overwrite another outstanding arm.

Resolver fires directly from its Engine at the existing pre-return position,
inside the same transaction. Sidecar runs on a reader: clone the engine-owned
handle into the existing evidence-only `EvidenceSearchReaderRequest`, then
`EvidenceCapture`; fire at the existing post-ranking/pre-sidecar position.
Keep ordinary search's zero-sized `NoEvidenceCapture` and request/channel shape
free of evidence hook state. No ordinary search allocation or product field is
justified by a test seam. `open.rs` initializes owned test state consistently.

A bounded RAII pause owns release; hook closure signals reached then waits on a
release channel with a finite deadline. Drop/timeout releases actors, and every
spawned actor is joined or its bounded child is terminated/reaped. Existing
unbounded two-party Barriers in changed bodies are replaced only with equivalent
acknowledgments/cleanup, leaving semantic result assertions intact. A failed
assertion must not strand the test executable. TestA and testB use distinct real
DBs and frozen contexts, not database mocks or copied golden outputs.

RED recipe per seam: armA then execute a valid operation on B first. A bounded
channel proves whether A's callback was stolen; B must complete without it,
then A alone must reach it once. For simultaneous ownership, armA then armB,
launch both, require each named acknowledgment, release independently and
assert exact original per-engine evidence identities. Current global overwrite/
theft yields an actual assertion failure, without relying on scheduler luck.
Stage/commit that regression and its raw RED before changing ownership. If the
required owner argument needs compile scaffolding, include only that test-hook
signature/callsite adaptation in the RED commit while retaining the existing
global storage/fire behavior. Then observe actual A/B theft/overwrite assertion
failure; absent/new-signature compilation is not RED. All existing semantic
assertions remain unchanged, and GREEN changes ownership without editing the
staged regression. Reuse separate sidecar/resolver controls rather than one
big-bang GREEN.

## Ready→pre-enqueue observation and race23

Add a one-shot adapter-owned `before_enqueue` hook in the `Next::Admit` arm,
after dropping registry mutex and before entering `admit` (including its fast
permit path). It consumes only an explicitly armed store/request token. Return
a scoped pause handle to the test with async bounded reached/release methods.
One-shot consumption prevents a second admission attempt after reopening from
pausing again. All state is under existing `test-seams`; ordinary hooks inline
to no-ops. `release_all`, handle Drop, task cancellation and panic release/reset
held observations. A reached counter must decrement via Drop, not only the
normal continuation. Do not park a Tokio runtime thread on a Condvar.

Minimal read-only test observations: store/attempt epoch at reached; load's
in_flight/queued/abandoned snapshot; stale phase/epoch rejection event; bridge
entry `(store, epoch)` at the existing `before_call` point. Only explicitly
armed observations collect events, avoiding unbounded logs/allocations in normal
calls. These are test-only witnesses, not new admission configuration or result
fields. Public `Seams` additions document timing, one-shot ownership, timeout,
release and panic/cancellation cleanup. Update the test-only contract note in
[the Rust interface](../../../../interfaces/rust.md) for the required Engine
argument/lifetime and adapter seam feature boundary in the same implementation
PR; product result/error/configuration contracts stay unchanged.

Primary race uses real idle expiry because it does not leave another store
competing for the one-open-store limit:

1. OpenA and record epoch1; arm pre-enqueue pause and spawn its search.
2. Await reached(epoch1), assert in_flight0/queued0. A blocked pre-enqueue
   observer owns no permit/ticket and must not prevent the genuine idle close.
3. Observe A closed and SDK lock available. Arm blocked reopen before release.
4. Resume old attempt, observe stale rejection and reopening; assert no bridge
   entry for this call at epoch1 and no leaked counter. No claim follows from
   final response alone, which could miss a stale intermediate engine call.
5. Release reopen; get independently expected `l:note:a`, epoch2. Assert only
   epoch2 entered bridge, then later call succeeds and idle close releases lock.

Use actual readiness/lock/reached events with bounded waits; waiting for an idle
clock is not itself the race oracle. Paused time alone cannot drive this path
because Registry uses `std::time::Instant`. Keep total fixture waits well inside
configured deadline with generous supervision limits.

A second control leaves reopen blocked beyond the original absolute wait bound:
assert exact `not_ready/reopening` and bounded termination, not a renewed full
wait. A third test drops/aborts the test-only pause future after reached and requires
its held observation to clear through RAII; release_all and panic/unwind cleanup
also remain bounded. This tests seam lifecycle, not a new product cancellation
guarantee before permit acquisition. Preserve existing queued-cancel and
running-cancel controls: queued calls leave the line, started work retains its
permit until completion.
Shutdown at the gap must still refuse `not_ready/draining`; no forced idle/LRU
drain with queued work. Retain current busy-store/LRU controls to prove no
semantic weakening. Add a separate deterministic LRU-gap fixture only if idle
proof does not exercise the same stale-recheck branch; do not duplicate without
an independent branch obligation.

This adds proof for behavior already implemented. It has no production-defect
RED until a failing current behavior is actually found. The TDD choice below
must be settled before landing it.

## Startup/runtime, warning and tripwire work

For a reproduced startup hook identity failure, canonicalize registration with
the same `canonical_database_path` authority used by open, propagate setup error
rather than guessing identity, and key separate arms by exact DB identity where
parallel ownership requires it. Preserve lock-before-classification ordering.
Do not change production path simplification, refusal, provider admission,
verification precedence or cleanup to make a test hook fire. Prove unrelated
path cannot consume the hook. Existing same-path serial visibility tests remain
until that ownership reason is independently removed. Opener rendezvous needs
bounded ready/release and early-opener-error handling before Windows reruns.

TC57 and Slice40 first run intact in ordinary parallel and isolated modes.
If no current failure exists, record unreproduced/already-fixed and do not edit
working helpers. A new deterministic runtime test must preserve configure-before-
first-open, not call sqlite3_shutdown or force process-global state reset.

Warning diagnosis uses a clean non-test release build of the MCP binary without
`test-seams`, separately from test/dev-feature unification. Record exact item
and cfg graph; change only proven ownership. If absent, no speculative warning
fix or cfg-text mirror test. Release build warnings are not promoted to a new
blanket `-D warnings` policy.

T2 guard locations: engine fusion unit module reads live `RRF_K`,
`RRF_WEIGHT_TEXT`, `RRF_WEIGHT_VECTOR`; embedder `candle_bge` module reads live
`DEFAULT_EMBEDDER_NAME`, private `MAX_SEQUENCE_TOKENS` under default-embedder.
No model construction/network/GPU or new production export. Failure text names
[re-evaluation](../../../../design/overflow-chunking-reevaluation.md).
That document's accepted assessment, not a snapshot regeneration, governs future
expectation changes. T1 answer study and T3 kernel remain outside20; preserve
standing ledger retarget duty at release close rather than close the monitor
because T2 test was added.

## Exact TDD alternatives for design review

The verified strict memory forbids passing-on-arrival coverage except docs.
Neither option below is currently an owner ruling:

| Alternative | Concrete behavior and evidence | Consequence |
| --- | --- | --- |
| A: explicit owner exception for already-correct coverage | Add race23 instrumentation and five-constant guard as coverage, recording baseline behavior already correct. Require real reached/phase/entry witnesses. Demonstrate meaningful negative controls in disposable isolated verification trees: bypass stale rejection must be caught by race oracle; change each actual constant must fail its named guard. Restore those trees; never label intentionally broken production RED or fabricate repair. Existing confirmed defects still use per-finding genuine RED/GREEN. | Supplies approved missing20 proof and T2 monitor once exact exception is recorded. Requires no new product policy. |
| B: defer already-correct additions under unchanged strict rule | Implement only genuine reproduced repairs with per-finding RED/GREEN. Keep race23/T2 explicitly unimplemented and A3/A5 open, preserving existing queue source proof and standing tripwire record. No dummy failing test, missing-seam compile claim or altered expected constant. | Parent records precise defer/receiving disposition; affected20 exit/30 dependency and Block1 proof cannot be claimed satisfied by review alone. |

Astra review evaluates completeness/fit and negative-control feasibility; it does
not choose policy for the owner. Parent presents this concrete choice only when
relevant. Pending choice does not block approved10 preparation or independent
real-defect diagnosis.

## Actual completed10 adaptation

Actual104dcbcbe92a1eca730d146b99ab17d444df6217 has fresh independent Astra final
PASS and parent integration/closure d7d928f8e49d7af6a792385534f4e991c13232b3.
Reread actual plan/requirements/design/tasks/resources/test mapping, execution,
terminal review, final helper/caller diff, current214 registry and JSON/board.
The older98fc pending-candidate input is superseded. Current parent checkout is
/mnt/data/worktrees/fathomdb-0.8.29;10 COMPLETE_ON_RELEASE_BRANCH,20 NOT_STARTED.
No inspected MCP/engine/embedder Rust source or Rust workspace/feature test
selector changed. MCP documentation corrections preserve queue policy/refusal.

10's18 observed pairs include five earlier parent-audited REDs and independently
reproduced venv/module ownership corrections. Its single full wrapper failed
at98fc; an interrupted attempt and original211-suite207pass/4fail collector remain
recorded. Qualified affected reruns repair those four failures; three new registry
entries ran separately. Current214 total/207fast/7heavy is a registration count,
not an all214 collector pass or uninterrupted full green. Rust/TS/N-API pass,
Python1746pass/36skip and scoped security/lint/typecheck remain source-bound10
receipts, not20 or Windows qualification. No reason to rerun10 campaign wholesale.

The final common helper's --check-ownership is nonmutating and runs before full/
heavy bootstrap installers/hooks/npm. It rejects foreign activated env, symlinked
venv and symlinked/escaping root/TS modules before any mutator. Only --provision
creates a venv with canonical Python>=3.11 selection; actual sys.prefix checks,
pip/Ruff/Pyright checks precede installation/native work. --provision and --build
both freshly build from src/python, locked release with pyproject features and
exact interpreter, install a noneditable wheel, verify site-packages prefix/digest
outside source with PYTHONPATH cleared, and record source/delta/lock/features/
wheel/interpreter/ABI/native identity. --check-ownership does not build or claim
native readiness. agent-build still delegates Python before Cargo with no
system/timestamp fallback. Use the integrated helper, not old manual setup.

The canonical Python test runner now scopes `PYTHONPATH="$PWD/src/python"` only
to pytest/children. The unchanged sanctioned fresh-wheel/clean-source/owned-venv/
nonce/digest gate qualifies that candidate; do not globally export PYTHONPATH,
modify conftest/test oracle, or use production module hook presence as identity.
Installed-production lifecycle smoke remains a separate site-packages route.
Retain distinct hermetic TS/N-API and production artifacts. This child-import fix
is completed10 work, not another20 defect or test rewrite.

Known exact10 tools are Node25.9.0/npm11.12.1, Rust1.95, Python3.12.3/pip24.0,
Ruff0.15.17/Pyright1.1.410, Markdown CLI0.23.3. Parent now owns0.23.3 for document
checks; source10 tools remain another checkout's property. Parent creates20
fresh source worktree after closed10 strict preflight;20 must provision its own
venv/modules/native receipts under refreshed task resource reservations. Run
--check-ownership before npm/other mutators; never borrow10 env/native/target.

Carry forward bounded one-native/full-phase and Cargo/CMake jobs2 planning.
10 reviewed limits were root32GiB start/20GiB stop, taskfs250GiB start/50GiB stop,
total owned task storage200GiB, MemAvailable16GiB start/8GiB stop and sustained
swap control.10 sampled peak57,674,990,155B is historical evidence, not reserved
capacity. Source relocation to /mnt/data changes filesystem accounting: refresh
actual source/tmp/task filesystem free bytes and combined same-filesystem use
before provisioning; do not sum duplicate free-space readings as extra capacity.
Stop only owned work at bounds; no cleanup or paid expansion is inferred.

No implementation is authorized by this document update. Parent-controlled
updated Astra design PASS, fresh worktree/base verification, strict closed10
preflight, owned setup and explicit source GO remain. The owner coverageA/B
answer is still pending; genuine confirmed repairs retain observed separate
RED/GREEN while A3/A5 coverage additions stay held. Readiness remains NOT_READY,
publication HELD; no new fault/GPU/Windows permission or coverage-policy waiver.
