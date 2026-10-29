# AC-013 page-cache experiment (W4, dev/0.8.29)

AC-013 (`ac_013_vector_retrieval_latency`, n = 10,000, 768-d) went from
p50 79 ms on `main` to about 123 ms after Slice 40 Unit I removed
`SQLITE_ENABLE_MEMORY_MANAGEMENT` from the vendored `libsqlite3-sys`. This
directory measures the gate at the engine and through the `fathomdb-mcp`
server, and diagnoses the cause with a pre-registered sweep.

- Pre-registration: [PREREG.md](PREREG.md)
- Summary results, every repeat: [results.json](results.json), written by
  `sweep.py summarize`

Raw logs are not committed. Every command below writes its summary line to
stdout or stderr; the sweep driver keeps the logs under its `--out` directory.

## Tools

Run every command from the repository root. All are release builds.

### Engine gate: `scripts/perf-experiments/run-ac013.sh`

The AC-013 gate itself. Needs `--features test-hooks`, which the script now
passes (commit `79693fd2b`; the same fix in `run-ac012.sh`, `run-ac019.sh`
and `.github/workflows/perf-canonical.yml`).

```bash
AC013_SCALE_TREATMENT=warm LOG_PATH=/tmp/ac013.log \
  bash scripts/perf-experiments/run-ac013.sh
```

- Prints `AC013_NUMBERS n= samples= seed_ms= p50_ms= p99_ms=` and, with
  `AC013_SCALE_TREATMENT=warm`, one `AC013_TREATMENT_RECORD` line with
  `seed_write_ms`, `projection_drain_ms` and every raw sample in µs.
- Knobs: `AC013_CORPUS_N`, `AC013_VECTOR_DIM`, `AC013_SCALE_TREATMENT`,
  `LOG_PATH`, and the engine PRAGMA levers below. Exits non-zero when the
  gate fails, after printing the numbers.
- Produced: the engine baseline and every gate cell of the sweep
  (`results.json`, arm `gate`).

### Engine probe: `src/rust/crates/fathomdb-engine/tests/exp_ac013_page_cache.rs`

The gate's fixture (copied generator, same seeds and embedder) plus what the
gate does not print: reader page-cache hits and misses per search, each
reader's `cache_used_bytes`, a text-only pass on the same queries, and seed
write and drain apart. `#[ignore]` and `test-hooks`-gated.

```bash
cargo test --release -p fathomdb-engine --features test-hooks \
  --test exp_ac013_page_cache -- --ignored --nocapture
```

- Prints one `AC013_PROBE {json}` line. Its header comment explains each
  field.
- Knobs: `AC013_CORPUS_N`, `AC013_VECTOR_DIM`, `AC013_SAMPLES`, `AC013_ISO`
  (adds 100 heavy text documents and the hybrid / heavy read / hybrid
  isolation phase), and the PRAGMA levers.
- Produced: misses and hits per search (about 141,000 page requests per
  768-d hybrid search in every cell), text-only cost, store size, RSS
  (arm `probe`), and the AC-013-shaped isolation ratio (arm `probeiso`).

### MCP harness: `src/rust/crates/fathomdb-mcp/examples/ac013_mcp.rs`

The same corpus and queries through the real `fathomdb-mcp` binary over
stdio, with Block 1's queue at its defaults (admission cap 4, queue limit
32, `run_limit_ms` 30 s, call deadline 10 s). It is an example, not a test,
because it drives a separately built server binary, like the LoCoMo
evaluator next to it, and needs no new test surface.

The server opens stores only with the pinned default embedder
(`bge-small-en-v1.5`, 384-d, CPU here), so the MCP store is embedded with it,
not with the gate's 768-d `VaryingEmbedder`, and every search embeds its
query. The `sdk` command runs the same store in-process to split the MCP
cost from the embedder cost.

```bash
cargo build --release -p fathomdb-mcp --features default-embedder \
  --bin fathomdb-mcp --example ac013_mcp
B=target/release
$B/examples/ac013_mcp seed /tmp/ac013-bge.sqlite
$B/examples/ac013_mcp sdk  /tmp/ac013-bge.sqlite /tmp/ac013-sdk
$B/examples/ac013_mcp mcp  /tmp/ac013-bge.sqlite $B/fathomdb-mcp /tmp/ac013-mcp 4,8,16,32 1000
```

- `drain` finishes a store whose seed drain timed out
  (`AC013_DRAIN_TIMEOUT_MS`); on a busy 24-thread CPU, embedding 10,000
  rows took about 34 minutes. It prints the corpus digest too.
- `seed` prints the corpus and query digests, seed write and drain time, and
  the node and vector-row counts. The digests must equal the engine probe's.
- `sdk` prints search and embed-only p50/p99/mean.
- `mcp` prints one record per phase (`c1_warmup`, `c1`, then each level):
  end-to-end p50/p99/mean of successful calls, throughput, errors by
  `class/code/reason`, and the queue wait of the calls that waited, from
  the server's debug log. The file header explains what is and is not
  observable.
- Knobs: `AC013_CORPUS_N`, `AC013_SAMPLES`, `AC013_DRAIN_TIMEOUT_MS`,
  `FATHOMDB_MCP_LOG`, and the PRAGMA levers, which the spawned server
  inherits.
- Produced: the MCP baseline at 1 to 32 in flight (arm `mcpconc`), the
  in-process split of search and query embedding (arm `sdk`), and MCP at 1
  in flight per cell (arm `mcp`). Its digests equal the probe's
  (`481020191c2684bb` corpus, `60c6a8fae6acc4d3` queries).

### Sweep driver: `dev/experiments/ac013-page-cache/sweep.py`

Runs a cell table (`cells.json`: build mode × reader / writer PRAGMAs ×
extra env) against the arms `gate`, `probe`, `probeiso`, `mcp`,
`mcpconc`, `sdk`, `i6` (REQ40-I6 clause i, the slice40 timing report on
the tc5-10000 pack) and `iso` (AC40-96 and AC40-97). It waits for 1-minute
load ≤ `--max-load` before each run, reverses the cell order on odd
repeats, appends one record per run to `<out>/results.jsonl` with the load
before and after, and keeps each run's log. `summarize` groups the records
by cell and arm and adds the paired per-query difference against
A-r2000. Its docstring lists every option. Tests:
`python3 -m pytest dev/experiments/ac013-page-cache/test_sweep.py`.

The runs behind `results.json`, from the repository root, with
`OUT=/tmp/ac013-sweep`, `MM=<restored target>` and
`STORE=<seeded MCP store>`:

```bash
D=dev/experiments/ac013-page-cache
R="--cells $D/cells.json --out $OUT --restored-target $MM --mcp-store $STORE \
  --tc5-inputs ~/.cache/fathomdb-comp/tc5-10000"
python3 $D/sweep.py run $R --arms gate --repeats 2
python3 $D/sweep.py run $R --arms probe --repeats 1 \
  --only A-r2000,R-r2000,A-r8000,R-r8000,A-mmap256,A-r16000,A-r32000,R-r32000,A-r64000,R-r64000,A384-r2000,A384-r32000
python3 $D/sweep.py run $R --arms sdk,mcpconc --repeats 2 --only A-r2000
C=A-mmap256,A-r32000,A-r64000,R-r32000,R-r64000,A-r2000,R-r2000
python3 $D/sweep.py run $R --arms gate --repeats 1 --repeat-offset 2 --only A-r2000,R-r2000,A-r64000
python3 $D/sweep.py run $R --arms i6 --repeats 2 --only $C
python3 $D/sweep.py run $R --arms iso --repeats 1 --only $C
python3 $D/sweep.py run $R --arms probeiso --repeats 1 --only $C
AC013_SAMPLES=500 python3 $D/sweep.py run $R --arms mcp --repeats 2 \
  --only A-r2000,R-r2000,A-mmap256,A-r64000
python3 $D/sweep.py run $R --arms probeiso --repeats 1 --repeat-offset 1 --only R-r32000,A-r2000
python3 $D/sweep.py summarize --out $OUT
```

Before a sweep, build both targets so no run compiles: the `cargo test
--no-run` and `cargo build` lines above, once plain and once with
`CARGO_TARGET_DIR=$MM LIBSQLITE3_FLAGS=-DSQLITE_ENABLE_MEMORY_MANAGEMENT`.

### Engine PRAGMA and build levers

- `FATHOMDB_PERF_EXPERIMENTS=1` with `FATHOMDB_PERF_READER_PRAGMAS` and
  `FATHOMDB_PERF_WRITER_PRAGMAS`: comma-separated `name=value`, applied at
  each connection open (`connection_runtime.rs`).
- `LIBSQLITE3_FLAGS=-DSQLITE_ENABLE_MEMORY_MANAGEMENT` with a separate
  `CARGO_TARGET_DIR` rebuilds the bundled SQLite with the shared page-cache
  group of 0.8.27 and earlier.
