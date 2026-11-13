# Slice30 P measured results

Status: P subset accepted after fresh final independent Astra-medium PASS on
clean0d8be5bb and parent integration2381de882. Qualification remains assembled
within the exact subjects/limits below. Whole30/downstream gates stay open.
All measurements below come from the same clean2fd772558e46d0b1645dce2ffea730d0bdd3305c,
not a historical prototype or a different installed artifact.

## Subject and raw evidence

Parent prerequisite/source baseline144bde29b75b31f1ce7277edfb82b5c6d5735068
contains qualified20/60/70. Actual private release executable SHA256 is
bbf336645c192d779705d791326274ab5dd767cdf8b07c33cf24e2b72da62ce6.
Default SDK features, provider none, format37; every store has zero vector rows.
CPU: AMD Ryzen Threadripper PRO5945WX,24logical CPUs; Linux7.0.0-42,
Rust/Cargo1.95.0, Python3.13.13. No model/GPU or host configuration work ran.
Build used jobs1 outside timing. OS cache remains uncontrolled.

All evidence is under
/mnt/data/tmp2/fathomdb-0829-continuation/slice30:

- p-campaign-v1/manifest.json, plan.json and environment.json are the read-only
  prospective source/binary/input/cell/method freeze.
- p-campaign-v1/summary.json is emitted by the committed control.py command.
- p-campaign-v1/results/<cell>/attempt-0001 contains cell.json, child.log,
  store.sqlite, samples/warmups/batches/marker-probes/matches.jsonl and result.json.
- receipts/p-campaign-v1.json records guard command, resource samples, exact
  clean source boundaries and exit0; receipts/p-summary-v1.json records summary.
- receipts/assembled-qualification.json and proofs/p0-final/identity-and-proof.json
  retain qualification attribution and actual normal CLI/native proof.

The100k corpus SHA is
cc5930e525a2af2954353f92dacf720cc4b79f40dbb7965c44bb5c985b5cb481;
500query SHA is0eeb5582fc93bee7e2a494054a55d26bb0940d771083b726f999a9682e74dc16.
Registered raw input/license and actual body-length statistics remain in
[inputs](inputs.md). Binary/corpus/query/cells/lock hashes were checked after the
run and remain identical to the prospective freeze. No input substitution,
regeneration, retry, extra repeat or cell-order change occurred.

## API latency and loop throughput

Each row has1000successful timed natural requests,0errors;500warmups peradapter
remain separate. The20rows total20000timed requests and10000warmups. Timings are
API monotonic nanoseconds, displayed below as milliseconds. RPS includes all
requests over the loop wall, including serialization/checkpoint I/O and P2
committed writes/marker probes. It excludes seed, warmups, open/close and the
untimed match diagnostic phase. Whole-cell elapsed is a different quantity.

| Cell/repeat | Adapter | p50 ms | p99 ms | Mean ms | Loop RPS |
| --- | --- | --- | --- | --- | --- |
| p1-n10000-r0 | engine | 3.773 | 11.605 | 4.272 | 231.506 |
| p1-n10000-r0 | sdk | 3.825 | 11.438 | 4.283 | 231.076 |
| p1-n100000-r0 | engine | 27.184 | 106.911 | 32.939 | 30.311 |
| p1-n100000-r0 | sdk | 27.438 | 107.192 | 33.012 | 30.243 |
| p2-b1-s1-r0 | sdk | 109.588 | 242.690 | 115.201 | 8.534 |
| p2-b1-s10-r0 | sdk | 109.370 | 238.455 | 114.973 | 8.676 |
| p2-b1-s100-r0 | sdk | 109.332 | 241.040 | 114.775 | 8.705 |
| p2-b10-s1-r0 | sdk | 110.138 | 241.701 | 115.418 | 7.720 |
| p2-b10-s10-r0 | sdk | 108.994 | 239.789 | 114.518 | 8.662 |
| p2-b10-s100-r0 | sdk | 109.475 | 239.206 | 114.825 | 8.697 |
| p2-b10-s100-r1 | sdk | 110.402 | 243.068 | 115.824 | 8.622 |
| p2-b10-s10-r1 | sdk | 110.335 | 243.200 | 115.740 | 8.570 |
| p2-b10-s1-r1 | sdk | 109.694 | 241.315 | 115.240 | 7.717 |
| p2-b1-s100-r1 | sdk | 110.106 | 244.965 | 116.134 | 8.603 |
| p2-b1-s10-r1 | sdk | 109.323 | 242.311 | 115.297 | 8.652 |
| p2-b1-s1-r1 | sdk | 108.745 | 240.994 | 114.610 | 8.578 |
| p1-n100000-r1 | sdk | 27.341 | 107.682 | 33.066 | 30.192 |
| p1-n100000-r1 | engine | 27.509 | 107.507 | 33.116 | 30.149 |
| p1-n10000-r1 | sdk | 3.784 | 11.443 | 4.251 | 232.640 |
| p1-n10000-r1 | engine | 3.791 | 11.655 | 4.271 | 231.504 |

All four unfiltered100k P1 passes are the tracked AC083 fixture: p50 is
27.184–27.509ms and p99 is106.911–107.682ms, below the tracked80/300ms targets.
These are descriptive target distances, not new numerical release gates.
P2 includes a kind filter and marker writes/probes; it is not the AC083 fixture.
Its109ms-scale p50 cannot be attributed solely to writes by comparing it with
unfiltered P1; the filter/route and schedule differ. No optimization is selected.

Paired SDK-minus-engine means over2000matched calls/500query identities are
-0.004335ms at10k (descriptive95% interval[-0.010996,0.002726]ms) and
0.011950ms at100k ([-0.009949,0.037272]ms). Fixed seed20261111/2000query-cluster
resamples preserve repeat pairing. These intervals do not establish a treatment
benefit or a general SDK overhead claim. Both repeats remain separately reported.

## Writes, probes and persistence

The table reports actual retained batch/probe records. Every batch is one
existing atomic governed write(batch) call; rows are its cohort size. No claim
about all ancillary SQLite commits follows. Probe/write means use the committed
metrics.distribution over raw elapsed_ns/write_ns, outside natural API timers.
Probe calls intervene between writes and natural searches; no projection-overlap
or pure write→search-without-probes claim follows.

| P2 cell/repeat | Governed calls | Written rows | Marker probes | Mean write ms | Mean probe ms |
| --- | --- | --- | --- | --- | --- |
| p2-b1-s1-r0 | 1000 | 1000 | 1000 | 1.339 | 0.562 |
| p2-b1-s10-r0 | 100 | 100 | 100 | 1.606 | 0.610 |
| p2-b1-s100-r0 | 10 | 10 | 10 | 1.830 | 0.618 |
| p2-b10-s1-r0 | 1000 | 10000 | 10000 | 3.419 | 1.054 |
| p2-b10-s10-r0 | 100 | 1000 | 1000 | 3.360 | 0.516 |
| p2-b10-s100-r0 | 10 | 100 | 100 | 3.377 | 0.458 |
| p2-b10-s100-r1 | 10 | 100 | 100 | 3.459 | 0.463 |
| p2-b10-s10-r1 | 100 | 1000 | 1000 | 3.406 | 0.534 |
| p2-b10-s1-r1 | 1000 | 10000 | 10000 | 3.416 | 1.078 |
| p2-b1-s100-r1 | 10 | 10 | 10 | 1.834 | 0.596 |
| p2-b1-s10-r1 | 100 | 100 | 100 | 1.584 | 0.610 |
| p2-b1-s1-r1 | 1000 | 1000 | 1000 | 1.316 | 0.570 |

All marker_current checks are true; natural/marker active counts are100000/10
in P2 and exactly n/0 in P1. Search snapshot authority remains unavailable;
write receipt cursors do not establish a reused search snapshot.

Every stream appends outside API/write timers and flushes every16records plus
phase/pass boundaries. No fsync/power-loss promise. At most15tail records per
stream are uncheckpointed between completed checkpoints; interruption during a
pending flush can leave that checkpoint partial. Tests preserve actual completed
P1/P2 raw prefixes after owned-child interruption and strict UTF8-prefix issues.
All campaign JSONL files are complete and summary reports0resource/record issues.
Raw persistence below covers warmups, samples, batches, probes and match records;
its checkpoint time is a subset of persistence time, not an additional duration.
Metadata creation/open and record hydration/cloning are outside this subtotal;
timed loop throughput includes their in-loop costs. No overhead subtraction.

| Cell/repeat | Encoded records | Encoded bytes | Checkpoints | Persistence ms | Checkpoint ms |
| --- | --- | --- | --- | --- | --- |
| p1-n10000-r0 | 3500 | 40664384 | 222 | 88.596 | 0.649 |
| p1-n100000-r0 | 3500 | 39202690 | 222 | 94.541 | 0.763 |
| p2-b1-s1-r0 | 4000 | 20106227 | 253 | 66.858 | 1.879 |
| p2-b1-s10-r0 | 2200 | 19656185 | 141 | 70.710 | 0.956 |
| p2-b1-s100-r0 | 2020 | 19616199 | 129 | 68.262 | 0.719 |
| p2-b10-s1-r0 | 13000 | 22779818 | 815 | 94.007 | 11.082 |
| p2-b10-s10-r0 | 3100 | 19915141 | 197 | 69.491 | 1.738 |
| p2-b10-s100-r0 | 2110 | 19634703 | 135 | 75.839 | 0.890 |
| p2-b10-s100-r1 | 2110 | 19634705 | 135 | 70.702 | 0.863 |
| p2-b10-s10-r1 | 3100 | 19915148 | 197 | 69.733 | 1.774 |
| p2-b10-s1-r1 | 13000 | 22780013 | 815 | 88.562 | 10.934 |
| p2-b1-s100-r1 | 2020 | 19616205 | 129 | 70.780 | 0.755 |
| p2-b1-s10-r1 | 2200 | 19656185 | 141 | 66.897 | 0.904 |
| p2-b1-s1-r1 | 4000 | 20106211 | 253 | 69.567 | 1.967 |
| p1-n100000-r1 | 3500 | 39202694 | 222 | 98.395 | 0.739 |
| p1-n10000-r1 | 3500 | 40664364 | 222 | 93.449 | 0.668 |

## Matches, resources and limits

Every cell retains500actual product-compiled FTS count queries. At10k the
min/mean/max raw count is24/3347.352/8287; at100k it is
185/33989.846/84182. Zero-count queries:0. These raw counts are separate from
limit10 returned hits and are not relevance gold. P2 diagnostics filter natural
doc rows. Exact query/row distributions remain in the raw files.

The16cells completed without retry, timeout, resource stop or noisy-cell flag.
Maximum sampled1minute load is2.079102, below the2.5noise threshold; quiet starts
were governed by<=2. Whole campaign elapsed3028.645s includes all cells/waits;
it is not API latency. Start/end SHA and dirty state match exactly and are clean.
Whole P2 cell elapsed224.1–239.5s includes seed/warmup/diagnostics; percell900s
and campaign14400s caps were retained. Final measurement output is below8GiB,
within16GiB. Guard minima: root83.071GB, task622.413GB, MemAvailable24.098GB;
peak owned67.953GB. Actual raw samples/DBs/errors and resource observations stay
under the assigned/mnt/data root; no sibling mutable runtime or target is used.

## Qualification and downstream use

Qualification is **assembled**, not an uninterrupted full-wrapper exit0.
Original clean765 wrapper stopped on SC2312; clean1f continuation completed
lint/type/security and214of216suites, with stale generated-map Pages failures.
Affected14Python/6Rust tests, Clippy/check/Ruff/Pyright pass; the two Pages suites
then pass20/20 and45/45 after legitimate generated inventory refresh. A new
execution-doc MD012/hook failure remains retained; whitespace correction alone
precedes clean2fd Markdown/public-doc checks. The affected run's exact staged
5bmetadata is recorded rather than relabelled clean. Original measurement-suite
passes are replaced by affected-suite proofs; unchanged suites reuse1f inputs.

Original1f inner results remain explicit: Rust406result lines sum2720passed,
67ignored,38filtered (not unique-test counts); Python1750passed/36skipped;
TS629passed/6skipped. Platform pool lint and unconfigured TS lint skips remain.
The1f production Python native receipt stays attributed to1f; relevant production
sources, Cargo/root dependency inputs and bindings are byte-identical at2fd.
The private measurement executable was rebuilt at clean2fd and actually invoked
through its normal CLI on tiny literal P1/P2 stores before timing.

The16genuine grouped new-tool RED/GREEN units and invalid124fixture history are
in [execution](execution.md); no already-correct product was broken to make RED.
Same fresh Astra accepted correction code at5b and light-audited the actual
freeze/first records without actionable findings. Final independent Astra results verdict is PASS on clean0d8be5bb; parent
accepted the P subset at2381de882. These later acceptance facts do not relabel
any runtime/native subject or change the frozen P evidence. Parent owns state.

P gives40/50 a current provider-free direct-text cost/filtered-write baseline,
not a selected cache/eligibility/ANN/default treatment or their source GO.
G/D require concrete extension review and GO. M needs20 A3/A5; Windows20 A6,
exact frozen TC5 fidelity F, whole20/30, pre90 Block1 and final160 remain held.
No final fidelity or whole-slice completion is inferred from this P subset.
