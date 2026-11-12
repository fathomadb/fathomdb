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
| [campaign.py](campaign.py) | Balanced cell/adapter repetition order, bounded real child execution, separate fresh attempts, retained failures and resume of incomplete cells. Current implementation; selective scheduling concepts from the same ref's ssweep.py and aa30085fcb293d40cf5f34eaa568b3e215339cfc ac013-seed-drain/drain_sweep.py. No old prototype/provider/runtime hooks. |
| [metrics.py](metrics.py) | Nearest-rank p50/p99, arithmetic mean, error-inclusive throughput, matched query/repeat differences and fixed-seed query-cluster descriptive intervals. Hand-computed oracles; no treatment/numerical verdict. |
| [control.py](control.py) | Freeze actual source/lock/binary/input identities and balanced plan; serialized Linux campaign with resource observations; report every complete/failed/not-run/partial cell. Long CLI runs require AGENT_LONG=1. |
| [slice30_gd_diagnostic.rs](../../../src/rust/crates/fathomdb-engine/examples/slice30_gd_diagnostic.rs) | G/D core-only diagnostic scaffold, real current APIs, fixed-selector completion test in native/test_completion.py; explicit maintained heavy suite. First RED used an empty feature; current private per-engine completion transport nests exact-reader snapshot metadata. No public SDK test accessor. |
| [gd_resources.py](gd_resources.py) | G/D conservative allocated/apparent aggregation, setup/raw reservations and owned child phase guard; selectively adapts the committed Slice60 resource guard. Literal math and actual tiny-child tests in test_gd_resources.py are selected by the existing maintained Python discovery suite. |
| [cells.json](cells.json) | Eight actual P base cells, two balanced repeats:16cells. P1 has engine/SDK passes; P2 is separately filtered marker-write work. |
| [slice30_measure.rs](../../../src/rust/crates/fathomdb-sdk/examples/slice30_measure.rs) | Current engine/SDK direct-text APIs, fresh governed real stores, warmups/raw calls, marker commits/probes, actual compiled-FTS match counts and close/reopen. Build with default SDK features; provider none, no model/GPU. |
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
terminated and waited. Logs/receipts persist under task/logs and task/receipts.
Tiny unit roots/caps exercise enforcement without a heavy/native/model workload;
normal CLI uses the accepted limits and real entry inventory. No tamper/drift or
new product resource API is introduced. Old P artifacts/environments stay read-only.
