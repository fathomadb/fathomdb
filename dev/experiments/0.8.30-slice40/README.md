# Slice 40 CPU SDK latency study

The [frozen design](../../plans/0.8.30/features/slice-40/design.md) specifies
the input, order, workloads and adoption criteria. This directory selects
those bounded cells through the existing
[Slice 30 adapter](../../../src/rust/crates/fathomdb-sdk/examples/slice30_measure.rs).
The original .29 archive remains read-only. The ordinary timing profile and
the per-request fidelity profile use distinct executable artifacts.

## Native artifacts and selectors

From the checkout root, build the ordinary and diagnostic executables for each
source. Copy each artifact after its build because Cargo reuses the final name.

```bash
cargo build --locked --release -p fathomdb-sdk \
  --features default-embedder --example slice30_measure
cp target/release/examples/slice30_measure "$ordinary"
cargo build --locked --release -p fathomdb-sdk \
  --features default-embedder,slice30-observer,slice50-measurement \
  --example slice30_measure
cp target/release/examples/slice30_measure "$diagnostic"
```

Set `$archive` to the previously released G task's directory and `$model`
to an owned XDG cache with the three checked BGE assets. Select the closed main
file from the accepted 100k seed's `seed` directory, where its archived WAL
has zero bytes. The initial corpus and query paths are in that task's
`inputs` directory. The wrapper verifies the closed subject, query and corpus
digests and compares the assets with the current production loader's pins.
It sets the CPU device and selects the default embedder. Use an absent output
path for each invocation.

```bash
study=dev/experiments/0.8.30-slice40
shared=(
  --seed "$seed" --queries "$archive/inputs/queries.jsonl"
  --corpus "$archive/inputs/corpus.jsonl" --model "$model"
  --ordinary "$ordinary" --diagnostic "$diagnostic"
)
python3 "$study/run_study.py" "${shared[@]}" --stage baseline --out "$baseline_out"
python3 "$study/summarize_study.py" "$baseline_out"
python3 "$study/run_study.py" "${shared[@]}" --stage written-reopen-regression \
  --out "$reopen_out"
python3 "$study/run_study.py" "${shared[@]}" --stage comparison \
  --candidate "$candidate" --candidate-diagnostic "$candidate_diagnostic" \
  --out "$comparison_out"
python3 "$study/summarize_study.py" "$comparison_out" --compare
python3 -m unittest discover -s "$study" -p test_summary.py
```

The wrapper creates independent mutable copies before each matched block.
The ordinary loop counts actual API return and ends after recorder flush.
GNU `time` reports process-wide CPU, peak RSS in KiB, and file-system operation
counters. WAL bytes are a maximum observed in 50 ms samples. SDK responses
have in-process fields. The summary counts returned body UTF-8 bytes and reports
the adapter's JSONL bytes and persistence separately. A missing response stays
missing when the process stops. The written-reopen regression exercises the
real SDK, model, governed commits and current FTS marker cursors on one
written subject across process close and reopen.

## Eligibility and hints diagnostics

Build `eligibility_probe.c` with the same bundled SQLite archive linked into
the ordinary SDK executable. Run it only on an owned copy. Pass the two exact
`SELECT EXISTS` statements as separate arguments, from
[`has_source_dependencies` and `all_nodes_directly_eligible`](../../../src/rust/crates/fathomdb-engine/src/dependency_closure.rs).
The probe runs the baseline, then creates two diagnostic shapes in transactions
that it rolls back. Its runtime plan statements and work counters are
diagnostic evidence. They are independent of the correlated caller spans.

```bash
cc -O2 -Wall -Wextra -Werror "$study/eligibility_probe.c" "$bundled_sqlite" \
  -lm -ldl -lpthread -o "$counter_probe"
"$counter_probe" "$owned_store" "$dependency_sql" "$eligibility_sql"
```

For the hints split, build the existing devloop test with
`cargo test --locked -p fathomdb-mcp --features test-seams --test hints_overhead
--no-run --message-format=json`. Select the `libfathomdb_mcp-*.rlib` from that
build's `compiler-artifact` message. Its features are `default,test-seams`.
Build the companion against that exact library and capture GNU `time` plus
the per-trial diagnostic rows. The existing 200 µs gate remains unchanged.

```bash
rustc --edition=2021 -L dependency=target/debug/deps \
  --extern "fathomdb_mcp=$hints_rlib" "$study/hints_diagnose.rs" -o "$hints_diagnose"
/usr/bin/time -v "$hints_diagnose"
```
