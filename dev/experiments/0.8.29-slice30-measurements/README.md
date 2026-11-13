# Slice30 current direct-text measurement tools

Initial P only: provider-free direct text, unchanged format37 and contracts.
[Design](../../plans/0.8.29/features/slice-30/design.md) and
[preregistration](../../plans/0.8.29/features/slice-30/preregistration.md) own scope.
G/D have a reviewed bounded design and explicit source GO; implementation/TDD
are underway. No G/D campaign or final qualification is claimed. M/F, whole30,
40/50 source and pre90 remain held.
No old prototype timing is a current baseline or fidelity result.

## Committed tools and origins

| Tool | Purpose and origin |
| --- | --- |
| [converter.py](converter.py) | First-seen unique nonempty LME turn/session order, bounded prefix, all question order and actual counts/hashes. Selectively adapted from07095a16dad020798bc5125cbde0c909e8c38a78 dev/experiments/ac013-scale-search/lme_corpus.py; no answer fields enter workload construction. |
| [g_inputs.py](g_inputs.py) | Thin streamed selected corpus→s30doc-ID/doc/body and selected questions→numeric ID/query record JSONL. Text/order/duplicates preserved; no turn/question reselection; P converter unchanged. Literal/Unicode framing/count/round-trip CLI oracles in test_g_inputs.py and test_g_query_inputs.py are selected by maintained root Python discovery. No full-size preparation. |
| [campaign.py](campaign.py) | Balanced cell/adapter repetition order, bounded real child execution, separate fresh attempts, retained failures and resume of incomplete cells. Current implementation; selective scheduling concepts from the same ref's ssweep.py and aa30085fcb293d40cf5f34eaa568b3e215339cfc ac013-seed-drain/drain_sweep.py. No old prototype/provider/runtime hooks. |
| [metrics.py](metrics.py) | Nearest-rank p50/p99, arithmetic mean, error-inclusive throughput, matched query/repeat differences and fixed-seed query-cluster descriptive intervals. Hand-computed oracles; no treatment/numerical verdict. |
| [control.py](control.py) | Freeze actual source/lock/binary/input identities and balanced plan; serialized Linux campaign with resource observations; report every complete/failed/not-run/partial cell. Long CLI runs require AGENT_LONG=1. |
| [slice30_gd_diagnostic.rs](../../../src/rust/crates/fathomdb-engine/examples/slice30_gd_diagnostic.rs) | G/D core-only diagnostic scaffold, real current APIs, fixed-selector completion test in native/test_completion.py; explicit maintained heavy suite. First RED used an empty feature; current private per-engine transport nests exact-reader snapshots, actual predicate/SQL spans and vector/text blocks. Its tiny None-provider driver and deterministic768/test-hooks closed-template fixture are not campaign adapters. No public SDK test accessor. |
| [template_verify.rs](../../../src/rust/crates/fathomdb-engine/examples/slice30/template_verify.rs) | Private shared closed-template verifier, now wired into D_TEMPLATE: exact per-kind canonical(kind,cursor)→mapping→physical(mapping.rowid)/span joins, missing/extra cardinality, physical dimension/profile and actual UTF8 span bounds. Emits detailed per-kind/per-row coverage; D keeps unknown NULL ends. Normal BGE closed producer wiring is accepted at7664a2557; generic campaign budget/population routing remains pending. |
| [slice30_held_tool.rs](../../../src/rust/crates/fathomdb-engine/tests/slice30_held_tool.rs) | Tiny D12 current-API controls: verified-empty configuration, freeze plus empty idle drain before governed rows, actual pending/vector states, explicit or Drop release before final drain/close, same attempt clock and independent physical/copy controls. Maintained test-slice30-gd-held; actual held/released reports serialize observed readiness/SQL counts and clock boundaries after the genuine missing-report RED. No cancellation/real-overlap or campaign qualification. |
| [slice30_projection_measurements.rs](../../../src/rust/crates/fathomdb-engine/tests/slice30_projection_measurements.rs) | Tiny real deterministic768 D worker source oracle: mixed doc/note governed cohorts, exact physical cursor/vector/span/profile and closed-copy controls, independent query stream, actual worker/job/clock conservation. Separate helper oracle propagates an actual readonly SQLite type error. Maintained test-slice30-gd-projection-workers selects both; actual separate worker/Drop transport includes fixed-bound loss/toggle/clock controls. The initial backed-buffer scaffold had genuinely unwired recording. No held/overlap or campaign proof. |
| [slice30_template_tool.rs](../../../src/rust/crates/fathomdb-engine/tests/slice30_template_tool.rs) | Tiny deterministic768 doc/note template metadata: existing governed write/drain/close, independent physical cursor/vector/span/profile checks, closed hash-bound copy and reopen. Literal plus bounded property oracles preserve input order and UTF8 body lengths; covered byte_end remains unknown. Maintained test-slice30-gd-template builds the explicit test-hooks driver and executes both tests. No BGE, lifecycle or campaign-sized template qualification. |
| [gd_overlap.py](gd_overlap.py) | One-attempt strict common interval/full positive readiness witness analysis, with explicit unavailable worker attribution for controls. Literal geometry and bounded translation oracles are in test_gd_overlap.py and selected by maintained Python discovery. |
| [gd_feasibility.py](gd_feasibility.py) | Pure lower-scale resource forecasts; current time/storage/shared-clock dimensions use integer-ceiling1.5headroom against870s work/900s total and16GiB per family including retained evidence, with prep/waits retained in the4h clock and30s cleanup reserved. Overall hypothetical eligibility requires complete lower evidence plus all three dimensions; reports ELIGIBLE_PROJECTION or NOT_RUN_RESOURCE. No database/model/launcher or actual resource admission. Hand-computed hypothetical observations and bounded monotonicity cases in test_gd_feasibility.py, selected by maintained Python discovery. |
| [native/build_g_host.py](native/build_g_host.py) | Locked release builder for the private slice30-g-measurement host. Records actual compiler profile/features/source/lock/binary and pins the immutable CUDA12.6 compiler. Normal slice30-g closure refuses TC5/test-hooks/observer; optional slice30-g-observer identity remains separate. Existing TC5 executable/protocol is unchanged. |
| [native/test_gg_governed.py](native/test_gg_governed.py) | Manual separate GG provenance/lifecycle copy: supported derived note/source doc, actual readiness and independent canonical/current/retained vector/span/FTS/closure inventory, literal valid/Deleted views then shared Core measured stages. Initial unsupported fact fixture remains NOT_RED. Maintained test-slice30-gg-governed; tiny65+2 source proof, not100k campaign or phase-budget qualification. |
| [native/test_gg_time.py](native/test_gg_time.py) | Manual GG time-copy stages: real baseline100, one governed90..110 window row, before80/inside100/after120 public-view literal controls followed by actual hybrid warmup/timed calls and same-reader guard/snapshot observations. [g_gg_time.rs](../../../src/rust/crates/fathomdb-sdk/examples/slice30/g_gg_time.rs) uses existing public APIs and the shared observer/Recorder, without test knobs. Maintained test-slice30-gg-time; source fixtures are65→66 records, not the100k campaign or governed dependency stage. |
| [native/test_g_observer.py](native/test_g_observer.py) | Manual distinct GN versus same-artifact GO off/on Core transport, actual correlated query records/drop deltas, mean N/A and original InvalidFilter/FrozenRead/no-stamp controls. Uses private neutral callbacks in [g_observer.rs](../../../src/rust/crates/fathomdb-sdk/examples/slice30/g_observer.rs) and actual public Core error calls in [g_observer_controls.rs](../../../src/rust/crates/fathomdb-sdk/examples/slice30/g_observer_controls.rs); feature references stay in the private host. Set S30_GO_ARTIFACTS to archived gn/go binaries and matching *-features.json; maintained test-slice30-g-observer builds/freezes both before running. No SDK forwarding, campaign or paired-cost qualification. |
| [native/test_g_g2_pass_plan.py](native/test_g_g2_pass_plan.py) | Manual supplied G2 descriptor forwarding: six SDK mode/schedule passes on ONE writable copy, fresh reset per pass, forward/reversed mode+RO/write order, warmups2/timed20 and cumulative current/history controls. Existing G1 absent-schedules routing is preserved. Selected by test-slice30-g-g2-pass-plan; balanced descriptor generation and campaign admission remain TGD14. |
| [native/test_g_generic_actuation.py](native/test_g_generic_actuation.py) | Manual generic SDK text/hybrid/mean b10/s10 linkage to the unchanged G scheduling helper: initial warmups2/timed20 plus bounded10/30 cases, actual raw timed boundaries, phase/probe/prefix/flush/current-history checks and unchanged read-only schema/control. Selected by test-slice30-g-generic-actuation; tiny fixtures do not qualify real cell budgets or population/pass descriptors. |
| [native/test_g_kind_filter.py](native/test_g_kind_filter.py) | Manual generic SDK G2 doc-only retrieval after real ten-note reset: actual returned note IDs expose missing text/hybrid filtering, literal and bounded doc-count/warmup oracles check eligible IDs, mean remains unfiltered. Selected by test-slice30-g-kind-filter; scheduled actuation remains separate. |
| [native/test_g_generic_reset.py](native/test_g_generic_reset.py) | Manual generic SDK G2 precursor: two actual governed ten-note resets on one writable closed-template copy, positive/zero warmups, bounded doc populations and independent current/history cursor joins. Same pass-origin reset/first-record placement excludes reset from the elapsed loop. Selected by test-slice30-g-generic-reset; kind filtering and scheduled actuation remain separate. |
| [native/test_g_pass_plan.py](native/test_g_pass_plan.py) | Manual real G1 pass execution: six completed Core/SDK modes in forward/reversed order, one cell-store copy, fresh per-pass open/close/raw proofs and separate warmup/timed populations. Literal and bounded repeat oracles count actual completed passes before full assertions. Maintained test-slice30-g-pass-plan uses existing manual GPU prerequisites; source fixtures are not campaigns. |
| [native/test_g_warmups.py](native/test_g_warmups.py) | Manual G1 warmup count/order oracle across all six actual modes, first query-prefix warmups versus unchanged timed population, invocation ordinals, zero-warmup control and bounded positive counts. API value/device/closed-copy controls precede missing-call assertions. Uses the same shared observation/clock path; G2 coupling remains for its initial actuation unit. Maintained test-slice30-g-warmups uses existing manual GPU prerequisites. |
| [native/test_g_query_feed.py](native/test_g_query_feed.py) | Manual normal-G single-pass input routing across all six Core/SDK modes: actual API-input fingerprints, arbitrary ordered IDs, literal text alpha/beta/no-match and bounded permutation invariants after real preparation/closed-copy/API/CUDA controls. Source fixture mode iteration is separate from the remaining same-store campaign pass scheduler/warmups. Maintained test-slice30-g-query-feed uses the existing manual GPU prerequisites. |
| [native/test_g_limit.py](native/test_g_limit.py) | Manual private normal-G Core/SDK hybrid retrieval-limit oracle on a real closed65-vector BGE fixture. Existing governed/physical/copy/API/CUDA controls precede limit10 assertions; no learned ID order. Shared g_template_fixture.py supports an explicit binary/proof while legacy defaults stay unchanged. Selected by maintained test-slice30-g-retrieval-limit with the existing manual GPU prerequisites. |
| [native/test_g_mean.py](native/test_g_mean.py) | Manual normal core mean source oracle: pinned3090/actual PID+UUID memory and utilization,128 real mean calls then exact raw records/checkpoints. SDK example slice30_g_mean uses embed-cuda only, no test-hooks/observer/standalone CLS. Source TDD, not campaign. Maintained test-slice30-g-core-mean selector requires AGENT_LONG=1 plus exact reserved UUID and forced cuda:0; otherwise reports manual skip. |
| [native/test_g_sdk_mean.py](native/test_g_sdk_mean.py) | Separate SDK mean runtime oracle with fixed sdk/sdk_mean literals and shared128-call/GPU/checkpoint contract. Actual SDK open uses use_default_embedder:true; mean calls use SDK::embed. Maintained test-slice30-g-sdk-mean has the same explicit manual GPU prerequisites, never standalone CLS. |
| [native/test_g_core_hybrid.py](native/test_g_core_hybrid.py) | Manual core hybrid population: governed two-row/two-vector fixture, pending0, literal alpha+beta result set,128 real calls and retained raw records. Uses normal mean BGE/GPU; no observer/test-hooks. Maintained test-slice30-g-core-hybrid uses the same explicit manual prerequisites. This fixture does not implement closed campaign templates. |
| [native/test_g_sdk_hybrid.py](native/test_g_sdk_hybrid.py) | Separate SDK hybrid oracle: existing SDK governed projections/write/drain/readiness/search, same literal two-ID fixture and retained records. Maintained test-slice30-g-sdk-hybrid uses the explicit manual GPU prerequisites. |
| [native/test_g_text.py](native/test_g_text.py) | Core text-only records on a fresh hash-bound copy of the accepted closed BGE fixture. Fixed bounded manual READY/ACK validates own PID/UUID memory before API clocks; no query inference-utilization claim. Maintained test-slice30-g-core-text uses normal Default/forced-CUDA identity and the explicit manual prerequisites. |
| [native/test_g_sdk_text.py](native/test_g_sdk_text.py) | Separate SDK text-only population with the unchanged closed-copy/READY-accounting oracle and fixed sdk/sdk_text literals. Existing public SDK API, no query inference-utilization claim. Maintained test-slice30-g-sdk-text has the same explicit manual prerequisites. |
| [slice30_g_template.rs](../../../src/rust/crates/fathomdb-engine/tests/slice30_g_template.rs) | Manual normal BGE closed producer oracle:65-row fixed64 cohort boundary plus small Unicode property series, actual CUDA/Mean/profile/nonNULL head-span coverage and fresh closed-copy/reopen controls. Private g_template.rs branch uses the shared physical verifier; build_g_template.py records actual normal feature closure and g_template_fixture.py bounds required GPU proof. Target test=false excludes automatic Rust selection; explicit maintained test-slice30-g-template uses AGENT_LONG/exactGPU prerequisites. No campaign-sized preparation. |
| [native/test_g_write_schedule.py](native/test_g_write_schedule.py) | Real SDK text G2 post-group actuation:10natural calls then one atomic10-row write/drain/10literal probes/current coverage. New G-only phase framing and four flush points preserve completed-flush prefixes with explicit gaps; current and historical vectors stay distinct. Literal20 plus bounded10/20/30real cases; maintained manual test-slice30-g-write-schedule. Fixed tiny source unit, not full generic campaign routing. |
| [native/test_g_readonly_schedule.py](native/test_g_readonly_schedule.py) | Bounded SDK text G2 zero-write control: ten note markers reset/drained outside pass, twenty actual doc-filtered natural calls, explicit zero scheduled writes/probes. Reuses closed-copy/READY accounting, no query inference claim. Maintained manual test-slice30-g-readonly-schedule; bounded SDK-text actuation is accepted at46ee7cf6e; generic/full-mode campaign schedules remain pending. |
| [gd_resources.py](gd_resources.py) | G/D conservative allocated/apparent aggregation, setup/raw reservations and owned child phase guard; selectively adapts the committed Slice60 resource guard. Literal math in test_gd_resources.py and real tiny-child/final-sample/group/du controls in test_gd_guard_*.py are selected by the existing maintained Python discovery suite. |
| [cells.json](cells.json) | Eight actual P base cells, two balanced repeats:16cells. P1 has engine/SDK passes; P2 is separately filtered marker-write work. |
| [slice30_measure.rs](../../../src/rust/crates/fathomdb-sdk/examples/slice30_measure.rs) | Current engine/SDK direct-text APIs, fresh governed real stores, warmups/raw calls, marker commits/probes, actual compiled-FTS match counts and close/reopen. Build with default SDK features; provider none, no model/GPU. |
| [test_tools.py](test_tools.py) | Fourteen bounded literal converter/math/runner/freeze/summary/manual-opt-in assertions. No database mocking, generated golden, tamper/digest-drift/path-hardening suite. |

The Rust example's six independently specified real-store assertions are
explicitly selected by test-slice30-measurement-adapter in the maintained agent
runner. Python tool tests are selected by test-slice30-measurement-tools;
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

## Current G/D query primitives

The engine-only slice30-measurement feature is independent of test-hooks and has
no SDK/binding forwarding. Current private query records contain completion,
same-reader snapshot, actual predicates/SQL subspan, vector/text candidate blocks
reader-result outcome/unavailable state, and a separate cumulative drop report.
The current tiny driver remains a
None-provider fixture; natural G/D adapters/model/campaigns are not implemented.
Exact scopes/failures/subjects live in
[actual execution](../../plans/0.8.29/features/slice-30/execution-gd.md).
Maintained heavy suites explicitly select these rerunnable targets:

```bash
cargo test --locked --release -p fathomdb-engine --test slice30_measurement_snapshot --features test-hooks,slice30-measurement
cargo test --locked --release -p fathomdb-engine --test slice30_measurement_predicates --features test-hooks,slice30-measurement
cargo test --locked --release -p fathomdb-engine --test slice30_measurement_stats --features slice30-measurement
cargo test --locked --release -p fathomdb-engine --test slice30_measurement_outcomes --features slice30-measurement
cargo test --locked --release -p fathomdb-engine --lib --features test-hooks,slice30-measurement measurement_for_test::tests:: -- --nocapture
cargo build --locked --release -p fathomdb-engine --example slice30_gd_diagnostic --features slice30-measurement
python3 dev/experiments/0.8.29-slice30-measurements/native/test_completion.py
```

The fixed32768capacity test uses32769actual tiny-store queries and no intermediate
drain; bounded property cases assert zero-drop conservation. Stats are cumulative
per Engine and are not atomic with a separate drain: join queries before count
proofs, report differences rather than adding repeated cumulative readings.
No capacity setter or product routing/default change is supplied.

## G/D phase resource guard

After the explicit G/D source GO, source the new owned env-runtime.sh and run:

```bash
python3 dev/experiments/0.8.29-slice30-measurements/gd_resources.py PHASE COMMAND ARG...
python3 dev/experiments/0.8.29-slice30-measurements/test_gd_resources.py
```

The owned task's receipts/gd-source-entry-aggregate.json supplies disjoint roots
and initial baseline. Allocated/apparent max is applied per root, with G/D raw
subtrees subtracted per metric before setup growth is computed. Limits remain
accepted96GiB setup/16GiB each raw/200GiB aggregate, plus fixed disk/memory/swap
floors. Raw campaign directories are task/g and task/d. The guard records actual
command/cwd/source/delta/exit, resource snapshots and separate child exit; a
resource stop is124, preflight refusal125. Only its own child process group is
terminated and waited, including descendants after leader exit. A process-local
Linux subreaper scope restores its previous state; only the created process
group is signaled/reaped (TERM15s, then KILL/5s completion bound). Final resource
observations govern disposition, and failed du numeric partial totals are retried
within two attempts then refused with diagnostics. Logs/receipts persist under task/logs and task/receipts.
Tiny unit roots/caps exercise enforcement without a heavy/native/model workload;
normal CLI uses the accepted limits and real entry inventory. No tamper/drift or
new product resource API is introduced. Old P artifacts/environments stay read-only.

## Normal G core mean source proof

Build the SDK example slice30_g_mean with --locked --release --features embed-cuda.
The manual native/test_g_mean.py oracle requires the exact approved3090 UUID in
CUDA_VISIBLE_DEVICES and FATHOMDB_EMBED_DEVICE=cuda:0, own S30_GD_TASK/Cargo
target and preverified pinned inputs. Invoke only under the reviewed current
resource guard (explicit13 roots/baseline69280403456), reserving the GPU and
checking actual floors. No model fetch or CPU alternative. Its bounded source
fixture hashes binary/test BEFORE runtime and retains output/GPU samples under
own proofs/g-mean. ELIGIBLE_PROJECTION never starts this fixture or a campaign.

The existing P recorder moved to the private shared example helper
examples/slice30/raw.rs. Methods retain the same16-record checkpoint/phase
flush policy and costs; unchanged P example controls cover relocation. G's
API timer stops before validation/serialization/checkpointing. Loop throughput
includes those costs and excludes separately scoped open/close. Core/SDK mean, hybrid and text-only populations, the bounded zero-write control
and normal/D closed-template tools are implemented. Write schedules and final
source/lifecycle/campaign qualification remain pending.
