# Cross-SDK synthetic experiment file

Copy [`synthetic-fts.example.json`](synthetic-fts.example.json),
[`synthetic-fixture.json`](synthetic-fixture.json), and one
[`arm.example.json`](arm.example.json) per SDK into a private directory outside
this repository. Replace each absolute artifact path and SHA-256 placeholder
with the local file and its digest. The fixed driver sources are
`experiments/sdk_drivers/python_driver.py`,
`experiments/sdk_drivers/typescript_driver.mjs`, and the separately built
`experiments/sdk_drivers/rust` executable. The Rust crate's
`build_manifest.py` produces its build attestation. Keep the resolved plan and
arm manifests unchanged between attempts.

Each arm also pins `wrapper_sha256`. Obtain it with
`python -c 'from experiments.sdk_benchmark import wrapper_sha256; print(wrapper_sha256("python", native_path="/absolute/path/to/_fathomdb.abi3.so"))'`
or substitute `typescript` (which hashes the fixed repository wrapper); Rust
uses the native binary SHA-256 as its wrapper
digest. The runner rejects wrapper drift and inherited `FATHOMDB_*` environment
overrides, except the test harness's `FATHOMDB_BENCH_NODE` selector.
Latency cases require exact ordered-ID oracles in the fixture, keyed by
operation and query. Relevance `gold` is separate: a retrieval run can score
poorly without being mislabeled as a binding failure.

The suite file owns the corpus, cases, fixed validity instant, arm artifacts,
cache state, repetition count, order, and requested metrics. The command only
selects that file and a **new** external output root:

```bash
python -m experiments.sdk_benchmark validate /absolute/private/suite.json /absolute/new-output
python -m experiments.sdk_benchmark run /absolute/private/suite.json /absolute/new-output
python -m experiments.sdk_scoreboard /absolute/private/suite.json /absolute/new-output/suite-manifest.json experiments
```

The runner writes one external result, execution config, witness, and raw
sample set per cell. It writes one path-free `experiments.record.v1` receipt
and index row per cell. Re-run the same immutable suite file into another new
external root; the suite attempt and run IDs change while treatment hashes
stay fixed. The synthetic output is descriptive. It does not establish a
performance winner or a registered Program campaign. A future Program run
must use a registered track and its controls.

`answer` cases additionally bind an `answer_gold` artifact and a preceding
retrieval case. The implemented local `first_sentence_v1` answerer is a
repeatable pipeline fixture, not a model-quality benchmark. Its configuration
and gold schema are exercised in `tests/experiments/test_sdk_benchmark.py`.
