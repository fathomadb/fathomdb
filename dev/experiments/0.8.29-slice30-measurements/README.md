# Slice30 P/G measurement tools

Accepted P remains provider-free direct text. Reviewed G source is now in
progress, with unchanged format37/product contracts and a separately gated
GPU campaign.
[Design](../../plans/0.8.29/features/slice-30/design.md) and
[preregistration](../../plans/0.8.29/features/slice-30/preregistration.md) own scope.
G design/actual20 fit passed independent Astra and source entry is authorized.
Scientific D/M/F execution and whole30 acceptance remain held; source checks do
not activate their campaigns.
No old prototype timing is a current baseline or fidelity result.

## Committed tools and origins

| Tool | Purpose and origin |
| --- | --- |
| [converter.py](converter.py) | First-seen unique nonempty LME turn/session order, bounded prefix, all question order and actual counts/hashes. Selectively adapted from07095a16dad020798bc5125cbde0c909e8c38a78 dev/experiments/ac013-scale-search/lme_corpus.py; no answer fields enter workload construction. |
| [campaign.py](campaign.py) | Balanced cell/adapter repetition order, bounded real child execution, separate fresh attempts, retained failures and resume of incomplete cells. Current implementation; selective scheduling concepts from the same ref's ssweep.py and aa30085fcb293d40cf5f34eaa568b3e215339cfc ac013-seed-drain/drain_sweep.py. No old prototype/provider/runtime hooks. |
| [metrics.py](metrics.py) | Nearest-rank p50/p99, arithmetic mean, error-inclusive throughput, matched query/repeat differences and fixed-seed query-cluster descriptive intervals. Hand-computed oracles; no treatment/numerical verdict. |
| [control.py](control.py) | Freeze actual source/lock/binary/input identities and balanced plan; serialized Linux campaign with resource observations; report every complete/failed/not-run/partial cell. Long CLI runs require AGENT_LONG=1. |
| [cells.json](cells.json) | Eight actual P base cells, two balanced repeats:16cells. P1 has engine/SDK passes; P2 is separately filtered marker-write work. |
| [slice30_measure.rs](../../../src/rust/crates/fathomdb-sdk/examples/slice30_measure.rs) | Current engine/SDK direct-text APIs, fresh governed real stores, warmups/raw calls, marker commits/probes, actual compiled-FTS match counts and close/reopen. Build with default SDK features; provider none, no model/GPU. |
| [g-cells.json](g-cells.json) and [g_campaign.py](g_campaign.py) | Complete fixed G-only inventory:84 cells including all72 required G-W; balanced stage/adapter/route/collector orders. Bounded owned attempts share whole-cell/cumulative/retained-byte caps, with explicit prerequisites and prospective feasibility. Frozen entry and actual campaign qualification remain separately gated. |
| [test_g_controller.py](test_g_controller.py) | Actual harmless Python children prove the literal complete stage inventory; no database/model simulation or positive GPU evidence. |
| [g_spans.py](g_spans.py) | G-only sample-level inclusive span tree and demonstrated disjoint direct-child remainders; overlap remains nonadditive, errors/missing/drop attribution unavailable. No percentile subtraction. |
| [test_g_spans.py](test_g_spans.py) | Six frozen literal interval/availability/operation-identity/containment oracles. Selected by existing Python discovery; no natural campaign. |
| [g_phase.py](g_phase.py), [g_resource.py](g_resource.py), [g_phase_tests.py](g_phase_tests.py) | Owned native/build executor: exact reviewed strict resource sampling, pinned foreground-group cleanup and refusal of leader-only success; actual harmless-descendant runtime controls. Resource caps are independent from measurement budgets and unchanged. |
| [g_boundary.py](g_boundary.py), [g_boundary_tests.py](g_boundary_tests.py), [g_boundary_entry_tests.py](g_boundary_entry_tests.py), [g_boundary_dispatch_tests.py](g_boundary_dispatch_tests.py), [g_phase_error_tests.py](g_phase_error_tests.py), [g_guarded_entry_tests.py](g_guarded_entry_tests.py) | Held task-owned cgroup-v2 controls contain the actual outer runner, separate cell session and native descendants. Real default-TERM/forced-escalation/normal-exit chains and controlled create/attach refusal retain independent runtime oracles. Unsupported delegation refuses before payload exec. |
| [g_runtime_tests.py](g_runtime_tests.py) and [normal CLI runner](../../../scripts/test-slice30-g-cli.sh) | Twelve actual normal-program controls (including closed-store span witness binding): engine/SDK collector+nested unwind,shared public seed/configure/drain/report/reopen,governed1000-row boundary,early cap refusal,shared first/warmup/read/off-on populations,exact diagnostic calls,normal G-W replacements,adjacent readiness,typed filters/future views,and actual open/device/close reports. Both use no provider and never qualify dense/GPU work. Canonical heavy runner registration is present; scoped native controls pass as recorded in execution.md; assembled source qualification is under final review; actual Default/GPU qualification remains pending. |
| [g_driver.py](g_driver.py), [g_attempt.py](g_attempt.py) | Current foreground driver binding and retained owned-process attempts under one absolute deadline. |
| [g_runner.py](g_runner.py), [g_freeze.py](g_freeze.py), [g_gpu.py](g_gpu.py), [g_report.py](g_report.py) | Strict build/freeze/run/report entry, registered source/input/cache/native pins, actual selected3090 observations and scoped retained reports. Native/GPU execution remains separately gated. |
| [g_forecast.py](g_forecast.py) | Whole G-W preparation/warmup/natural/write/readiness/point+FTS/raw-growth forecast with unchanged fixed caps. |
| [test_g_dispatch.py](test_g_dispatch.py), [test_g_lifecycle.py](test_g_lifecycle.py), [test_g_runner.py](test_g_runner.py), [test_g_pipeline.py](test_g_pipeline.py), [test_g_forecast.py](test_g_forecast.py), [test_g_freeze.py](test_g_freeze.py), [test_g_report.py](test_g_report.py), [test_g_entry.py](test_g_entry.py) | Frozen literal/actual harmless-child controls selected by discovery; no positive native/model/GPU qualification. |
| [g_reuse.py](g_reuse.py) | Read-only original-identity references through accepted base/branch/100k seed records; strict measurement equivalence, no replay/retagging. |
| [test_g_branch_polling.py](test_g_branch_polling.py), [test_g_reuse.py](test_g_reuse.py), [test_g_stages.py](test_g_stages.py), [test_g_exploratory.py](test_g_exploratory.py) | Nine real-child scheduling/resume/stop/admission controls; original51 G controls unchanged. No100k or positive GPU qualification. |
| [test_g_normal_resume.py](test_g_normal_resume.py), [test_g_normal_schedule.py](test_g_normal_schedule.py) | Normal seed-chain entry semantic RED/GREEN and harmless full84 order/resume preservation. Typed readiness fixtures are not native/model/GPU evidence. |
| [test_tools.py](test_tools.py) | Fourteen bounded literal converter/math/runner/freeze/summary/manual-opt-in assertions. No database mocking, generated golden, tamper/digest-drift/path-hardening suite. |

The Rust example's independently specified real-store assertions are
explicitly selected by test-slice30-measurement-adapter in the maintained agent
runner. Fourteen Python tool tests are selected by test-slice30-measurement-tools;
its controller is Linux-only and reports that boundary on other platforms.
These are bounded ordinary tests. They never launch the natural100k campaign.

## Commands

Use this checkout's own Python/native/tool environment and private/mnt/data
paths. Set TMPDIR, AGENT_LOG_DIR, CARGO_HOME, CARGO_TARGET_DIR and package caches
there before setup/build. Never borrow another checkout's install or target.
Build outside measurements:

```bash
cargo build --locked --release -p fathomdb-sdk --example slice30_measure
.venv/bin/python -m unittest discover -s dev/experiments/0.8.29-slice30-measurements -v
cargo test --locked --release -p fathomdb-sdk --example slice30_measure -- --nocapture
```

With OWNED and CARGO_TARGET_DIR pointing to the assigned/mnt/data task paths:

```bash
D=dev/experiments/0.8.29-slice30-measurements
.venv/bin/python "$D/converter.py" \
  --lme "$OWNED/inputs/longmemeval_s_cleaned.json" --limit 100000 \
  --out "$OWNED/converted"
.venv/bin/python "$D/control.py" freeze --cells "$D/cells.json" \
  --corpus "$OWNED/converted/corpus.jsonl" --queries "$OWNED/converted/queries.jsonl" \
  --binary "$CARGO_TARGET_DIR/release/examples/slice30_measure" --out "$OWNED/p-campaign"
AGENT_LONG=1 .venv/bin/python "$D/control.py" run "$OWNED/p-campaign"
.venv/bin/python "$D/control.py" summarize "$OWNED/p-campaign"
```

Freeze requires a new output directory. Each incomplete retry gets a fresh
attempt; completed cells remain attributed to their original files/native
subject. Preserve aborted attempts. A changed source/input/artifact campaign
needs a new frozen directory and the applicable reviewed scope, not a hidden
replacement. All raw text/DB/WAL/samples/logs stay outside tracked source.

## Output and interpretation

Converter outputs corpus.jsonl, queries.jsonl and inputs.json. Campaign freeze
writes owned read-only manifest.json/plan.json before timing. Runtime writes
resources.jsonl and results/campaign-state.json; each attempt retains cell.json,
child.log, store.sqlite and separate samples/warmups/batches/marker-probes/matches
JSONL plus result.json. Summary includes missing/incomplete cells and preserves
raw distributions; interrupted partial files are identified. Compile logs and
source qualification are separate from measurement output.

P1 uses exactly its stated natural-record count and no marker/filter. Only
complete unfiltered100k/500question/1000call P1 passes receive tracked AC083
80/300ms target distances; those are reports, not numerical pass gates. P2 adds
ten marker IDs, makes one existing atomic governed write(batch) call per1/10-record
cohort, filters doc-kind natural records, and inserts separate marker
freshness proof queries between writes and natural searches. Report their cost
and cache effect; P2 is not the tracked unfiltered100k fixture or a pure
write→search schedule without proof calls. batches.jsonl/batch_count count successful
governed calls; rows records their cohort size. Canonical batch atomicity follows
the existing SDK/core commit contract, not instrumentation of all ancillary
SQLite commits. No projection-overlap claim follows.

Raw streams append after each completed operation and flush every16records per
stream plus warmup/pass/diagnostic boundaries, without fsync. Result metadata
reports encoded bytes, record/checkpoint counts, persistence and checkpoint time.
Between completed checkpoints, up to15tail records per stream may be unavailable;
interruption during a pending flush may leave that checkpoint partial. Completed
UTF8 JSONL prefixes survive a partial final line, which is reported as an issue.
A useful raw prefix never turns an interrupted cell into complete qualification.
The bounded child test stops only its own process after the16th checkpoint ACK;
this proves tool output retention, not database recovery or power-loss durability.

API sample timing stops before output record hydration/serialization. Loop
throughput includes serialization/checkpoint I/O and, for P2, writes/probes; it is labelled
separately from API latency. Raw FTS match counts use the actual default product
compiler on the controlled active node fixture, separately from limit10 returned
hits. This is match/cost characterization, not relevance gold. Snapshot authority
is explicitly unavailable; write receipts do not prove search-snapshot reuse.
OS cache is uncontrolled, so fresh-engine/process calls are not cold-disk claims.

One lock, descriptive CPU load with noisy flag>2.5,900s cells, cumulative4h and16GiB
measurement output cap apply with the reviewed outer owned resource guard.
No compilation/concurrent owned campaign is allowed in the timing window.
Source, failed attempts, skips and outliers remain explicit. Whole30, fidelity,
MCP/Windows/Block1 and final160 obligations are not discharged by this P subset.

## G implementation and bounded ordinary tests

The [reviewed continuation](../../plans/0.8.29/features/slice-30/continuation-gd.md)
and [execution](../../plans/0.8.29/features/slice-30/execution.md) distinguish source
GO from technical GPU campaign GO. No accepted P measurements are repeated.
The existing SDK example now has four additional ordinary route/fixture/
replacement-schedule/provider-selection assertions. The original T2 unit readiness fixture has two natural doc nodes and
two separate body-bearing control edges; pending2/blocked without a provider is
not positive dense or GPU qualification. The engine observer ownership target
uses two real stores and no model:

```bash
cargo test --locked --release -p fathomdb-sdk --example slice30_measure g_route_tests -- --nocapture
cargo test --locked --release -p fathomdb-sdk --example slice30_measure g_fixture_tests -- --nocapture
cargo test --locked --release -p fathomdb-engine --features slice30-observer,test-hooks --test slice30_g_observer -- --nocapture
```

The nondefault observer is bounded and engine/session/operation owned. Nine tiny
real-store tests now cover ownership/reader transport, exact guard short-circuit
facts, fallback selection, same-call snapshot/time authority and API/reader phase
attribution. Source-window membership and pending-probe controls use a deterministic
8d test provider, not BGE. The actual normal CLI now also
proves SDK observer entry/reader transport and nested operation restoration across
unwind in the explicit negative-provider control. The test-only
forwarding/phase surface is documented in rust.md. No G campaign, BGE/GPU execution
or selected natural dense store is qualified by these ordinary assertions.

The G-only replacement schedule updates existing first-ten natural doc IDs with
original body plus a cohort-specific marker token, using one governed write per
cohort. Point and direct-text marker probes have separate clocks and precede
natural unfiltered calls; they affect cache state and loop throughput. No
interbatch drain or separate marker nodes are added. The tiny ordinary schedule
test uses no provider and explicitly expects the typed embed failure.

The shared G seed driver commits batches of1000, including the final partial
batch, and reports actual successful chunk lengths/receipt cursors. P's1024 loop
is unchanged. Positive G seed calls must receive the controller's numeric
remaining_cell_ms in0..=900000; it can only reduce the fixed cap. Negative source
controls may omit it. Zero allowance returns NOT_RUN_CAP_TIME before DB creation;
after database creation, exhausted allowance remains PARTIAL_CAP_TIME with the
owned store retained. driver_wall_ns starts before corpus/query read and output
setup. It is not the full parent cell clock: outer controller startup/config
read/final reporting and aggregate14400s/16GiB enforcement remain required.

The shared G read driver now emits separate first-call/warmup/measured streams,
per-route loop walls and actual operation batches when enabled, outside each
outer API timer. Ordinary and diagnostic variants use the same current route
calls; ordinary builds reject requested collection and carry no engine observer
state. Tiny negative route controls are not the final G-O branch/time matrix or
positive Default workload. Production diagnostic configuration and whole-cell/
cumulative controller binding are implemented below; actual Default/GPU
qualification remains pending.

Normal G-W now shares the tested scheduler with its ordinary unit control. One
adapter/route uses one fresh matching store; warmups precede writes, first-ten
existing natural IDs are replaced, and natural calls remain unfiltered. Point
and FTS probes are separate. Optional operation batches include warmup/natural/
FTS-probe calls; loss marks incomplete attribution. Independent readonly SQLite
history/active counts witness both atomic cohort sizes and absence of extra
marker nodes. These provider-free controls do not qualify the72-cell GPU matrix.
The readonly Python witness uses contextlib.closing; its cleanup-only patch was
independently approved with URI/queries/assertions unchanged.

Diagnostic G-O filters and explicit valid_as_of sequences use the current public
core/SDK APIs; natural G-R stays unfiltered with its default view. The source
window control uses public canonical/derived writes and dependency registration
with a current-admissible future boundary. It is synthetic, not a natural LME
fixture. Captured engine facts bind each query's authority/cursor/resolved time.
Each G open now retains the actual public report, child PID/executable and device
resolution as reported; None remains absent, never fabricated CPU/CUDA. Separate
close outcomes and seed reopen-close clocks are retained. This enables later
GPU qualification, but no Default/model/GPU execution is established here.

`g_driver.py` binds G-only inventory to current measurement-program commands.
The G-W command protocol seeds each fresh matching subject before the write
step, passing one declining absolute deadline. `test_g_dispatch.py` uses actual
harmless Python command children and qualifies protocol only; it never opens a
DB or supplies model/GPU evidence. All fixed stage bindings and frozen entry are
implemented; actual native/Default/GPU qualification remains pending. Run controller/dispatch guards via unittest
discovery (`-p 'test_g_*.py'`), so classes following the historical direct-file
main guard in `test_g_controller.py` are imported and exercised.

Actual private-cgroup controls are separate from portable `test_*.py` discovery.
Run `AGENT_SLICE30_LIVE_CONTROLS=1 bash scripts/test-slice30-g-control.sh` with
checkout-owned `.venv`, task-owned `S30_TASK`/temporary files and an unconfined
executor with writable delegated cgroup-v2 support. Selection hard-fails missing
prerequisites and checks the unchanged32GiB root/250GiB task/16GiB memory floors.
The final owned source gate selects this route with
`AGENT_SLICE30_LIVE_CONTROLS=1 ./scripts/agent-verify.sh`; an ordinary unselected
gate is portable and does not establish live containment coverage. No model or
selected measurement store is accessed by these controls.

G stage binding now covers G0, G1, G-R, natural G-O and all six tiny G-O cases.
Tiny branches retain a common untouched subject for direct/hybrid/filter/no-match
reads, a public source fixture with returned future T-1/T views, and four fresh
seed/write subjects for commit-before/after comparison. They share one900s cap.
Harmless children prove command protocol only; actual branch facts, qualified
Default subjects/device/engagement and production frozen-controller integration
remain required before any campaign qualification.

The canonical heavy tier explicitly selects the observer target, normal CLI
controls and standalone feature-boundary consumers. The latter retain positive
and negative compile logs; expected absent-method E0599 errors are compile-only
contract evidence, never runtime RED. The source-derived feature matrix records
the observer target's exact feature requirements. Native execution and the final
full gate remain serialized behind the assigned heavy/native window.

`g_attempt.py` owns each G child process group, with retained per-attempt configs,
logs/receipts and an absolute deadline spanning preparation/startup. Cap timeout
terminates the owned group so descendants cannot continue work after the cell;
the accepted P runner remains unchanged. This controller timeout proof uses only
harmless Python children, without a product fault or model/GPU execution.

`test_g_lifecycle.py` separately proves monitor-exception cleanup, unreaped-leader
ownership through signals, and rejection/cleanup of successful leaders leaving
live foreground descendants. Fixed G production drivers inherit their attempt's
private session; detached commands are not part of this contract. Group scans
exclude already dead zombies, retain uncertainty on unavailable evidence, and
never release the leader pin before the last signal/quiescence decision.

`g_phase.py` additionally holds an exclusive private cgroup under its current
delegated scope; its guardian stays outside that child boundary. Attachment is
only a preopened-descriptor write/close before exec in a single-threaded guardian.
Existing pinned outer TERM/grace remains; exact private cgroup forced cleanup
also contains nested sessions, and populated0 precedes exact empty-group removal.
No parent controller, limit or unrelated process is changed. Unavailable control
or cleanup is a refusal/unknown result, never an uncontained fallback. Composed
controls use real harmless children and test-only direct-libc pidfd rescue;
forbidden writes are judged only after guardian completion.
The task's `g-containment-state.json` is claimed exclusively before any payload.
A retained marker prevents another guardian from overwriting unresolved ownership.
Normal G entry requires a live guardian and matching private boundary. Missing,
unknown, malformed or stale state refuses before dispatch, even if the shared
budget's inflight field is clear; a bare unguarded run has no fallback. Root
membership records use one separator. That root-path protocol control does not
claim actual namespace execution. No marker is silently cleared to retry.
Unknown private boundaries remain retained for explicit resolution. Resource
errors retain original text diagnostics and a failed terminal receipt; child
stdout/stderr still write directly through the log's file descriptor.

`g_runner.py` is the bounded G build/freeze/run/report entry.
The reviewed November14 [staged continuation](../../plans/0.8.29/features/slice-30/continuation-gd.md#november14-proposed-staged-g-continuation)
adds `freeze --stage branch-controls|first-100k-seed-exploratory
--reuse-completed-from ROOT`. These closed stages retain all84 obligations and
original completed-record identities. Each normal `run ROOT` stops after its
named cell and keeps incomplete-campaign exit1; `last-run.json` separately
reports stage completion. Normal no-stage freeze may instead use only the accepted
exploratory seed root, inheriting all six original references while assessing the
unchanged full84 plan; it adds no subset or stage flag. New actual assessment
remains held pending source acceptance/full qualification and technical GO.
Exploratory admission is one-shot in the same shared
ledger, retains the failed2x forecast and both900s hard caps, and checks prior/
assessment/preparation use before dispatch. Source acceptance/full qualification
and separate technical GO remain required before either new actual stage.

`test_g_runner.py` uses actual harmless commands to prove protocol-only reports
cannot count as Default qualification and that native-command PIDs are observed
independently. Positive runtime facts still require the reserved3090 campaign.
Provider driver/toolkit fields are optional in current Candle; fixed R580 and
primaryCUDA12.6 must be observed separately, never filled into provider reports.

`g_forecast.py` computes every required whole G-W preparation/workload/probe
component and raw-growth estimate. `test_g_forecast.py` freezes literal cost
inputs, exact cohorts and an actual forbidden harmless-child dispatch witness.
These are prospective-policy controls, never substituted measurement evidence;
missing current observations refuse the still-required cell.

## G production entry

These commands are provided for the authorized native/campaign phases; no build,
GPU query, model call or campaign has run during Python preparation. Use the
checkout-owned environment and reviewed resource executor. Build/native checks
wait for the native window; timing additionally waits for the clean tested
source/preregistration/assets/reserved3090 technical campaign GO.

```bash
G_DIR=dev/experiments/0.8.29-slice30-measurements
G_TOOLS="$S30_TASK/g-tools"
.venv/bin/python "$G_DIR/g_runner.py" build --out "$G_TOOLS"
.venv/bin/python "$G_DIR/g_runner.py" freeze \
  --corpus "$S30_TASK/inputs/corpus.jsonl" \
  --queries "$S30_TASK/inputs/queries.jsonl" \
  --license "$S30_TASK/inputs/LICENSE.txt" \
  --ordinary "$G_TOOLS/slice30-measure-ordinary" \
  --diagnostic "$G_TOOLS/slice30-measure-diagnostic" \
  --build-receipt "$G_TOOLS/build-receipt.json" \
  --model-cache "$HF_HOME/hub/models--BAAI--bge-small-en-v1.5/snapshots/5c38ec7c405ec4b44b94cc5a9bb96e735b38267a" \
  --gpu-uuid "$G_GPU_UUID" --gpu-ordinal "$G_CUDA_ORDINAL" \
  --allocation "$S30_TASK/gd-allocation" --out "$S30_TASK/gd-allocation/g-frozen"
AGENT_LONG=1 .venv/bin/python "$G_DIR/g_phase.py" g-campaign-first \
  .venv/bin/python "$G_DIR/g_runner.py" run "$S30_TASK/gd-allocation/g-frozen"
.venv/bin/python "$G_DIR/g_runner.py" report "$S30_TASK/gd-allocation/g-frozen"
```

Copy only existing registered static input/cache bytes into those owned paths;
there is no download or dataset regeneration path. Freeze verifies current
input/model pins, exact ordinary/diagnostic features and bytes, clean source,
Rust/native inputs, owned noneditable Python identity and selected3090/driver/
primaryCUDA metadata. It prints the campaign UUID for the technical ready packet.
Ordinary tools use embed-cuda only; diagnostics add slice30-observer, never test-hooks.

Run preserves all84 obligations and failed attempts. Preparation/startup share
900s percell; all tranches share14400s and retained16GiB through the allocation
ledger. Unknown inflight work refuses; completed matching outcomes survive.
G-W needs qualified current seed/read/tiny-probe costs and the complete forecast.
Each native PID is independently observed; selected-GPU memory/utilization is
sampled after1s polling waits with full raw records. G0 alone uses50ms polling
waits to observe short-lived native processes; observation latency adds to each
wait, so this does not guarantee20Hz. Actual same-PID memory and selected-GPU
positive-utilization evidence remains required; scheduling controls cannot qualify it. Provider null optional driver/toolkit values
remain null; actual host checks supply those facts. Environment/attempt metadata
uses unique retained invocation/prepared files; a failed postflight invalidates
its new final attempt rather than rewriting native raw data or older evidence.

Reports keep every obligation and all errors/outliers. G-O/small diagnostics have
counts and means only; ordinary1000-call populations use accepted nearest-rank
math and query-cluster paired intervals. Span remainders are emitted only when
per-sample parentage/disjointness supports them; overlapping children stay
nonadditive. These are host intervals, never exclusiveGPU costs or corrected
latency. Python33-control evidence is not native/GPU qualification.

Remaining native entry work includes actual stored byte-range/body-length span
coverage (with UTF-8 byte offsets), integrated normal-program/feature consumers,
then the one stable full qualification and fresh final implementation review.
No D/M/F/whole30/40/50/Block1 or Windows qualification follows from this runner.

Stored coverage is retained as `steps[].span_coverage` in each actual cell
`result.json` for native G-seed and source-fixture steps. The read-only
[g_span_coverage.py](g_span_coverage.py) witness exposes every stored window/start/
exclusive-end/body-length in UTF-8 bytes, plus full/partial/unknown counts and
known represented-byte totals. Unknown ends remain null; span counts alone make
no full-body claim. The real-store scalar-head Unicode control uses a test
provider and does not qualify Default384/GPU behavior. This reporting does not
change projection storage or impose a new partial-coverage acceptance threshold.


## CLOSED-copy source candidate

The existing G entry now supports `--preparation closed-copy-v1` with fixed
`closed-copy-calibration`, `closed-copy-read-r1` and `closed-copy-write-matrix`
modes. Each stops before another mode; all84 primary obligations remain visible,
calibration is separate and exact original references are never redispatched.
`--calibration-from` binds qualified records; `--cost-review-from` binds the
reviewed write table/profile. Freeze records the prerequisite; supervised run
recomputes actual costs and refuses unavailable/changed admission. These source
interfaces grant no measurement or prospective-cap authority.

| Tools | Contract |
| --- | --- |
| [g_closed_entry.py](g_closed_entry.py), [g_closed_common.py](g_closed_common.py), [g_closed_guard.py](g_closed_guard.py), [g_common_storage.py](g_common_storage.py) | Supervise and charge common freeze/preflight/report/guardian work, including retained metadata and interrupted active cells; unknown cleanup blocks entry. |
| [g_closed_copy.py](g_closed_copy.py), [g_admission.py](g_admission.py) | Supervised fresh CLOSED copies and validation before native calls; public cell/common workers require live matching owned controller/boundary and exact admitted config/deadline. |
| [g_closed_costs.py](g_closed_costs.py), [g_closed_forecast.py](g_closed_forecast.py), [g_closed_matrix.py](g_closed_matrix.py) | Match actual qualified calibration/R0/R1 costs, retain original refusals and conservative2x forecasts, and bind the complete reviewed G table with explicit held D/M unknowns. |
| [g_closed_limits.py](g_closed_limits.py), [g_closed_review.py](g_closed_review.py) | Validate prospective-only named G profile/table; preserve old11 and qualified R1 subject identities, prior charges/token and unchanged native artifacts. |

Portable source controls remain under `test_g*.py`; selected actual cgroup
controls use the explicit live route above. Tiny real SQLite/harmless children
are source protocol evidence, never Default/GPU qualification. Current scoped
results and original failures are retained in the existing execution record.

### D fixed-table feasibility controls

`d-cells.json` retains all24 required fresh C/T executions and900s cell limits.
`d_controller.py` exposes the pure prospective assessment/dispatch seam and a
closed initial held quartet, with all remaining obligations explicit. Its
stdlib controls run with the checkout-owned minimal Python:

```sh
.venv/bin/python -m unittest discover -s dev/experiments/0.8.29-slice30-measurements -p 'test_d_*.py'
```

Literal lower-cost fixtures and harmless child markers qualify admission math
and ordering only. They do not measure D, supply GPU/overlap/fidelity evidence,
or mutate the real shared allocation. Fresh supervised wall/retention/budget
custody and native observation/report bindings are implemented; final source
qualification and separately selected runtime admission remain required.

`d_driver.py` adds the callable accounting adapter for that controller. Its
`test_d_driver_accounting.py` controls use only hand-specified clocks and in-memory
budgets: charge the outer wall once across common/failed-cell portions, bound the
next deadline by remaining allowance and retain unresolved owned cleanup. This
has a guarded CLI and durable allocation/supervisor binding; the initial held
quartet remains the default stop boundary and no campaign is activated. Successful selected
executor results are bound to their current native report inside the accounted
cell interval before publishing lower evidence. `test_d_execution_routing.py`
checks this connection; missing native/C capture or observed storage cost stays
incomplete, without a live payload dispatch. C's actual JSONL paths now feed
that qualification through owned read-only file contexts. Missing/partial
streams preserve unavailable results and raw prefixes; T opens no C stream.

`d_report.py` binds current receipts/physical closure to exact-engine/session
worker groups, durable commit coverage and natural-call overlap witnesses.
`test_d_report.py` uses hand-computed two-group intervals; untimed facts stay
outside duration means and T keeps worker attribution unavailable. The private
C-H and C-O native entries have tiny real-store stream/closure controls,
including pre-seed acknowledgements and final take after close. D capture uses
bounded nonblocking multi-producer transport; full/closed loss still prevents
completion. C-O capture alone does not establish an overlap witness. The fixed
native entry routes the same canonical workload to T/C helpers; `d_attempt.py`
has real harmless-child success/deadline/descendant controls. `d_runner.py`
uses the shared named-phase guardian with separate D custody/common charges,
explicit reservations and actual outer-wall reconciliation. Owned refusals add
only their actual D common charge; foreign custody remains unreconciled/held
without changing its ledger. All24 outcomes and original attempted records
remain visible; no scientific D cell follows from source qualification.

The maintained heavy runner selects the D native example, transport and private
engine contracts. `test_d_*.py` discovery selects portable math/process controls.
Linux private-boundary/actual CLI controls are explicitly selected by
`AGENT_SLICE30_LIVE_CONTROLS=1`, using this checkout's own `.venv` and delegated
`S30_TASK`; missing prerequisites fail the dedicated command:

```sh
bash scripts/test-slice30-d-control.sh
```

`d_runner.py source-control --root "$S30_TASK/d-source-control"` runs only an
isolated harmless controller/payload protocol and reports `qualifies_natural`
false. Future scientific entry is `AGENT_LONG=1 d_runner.py run --options FILE`
under the same executor. Options name owned `root`, original `allocation`, native
`binary`, selected `profile` file and a funded `common_limit_ns` inside the
unchanged shared total. The profile must explicitly bound other-work reserves;
unknown is never zero. No selected profile or runtime GO is supplied here.

Python admission and the native pre-store entry record/refuse inherited active
projection batching or presence-gated reader/writer PRAGMA/search-limit
experiments. They retain inert values unchanged and never silently clear the
environment to label an experimental execution as default.

Initial entry runs only C-H-r0,T-H-r0,T-H-r1,C-H-r1. Continuation sets `initial`
false and names a `review` file: explicit ACCEPT/reviewer/subject, next `n`/`mode`,
nonfuture used/retained checkpoint, qualified prefix attempt identities and
positive C/T time/retained bounds. The existing same-mode conservative minima
still apply to larger sizes. Exactly that next quartet is eligible; later
obligations stay held, and actual failed/incomplete attempts are never retried
automatically. `test_d_continuation.py` states independent manual examples.

`d_overlap.py CASE.json` reads one exact cell/attempt/size/repeat and owner/session
frame, with natural operation identity, tagged observer records and bracketed
pending observations. It qualifies only an entire positive pending bracket
inside a nonempty actual query/worker intersection. Missing/loss/error data is
INCOMPLETE; disjoint/straddling data is NO_OBSERVED_OVERLAP; T remains
ACTUAL_OVERLAP_UNOBSERVED. Raw witness bounds are retained without exclusive
cost or percentile subtraction. The tiny real-store rendezvous and literal
scope controls prove this predicate, not any scaled scientific execution.

## M tooling — campaign runtime held

[m_runner.py](m_runner.py) is the committed M entry. The accepted inventory is
[m-cells.json](m-cells.json):2 M0,20 M-R,4 M-C,8 M-L and12 M-E. All campaign
commands retain the selected original shared ledger, prior charges/failures,
900s whole cells and exact production defaults. No M-only candidate profile is
active, and no calibrated latency or weak-result verdict is emitted.

Portable source controls use new SDK-created tiny stores and actual stdio:

```bash
M_DIR=dev/experiments/0.8.29-slice30-measurements
bash scripts/test-slice30-m-cli.sh
```

That maintained runner builds separate owned provider-free CLI and test-seams
helper targets with locked release/jobs2, then explicitly executes
[m_native_tests.py](m_native_tests.py). It requires checkout-owned `.venv`; no
environment-absent skip can replace the tests. Bounded harmless child/hand-metric
tests remain in ordinary `test_*.py` discovery. Actual guardian/controller-death
and public prepared-dispatch controls run separately:

```bash
bash scripts/test-slice30-m-control.sh
```

The live control route requires owned `S30_TASK/tmp`, the preceding owned CLI/
helper artifacts, delegated private cgroup controls and32/250/16GiB root/task/
available-memory start floors. It fails actionably when unavailable. Both
runners are registered explicitly in the Linux heavy test tier; live controls
are selected only with `AGENT_SLICE30_LIVE_CONTROLS=1`, independently of
`AGENT_LONG`. The owned final qualification selects and executes that route;
ordinary developer verification keeps the private-executor control opt-in.
The dedicated control script itself always hard-fails missing prerequisites.
The direct `source-smoke --root ... --cli ... --helper ...` command is portable;
`source-dispatch` additionally exercises actual preparation, inherited private
admission, normal MCP processes, shared/common accounting and retained reports
under the live guardian. Both explicitly remain tiny provider-free source
controls, never natural/GPU qualification or renewed campaign allocation.

The separate [bounded calibration](m_calibration.py) command is implemented;
runtime remains held for parent selection and candidate/input/device readiness:

```bash
AGENT_LONG=1 .venv/bin/python "$M_DIR/m_runner.py" calibrate --options selected-calibration.json --root "$M_CAL" --cell M-R-1-1-warn
```

Options reuse the selected original allocation/profile and natural input fields,
with only the ordinary build and no primary forecasts. Explicit `calibration`
limits are `phase_ns`, `payload_ns` and `new_retained_bytes`; the reviewed first
proposal is900s/800s/2GiB. Entry reserves2x remaining time/growth headroom and
charges failures to the same cumulative ID across roots. Existing raw/attempt/
report files retain calibration separately from all46 held primary rows. Only
this unchanged1501-call compact hybrid c1/warn configuration is selectable;
there is no automatic sweep or promotion. The three `test_m_calibration*.py`
modules use literal arithmetic, fake dispatch/boundaries and harmless children;
they establish no natural/native/GPU/cgroup qualification.

Future reviewed primary runtime uses the same entry, after independent source review,
owned tooling qualification, an explicitly approved common TOTAL allocation
and complete bounded remaining-component forecasts:

```bash
# Each build requires the existing guarded owned CPU/CUDA lease as applicable.
.venv/bin/python "$M_DIR/m_runner.py" build --variant providerfree --output "$S30_TASK/m-build-providerfree"
.venv/bin/python "$M_DIR/m_runner.py" build --variant helper --output "$S30_TASK/m-build-helper"
.venv/bin/python "$M_DIR/m_runner.py" build --variant ordinary --output "$S30_TASK/m-build-ordinary"
AGENT_LONG=1 .venv/bin/python "$M_DIR/m_runner.py" freeze --options selected-runtime.json --root "$M_FROZEN"
AGENT_LONG=1 .venv/bin/python "$M_DIR/m_runner.py" run "$M_FROZEN"
AGENT_LONG=1 .venv/bin/python "$M_DIR/m_runner.py" report "$M_FROZEN"
```

`selected-runtime.json` is an explicit private input configuration, not generated
authority. It names `allocation`, `profile`, `forecasts`, `closed_subject`, pinned
`corpus`/`queries`/`license`, owned `model_cache`, separate `builds` receipts and
the reserved `gpu` identity. [m_freeze.py](m_freeze.py) defines the checked fields.
The profile must be `slice30-m-budget/v1`, `status:APPROVED`, pin the original
`g-budget.json` and state TOTAL nanosecond/retained-byte caps plus900s cells.
It also declares `reserved_other_work_ns`/`reserved_other_work_bytes`, an explicit
reviewed `reservation_disposition`, and `other_work_cost_status` (`BOUNDED` or
`HELD_UNAVAILABLE`). A zero reservation is an explicit bounded-M disposition,
never an estimate of unknown G/D work; unknown combined feasibility stays
`UNKNOWN`. Neither reservation nor common growth silently expands the original
TOTAL allowance.
Forecasts include every cell's `components_ns` and `new_retained_bytes`, plus
`common_components_ns` and `common_new_retained_bytes`;
[m_controller.py](m_controller.py) names all13 components. Complete remaining
cell/common growth is reserved at2x before selected packaging inspection or
preparation; external reservations are additionally protected by the guardian.
No cap, profile approval, missing component, closed custody or companion file is
inferred. No live-source copying, checkpoint, reset, projection edit or model
fetch is implemented. Ordinary builds use only `embed-cuda`; the diagnostic
helper is separately provider-free and uses only existing private Seams.

The outer [m_guard.py](m_guard.py) owns the exact private boundary and original
shared accounting even if the controller dies. Interrupted active wall is
charged once to its cell; residual outer work goes to common usage, and retained
`guardian_stopped`/uncertain inflight state blocks redispatch. Inner attempts
reserve7s for existing bounded cleanup inside their absolute allowance; the
outer guardian reserves20s for group/boundary cleanup and finalization. Public
commands bind private cells to the live owning controller and active attempt.
Reports retain all46 rows and every attempt/error/unresolved observation. Actual
MCP-PID memory increase plus positive utilization qualifies hybrid compute;
direct-text query compute and provider-free controls are N/A. No extra query,
borrowed GPU witness, CPU fallback or service-time subtraction is introduced.
Raw stdout/stderr retain the fixed per-line flush policy and report actual
write/flush bytes and costs separately; decoded JSONL uses16-record/phase
checkpoints. Queue summaries remain at whole-process scope, with phase counts
as population context; uncorrelated admitted-wait events are never assigned to
first/warmup/timed windows.

Registered queries are JSONL strings, decoded before both loop and lifecycle
submission; quotes, escapes, Unicode and original query indices are preserved.
[m_load.py](m_load.py) observes actual one-minute host load. Under the owner's
November15 shared-environment ruling in the
[protocol](../../plans/0.8.29/features/slice-30/preregistration.md#statistics-noise-and-attribution),
CPU load is descriptive across P/G/D/M: busy entry proceeds without a CPU-load
wait, refusal or disqualification. Observed load>2.5 still marks noisy without
discarding results, changing ceilings or correcting latency. Cell monitor
observations, calibration authorization observations, search-phase boundaries
and completed guardian-phase facts remain in their raw outcomes/reports.
Missing observations are never zero load. Resource floors, time/growth caps,
owned containment, exclusive GPU locks and GPU noninterference remain enforced.

## Slice50 attributable comparison support

The private `slice50-measurement` engine/SDK feature selects the retained eager-
body/deferred-identity route with
`FATHOMDB_SLICE50_EAGER_TEXT_FOR_MEASUREMENT=1`. Unset retains winner hydration;
identity, providers, defaults and direct-text streaming stay the same. General
`test-hooks` is excluded from both measurement builds. Existing G manifests and
ordinary/diagnostic feature contracts remain unchanged; their original native
artifacts cannot be retagged as this pair.

The public entry is the existing `g_runner.py`, with a separate private canonical
scope:136 ordinary members from68 original search cells,24 diagnostic members
from12 named origins, and a distinct four-member10k calibration. Default G84/P16
inventories and artifact admission remain unchanged.

Build current owned tools only in the parent's reserved native window, then
select the full lower-stage calibration explicitly:

```bash
.venv/bin/python dev/experiments/0.8.29-slice30-measurements/g_runner.py \
  pair-build --out "$OWNED/pair-tools"
AGENT_LONG=1 .venv/bin/python dev/experiments/0.8.29-slice30-measurements/g_runner.py \
  pair-calibrate --options "$OWNED/pair-options.json" --out "$OWNED/calibration-10k"
.venv/bin/python dev/experiments/0.8.29-slice30-measurements/g_runner.py \
  pair-report "$OWNED/calibration-10k"
```

The options name `build_receipt`, `allocation`, `profile`, `corpus`, `queries`,
`license`, `model_cache`, `gpu_uuid`, `gpu_ordinal`, and `g_donors` (mapping
`"10000"` to the original qualified G root; primary also needs`"100000"`).
Calibration additionally selects `calibration` with integer nanoseconds/bytes:
`phase_ns:900000000000`, `payload_ns:800000000000`,
`new_retained_bytes:2147483648`. It runs four fresh full500-warmup/1000-sample
members through both adapters and text/hybrid, ordinary then diagnostic.
`pair-run` cannot resume this calibration as primary work.

The explicit owner-selected profile has schema`slice50-pair-budget/v1`,
status`APPROVED`, `whole_cell_ns:900000000000`, the original allocation's
`ledger` path, positive `total_shared_ns`/`total_shared_retained_bytes`, explicit
`reserved_other_work_ns`/`reserved_other_work_bytes`, `other_work_cost_status`
(`BOUNDED` or explicitly `HELD_UNAVAILABLE`) and a nonempty
`reservation_disposition`. The existing G-only profile grants no50 allocation.
The original ledger, prior failures/common/calibration charges, retained data and
reservations carry forward. Calibration needs twofold remaining time/growth
headroom and cannot renew its cumulative allowance through another output path.

`pair-freeze --options ... --out ...` and `pair-run ROOT` handle primary scope
only after complete current paired forecasts exist. Options must include a
positive bounded `common_limit_ns` and `forecasts` with
status`CURRENT_PAIRED_BOUNDED`, positive `common_ns`/`common_bytes`, and all160
member IDs in `cells`. Each member declares positive `whole_ns` and
`new_retained_bytes` at least as large as its applicable actual current
`observed_from` root/`observed_member` cost. Provider, size, route/adapters,
mode, observer and write schedule must match. Old timings and10k proxies cannot
price P preparation,100k or writes; those remain unavailable after calibration.
No automatic next stage, caller subset or inferred full-table fit is admitted.

Freeze copies the registered inputs/model bytes to owned paths and binds current
artifacts/features plus data-only closed G donors. Primary preparation creates
provider-none P templates once with the original1024-write seed loop and closes
them; every member consumes a fresh validated closed copy. The public guardian
owns qualification, preparation, dispatch, reporting and strict cleanup.
`pair-report` reads the retained supervised report; if none exists for a frozen
root, explicit`AGENT_LONG=1` generates it through the same bounded guardian.
A source-only report without a freeze lists every member as unrun.

The existing `g_driver.run_program_step` sets `slice50_pair_mode`: `winner`
clears the private selector and `eager` sets1 in that child. Both modes use the
same actual current artifact. Diagnostic work/RSS uses the separate observer
artifact and never enters ordinary percentiles. Exact ordered native calls,
record cursors/score bits/sidecars, full populations and actual closes are checked
before pair credit. `retire_complete_copies` defaults false; when explicitly
selected it retires only compared complete disposable copies after required
case diagnostics and known actor quiescence. Immutable donors/templates, original
P/G histories, failed copies and raw calls/results remain retained.

Before actual runtime, bind the exact applicable existing30 cell IDs/configs and
all remaining obligations, paired mode/repeat order, qualified immutable inputs,
current native/features/pins/model and conservative complete-table feasibility.
Use fresh matching CLOSED subjects for each write member and reverse mode order
on repeat1. Preserve original sizes,500 warmups/1000 timed calls,500 question
order, adapters/routes, write cohorts/probes and error dispositions. No selected
subset, old artifact or refused prefix qualifies missing cells. Current30 natural
cells have snippets off: natural prefix benefit is N/A, with completed prefix
work/allocation proof retained separately.

Pair-feature search rows add ordered `hits` containing cursor/source/stored body/
kind/branch and exact score/CE bits after the existing API timer, alongside existing
cursor/fallback/degradation metadata and same-build snippet/explanation Debug
sidecars. The latter are None in fixed natural cells; no cross-build serialization
contract is implied. No public Engine or SDK result changes.

With an active existing operation observer, successful readers emit one bounded
`slice50_work` record: route, lightweight rows, vector/FTS owned-body conversion
counts and UTF-8 byte sums. Vector and FTS conversions of the same record count
separately; borrowed text validation is excluded. Missing/error/lost attribution
is unavailable, never zero or SQLite-I/O/total-allocation evidence. Relevant
request limits remain the fixed current30 limit10/direct-text node window100 and
current vector fanout, with actual returned/candidate counts retained separately.
Diagnostic step output includes sampled child `rss_bytes`/`peak_rss_bytes` from
VmRSS/VmHWM with timestamps/availability; the existing poll wait and observer
latency apply. Unsampled peaks are unavailable. Ordinary latency and loop
throughput retain all errors/outliers and are never corrected by subtracting
these diagnostics.

Maintained source controls are `search::paired50_tests::` (real fixed-embedder
stores, no general test hooks), the existing example's paired record-output test,
`Paired50DispatchTests` in `test_g_dispatch.py` (harmless children only), and
`test_pair_public.py` (public inventory/admission, actual stream comparison,
existing driver/copy binding and cumulative calibration limits).
The agent runner explicitly selects both native private-feature suites; these
controls do not start a natural/model/GPU campaign or discharge R6/whole30.
