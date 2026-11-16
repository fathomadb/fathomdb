# Slice 30 initial P preregistration

Status: P design accepted; immutable actual campaign freeze still precedes timing.
This is an observation campaign, not performance tuning or a new release gate.
Only P is the initial source/runtime review request. Later groups require the
concrete fit/review boundary in [design](design.md).

## Fixed workload and cells

Pin registered LME raw input/revision/license and the converter output identities
in [inventory](inputs.md). Select first100000 unique nonempty turns; all500questions
repeat twice per timed pass, identical order for every adapter/repetition. Limit10,
current default view, no snippets/explain/graph/rerank/provider/vector projection.
Preserve installed current query compiler, stop words and ranking defaults.
No cache settings/build flags, OS page-cache dropping or host-affinity changes.

| Cell | Records and adapter | Schedule | Warmups/timed calls | Repeats |
| --- | --- | --- | --- | --- |
| P0 | Tiny literal fixture, engine and SDK | Fresh real-store write/text/close/reopen | Untimed correctness only |1peradapter|
| P1 | LME prefix10k and100k, engine then SDK | Read-mostly; no marker rows/filter |500/1000 peradapter |2|
| P2 | LME100k +10marker rows, SDK direct text; kind filter selects natural rows | batch size1 or10;1/10/100 natural searches per committed batch |500beforewrites/1000 timed natural searches |2perconfiguration|

Second repetition reverses adapter order and P2 configuration order. Fresh store
per size/repetition/P2 configuration; adapters in P1 close/reopen the same freshly
seeded store and run identical queries. Report first-open/first-search separately
from steady-state; this is fresh-process or fresh-engine state with **OS cache
uncontrolled**, never a cold-disk claim. Do not time compilation/model loading
as retrieval. P2 batch counts are1000/100/10; marker correctness probes occur
outside the timed natural calls and are separately counted/timed. Batch size1
rotates over ten marker IDs; batch size10 replaces all ten in one existing atomic
governed write(batch) call, not ten independent calls. batch_count counts successful
governed calls; no all-ancillary-SQLite-commit count is claimed. Validate exact
marker content/version immediately after each commit. No concurrent writer in
P; committed interleaving is distinct from future D/G overlap.

Compute actual query match counts with the same compiled FTS expression on the
seeded database via a separate untimed diagnostic. Do not use naive whitespace
OR as the product compiler. Count duplicated/zero-hit queries; report body-byte
and matched-row distributions. Use the current fathomdb-query default compiler with the same default stop-word
configuration and a read-only rusqlite query against the fixture FTS table, with
its natural-kind restriction where present. P contains only active node bodies,
no edges/source dependencies/windows/custom stop words, so report both raw FTS
row counts and product returned-hit counts with that limit stated. No public
introspection API or naive whitespace query is introduced. Missing attribution
cannot silently pass.

## Statistics, noise and attribution

Use monotonic nanosecond measurement, raw samples in microseconds or nanoseconds
without per-sample integer-millisecond truncation. Define nearest-rank p(p)=
sorted[ceil(p*n)-1], arithmetic mean and sample count. Report each repeat,
not only pooled best results.500 distinct queries/1000calls does not create1000
independent query identities. Report per-query paired adapter differences and
bootstrap95% descriptive intervals over the500query IDs (fixed seed20261111,
2000resamples), preserving repetition pairing. Do not claim significance or
adopt a treatment from those intervals. AC083 distance uses unfiltered P1
100k direct text only, tracked80/300ms targets; no target assertion is added.

One owned campaign lock, serialized measurements, no concurrent compilation or
owned heavy campaign. The owner's November15 ruling (steward seq390),
“You’re not going to get a quiet machine. Go with the current environment”,
supersedes CPU quiet-entry requirements throughout the P/G/D/M execution paths,
including inherited continuation text. Record actual load/CPU/disk/memory
before/during/after. CPU load is descriptive: it cannot cause entry refusal,
waiting, a failed assessment or result disqualification. Load>2.5 still flags a
noisy cell; preserve every observation and result. Permit one additional balanced
repeat only if needed to interpret an apparent target miss or adapter difference,
within existing caps. This is not retry-until-green. Memory/disk/swap floors,
time/growth/shared-budget caps, cgroup/owned-actor containment, exclusive GPU-ID
locks and actual GPU noninterference checks remain unchanged. Numerical targets
and acceptance populations remain unchanged.
No filtering out slow calls/errors/outliers. Error latencies/counts are separate
from successful-call percentiles; all requests remain in throughput denominators.

## Raw output checkpoints

Each raw stream appends outside API/write timers and flushes every16records plus
warmup/pass/diagnostic boundaries. No fsync or power-loss durability claim. The
<=15uncheckpointed-tail bound applies between completed checkpoints; interruption
during a pending flush can leave that checkpoint partial. Record encoded bytes,
records, successful checkpoint calls/time and total persistence time. Pass
persistence totals include warmups; timed throughput excludes warmups and includes
timed serialization/checkpoints, plus P2 committed writes/marker proof queries.
Shared CPU/cache/I/O influence remains; do not subtract this overhead from latency.
Strict per-line UTF8 decoding preserves complete JSONL prefixes, reports invalid
or truncated final lines, and retains failed/incomplete cell disposition.

## Resource and stop rules

All mutable resources live under own/mnt/data checkout/task root, including
TMPDIR, AGENT_LOG_DIR, Cargo home/target, Python venv, npm/pip/UV/XDG/model caches,
inputs/DB/WAL/output/tools. Installed immutable tools are read-only inputs.
No sibling venv/native/module/target/cache borrowing; no HOME reassignment.
Setup follows current owned scripts after GO, with exact pins and own source-
bound noneditable production artifacts; failure of own preflight stops setup.

Reuse reviewed10 start floors: root>=32GiB, taskfs>=250GiB,
MemAvailable>=16GiB; stop owned child phase at root<20GiB/taskfs<50GiB/
MemAvailable<8GiB/owned total>=200GiB or sustained swap>=64MiB/s for three30s
samples. Cap active DB+WAL+raw measurement outputs16GiB, each measurement cell
15minutes, total timing campaign4hours. Setup/full gate outside timing but
inside storage/memory monitoring. One native phase, build jobs2 and canonical
verification job settings. Poll resources at30s; record PID/process group,
command,cwd,SHA/delta and actual exit. Stop only owned child groups; preserve
partial output, no deletion/cleanup of other resources or automatic budget growth.
If sizes/time make a cell infeasible, report INCOMPLETE and revise the exact
proposal from observed evidence before expanding. No smaller fixture quietly
qualifies100k, no unexecuted future matrix is marked complete.

## Review and final source qualification

Scoped checks per behavior; final required agent-verify once on final clean
stable code tree, no edits while it runs. Retain failed attempts, diagnostics,
skips and affected resumes honestly. Fresh final independent Astra review and
parent integration gate the qualified partial. No CI dispatch/workflow execution,
registry publication, paid model API, new download, fault/reset/VM operation or
GPU workload occurs in initial P. G later reserves3090 and proves engagement;
its GPU requirement is not imposed on provider-free P.

## G/D prospective extension

The P preregistration and executed freeze above remain unchanged. The
[G/D continuation](continuation-gd.md) preregisters separate G/D routes, bounded observer
controls, staged cell order and feasibility refusal before larger cells. It
requests design review only; source/runtime GO and actual evidence are separate.
