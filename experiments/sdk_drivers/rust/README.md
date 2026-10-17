# Rust SDK benchmark driver

This standalone, non-publishable crate measures the public `fathomdb-sdk`
`Engine::search_text_only`, `read::get`, and `graph::neighbors` calls. It is
outside the Rust workspace and has its own lockfile. The executable accepts
exactly one argument: the path to one
`sdk.driver.v1` JSON request. It writes one JSON response to standard output.
Diagnostics belong on standard error.

The two request modes are `capabilities` and `run-repetition`. The latter is
one process and one independent repetition. Its `queries` list uses `{id,
text}` objects. The required `arguments` are `limit` and `view.valid_as_of`
(epoch seconds) for lexical FTS, `view.valid_as_of` for `read.get`, and `depth`
(1–3) plus `view.valid_as_of` for `graph.neighbors`. Read and graph query text
is the logical ID. The request also includes `sdk: "rust"`, one registered
`operation_id`, `operation_version: 1`, an absolute
`database_path`, `warmup_per_query`, `timeout_ms`, and the expected SHA-256 of
this exact executable. The response contains ordered prefixed hit IDs and
nanosecond samples. A failed call, close, or attestation returns `blocked`
with no samples.

The coordinator must prepare a fresh database through
`experiments.fathomdb_test_setup.prepare_test_database`, verify its fixture and
doctor evidence, and enforce a process timeout. The driver checks the binary
digest, requires an existing database, configures SQLite before opening it,
drains, performs declared warmups, measures each public SDK call with
`Instant`, and closes normally. Model features are disabled by default. Do not
infer feature equivalence to Python or TypeScript from a successful Rust result.

After committing the source, build and write an external attestation:

```sh
python3 experiments/sdk_drivers/rust/build_manifest.py \
  --profile release --output /absolute/external/path/rust-build.json
```

The build helper requires a clean checkout and records the source revision,
compiler, target, profile, feature set, `RUSTFLAGS`, reviewed patch sources,
resolved dependency closure, lockfile digest, and executable digest. It checks
the standalone patches against the repository root before building. Pass
`--features` only for an explicitly registered model treatment. Compare the
resolved closure and engine source revision with the other SDK arms before
scoring a paired cell.
