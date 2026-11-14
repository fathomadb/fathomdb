# Slice30 P/G measurement tools

Accepted P remains provider-free direct text. Reviewed G source is now in
progress, with unchanged format37/product contracts and a separately gated
GPU campaign.
[Design](../../plans/0.8.29/features/slice-30/design.md) and
[preregistration](../../plans/0.8.29/features/slice-30/preregistration.md) own scope.
G design/actual20 fit passed independent Astra and source entry is authorized.
D source, M/F, whole30,40/50 source and pre90 remain held.
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
| [g_phase.py](g_phase.py), [g_resource.py](g_resource.py), [test_g_phase.py](test_g_phase.py) | Owned native/build executor: exact reviewed strict resource sampling, pinned foreground-group cleanup and refusal of leader-only success; actual harmless-descendant runtime controls. Resource caps are independent from measurement budgets and unchanged. |
| [g_runtime_tests.py](g_runtime_tests.py) and [normal CLI runner](../../../scripts/test-slice30-g-cli.sh) | Twelve actual normal-program controls (including closed-store span witness binding): engine/SDK collector+nested unwind,shared public seed/configure/drain/report/reopen,governed1000-row boundary,early cap refusal,shared first/warmup/read/off-on populations,exact diagnostic calls,normal G-W replacements,adjacent readiness,typed filters/future views,and actual open/device/close reports. Both use no provider and never qualify dense/GPU work. Canonical heavy runner registration is present; scoped native controls pass as recorded in execution.md; assembled source qualification is under final review; actual Default/GPU qualification remains pending. |
| [g_driver.py](g_driver.py), [g_attempt.py](g_attempt.py) | Current foreground driver binding and retained owned-process attempts under one absolute deadline. |
| [g_runner.py](g_runner.py), [g_freeze.py](g_freeze.py), [g_gpu.py](g_gpu.py), [g_report.py](g_report.py) | Strict build/freeze/run/report entry, registered source/input/cache/native pins, actual selected3090 observations and scoped retained reports. Native/GPU execution remains separately gated. |
| [g_forecast.py](g_forecast.py) | Whole G-W preparation/warmup/natural/write/readiness/point+FTS/raw-growth forecast with unchanged fixed caps. |
| [test_g_dispatch.py](test_g_dispatch.py), [test_g_lifecycle.py](test_g_lifecycle.py), [test_g_runner.py](test_g_runner.py), [test_g_pipeline.py](test_g_pipeline.py), [test_g_forecast.py](test_g_forecast.py), [test_g_freeze.py](test_g_freeze.py), [test_g_report.py](test_g_report.py), [test_g_entry.py](test_g_entry.py) | Frozen literal/actual harmless-child controls selected by discovery; no positive native/model/GPU qualification. |
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

One lock, start load<=2, noisy flag>2.5,900s cells, cumulative4h and16GiB
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

`g_runner.py` is the bounded G build/freeze/run/report entry.
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
AGENT_LONG=1 .venv/bin/python "$G_DIR/g_runner.py" run "$S30_TASK/gd-allocation/g-frozen"
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
sampled at1s with full raw records. Provider null optional driver/toolkit values
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
