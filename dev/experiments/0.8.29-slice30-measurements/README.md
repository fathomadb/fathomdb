# Slice30 current direct-text measurement tools

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
| [g_spans.py](g_spans.py) | G-only sample-level inclusive span tree and demonstrated disjoint direct-child remainders; overlap remains nonadditive, errors/missing/drop attribution unavailable. No percentile subtraction. |
| [test_g_spans.py](test_g_spans.py) | Six frozen literal interval/availability/operation-identity/containment oracles. Selected by existing Python discovery; no natural campaign. |
| [g_runtime_tests.py](g_runtime_tests.py) and [normal CLI runner](../../../scripts/test-slice30-g-cli.sh) | Five actual normal-program controls: engine/SDK collector+nested unwind,shared public seed/configure/drain/report/reopen,governed1000-row boundary,early cap refusal,and shared first/warmup/read/off-on populations. Both use no provider and never qualify dense/GPU work. Canonical runner registration remains pending before final gate. |
| [test_tools.py](test_tools.py) | Fourteen bounded literal converter/math/runner/freeze/summary/manual-opt-in assertions. No database mocking, generated golden, tamper/digest-drift/path-hardening suite. |

The Rust example's six independently specified real-store assertions are
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
replacement-schedule/provider-selection assertions. The negative readiness fixture has two natural doc nodes and
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
positive Default workload. That production configuration, normal G-W wiring and
whole-cell/cumulative controller remain pending.
