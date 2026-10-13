# Competitive experiments and runner

Copy [`experiment.example.json`](experiment.example.json) for a repeatable local
experiment. This is one versioned input file: `run` contains the product,
workload, metric family, measurement boundary, target hardware, and settings;
`adapter` selects a reusable Python source file by stable ID, path, digest, and
timeout;
`inputs` pins the corpus, query, and optional gold files by absolute path and
SHA-256; `output.metrics` pins the exact aggregate metric names a complete
result must contain. Retrieval and answer experiments require gold. The runner verifies
these identities before execution, in its worker, and after execution. A config may be
run repeatedly without changing it; give each attempt a new external output
directory. Set `program_track` to its registered track ID for a PROGRAM run;
use `null` for an experiment outside that program.

An adapter implements `run(definition, inputs, artifact_root) -> dict` under
`competitive.adapter.v1`. `inputs` maps names to resolved `Path` objects. It
does the benchmark work and returns one `competitive.result.v1` object. The
adapter writes its raw data and a summary artifact under `artifact_root`; the
result's `source.locator` is the absolute path to that artifact and
`source.sha256` is its byte digest. The runner invokes the same protocol for
every adapter. The input file contains no command line or shell fragment.

The included [`fathomdb-fts` adapter](../../adapters/fathomdb_fts.py) measures
the Python `Engine.search` FTS path on a fresh database. Its external corpus is
a JSON array of `{ "id": "...", "body": "..." }` objects; queries are a JSON
array of strings. It supports CPU, no filter, `top_k: 10`, concurrency 1, and
steady cache state. Settings pin the absolute `cli_path`, its `cli_sha256`, and
`embed_device`/`rerank_device` as `cpu`. It creates a fresh database through
`fathomdb_test_setup`, warms each query once, uses the configured seed to order
measured queries, and records per-query latencies, hit counts, CLI and native
binding digests, and doctor digest in an external summary. Latency percentiles
use nearest rank; query throughput uses the measured serial loop wall time.
Its results are `context_only` until a paired comparison protocol qualifies the
workload and hardware.

```bash
python -m experiments.competitive_runner preflight EXPERIMENT.json --output-root /external/new-attempt
python -m experiments.competitive_runner run EXPERIMENT.json --output-root /external/new-attempt
python -m experiments.competitive_run render
```

Preflight reads and hashes inputs without creating output. `run` creates the
new external attempt directory, writes a resolved plan, captures adapter
stdout/stderr there, and validates the adapter result and source artifact. It
then writes `result.json`, `execution.json`, the standard `experiments.record.v1`
receipt, and one index row. The receipt captures the safe aggregate and the
`competitive.execution.v1` witness: config/plan/runner/adapter/input/result
digests, exit status, duration, and host identity. Failed, timed-out, or
invalid adapter attempts produce blocked results with no metrics. A complete
local result requires a runner witness; a published reference remains context
only.
The generated scoreboard shows the configuration, full plan, and adapter
digests so rows with different execution code are visibly distinct.

[`run.example.json`](run.example.json) is the standalone definition shape;
[`result.example.json`](result.example.json) is a blocked aggregate example.
All three example files contain placeholder paths, hashes, and hardware; replace
them before preflight or execution. They are not benchmark evidence.

Use one case per product, workload, scoreboard, and timing setup.

The definition pins the product and embedded deployment, evidence source,
workload and input digests, measurement boundary, intended reference hardware,
and product settings. Settings are a flat map of scalar tuning values, short
identifiers, and local runtime paths; use dotted names to describe nested product options. Secret keys,
raw content fields, nested values, and free-form text are rejected. Keep API
credentials in local secret storage and prompts, queries, predictions, and logs
in external artifacts. `workload.queries` is the number of measured queries per
repetition; a complete result records that count times `measurement.repetitions`.
`evidence.kind` is `local` or `published`. A published case requires an HTTPS
`reference_url`; missing published corpus/query digests are
allowed, but its result must be `context_only`. A local case may be marked
`candidate_match`; that only makes it eligible for a later paired comparison,
and the renderer does not calculate cross-product deltas.

An adapter returns a content-free `competitive.result.v1` JSON object with these
fields: `schema_version`, `case_id`, `config_sha256`, `status`, `observed`,
`metrics`, `source`, `comparability`, and `reason`. The observed object records
the actual product version, CPU, core count, RAM, storage, GPU, resolved
settings, workload, measurement settings, completed queries, and errors. The
observed workload and measurement settings must match the run definition. The
source object records its URL (or `null` for local runs), an external artifact
locator, and the SHA-256 of the external result artifact or published source
snapshot. `metrics` contains finite aggregate values for the
selected scoreboard. Blocked results have no metrics and a reason; a run blocked
before measurement may have `observed: null`.

The validator rejects unknown fields, hash and setting drift, wrong metric
families, incomplete results marked complete, and published rows marked as
matched measurements. Raw queries, predictions, logs, and database files stay
outside the repository. The existing `experiments.record.v1` envelope stores
the resolved definition and validated aggregate result.

```bash
python -m experiments.competitive_run validate RUN.json
python -m experiments.competitive_run validate RUN.json RESULT.json
python -m experiments.competitive_run record RUN.json RESULT.json
```

`record` appends one experiment index row and writes the standard receipt under
`experiments/runs/`. `render` generates `experiments/COMPETITIVE-SCOREBOARDS.md`
from those receipts. `record` is for published references and blocked results;
the runner records completed local measurements. Neither `record` nor `render`
executes a benchmark or acquires data.
The runner derives a unique attempt ID from each external output directory, so
same-minute repeats retain separate receipts while sharing one definition hash.
Manual `record` calls with the same definition within one UTC minute still
refuse a collision.
The [performance program](../../../dev/performance-benchmarking/PROGRAM.md)
and [Track Runner](../../../dev/performance-benchmarking/TRACK-RUNNER.md)
continue to govern live runs under that program.
