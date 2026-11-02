# Slice 60 bare-vec0 scale benchmark

This report-only experiment compares 64-, 128- and 256-row chunks using the
shipping rusqlite / sqlite-vec dependencies. It isolates the vector table:
no embedder inference, eligibility scan, FTS, provider, or projection worker.
It does not replace AC-013 or real-bge eu7 recall qualification.

Run from the release checkout, with no concurrent performance jobs:

```sh
cargo run --release -p fathomdb-engine --example slice60_vec0_scale > /tmp/slice60-vec0-scale.jsonl
cargo test -p fathomdb-engine --example slice60_vec0_scale
```

Defaults: dimensions 384 and 768; 10k, 100k, 300k, 1M and 2M vector rows;
chunks 64, 128 and 256; 100 fixed queries per cell. A smoke run is:

```sh
S60_COUNTS=1000 S60_DIMENSIONS=384 S60_SAMPLES=5 cargo run --release -p fathomdb-engine --example slice60_vec0_scale
```

`S60_COUNTS`, `S60_DIMENSIONS`, `S60_CHUNKS` are comma-separated overrides;
`S60_SAMPLES` is a positive count. Every cell uses a private temporary database
and drops it after reporting, so peak scratch consumption is one layout. Set
`TMPDIR` to a volume with sufficient free space (reserve at least 12 GB for
2M rows at 768 dimensions). Do not delete any other process's scratch files.
A failed allocation or query fails the run; it is never a zero-latency sample.

The deterministic vector generator uses the prototype's FNV-seeded xorshift
and L2 normalization, applied to `corpus:<row>` and `query:<index>` strings.
The same row prefix and queries are regenerated for every layout; vectors
are synthetic, not bge output. This isolates physical cost, not real-corpus
recall. Quantization uses original vectors without a corpus mean.

Each table has the production f32, bit, partition and metadata columns.
Connections use a 2 MiB cache, disabled mmap, 4096-byte pages, FULL synchronous
writes and DELETE journaling. Seed time includes vector generation and commits
in batches of 10k. Disk bytes describe the committed main file. One complete
query pass warms the connection before measurement; OS cache is not flushed.

`total_ms` times the shipping-shaped CTE: 192 Hamming candidates followed by
original-f32 L2 rerank, consuming all 192 results as production does. The
reported top 10 is selected after timing. Query vectors use JSON text and two
bindings, matching the production query representation. Diagnostic bit-KNN and rerank measurements
run separately immediately afterward. The rerank diagnostic uses one prepared
point lookup per candidate plus a Rust sort, so its timing is an attribution
probe, not an exact decomposition of the CTE. Do not sum diagnostic timings
or present them as the production total. All summaries use nearest-rank p50
and p95 over every measured query.

Each JSON line includes full candidate IDs/distances and top-10 IDs/distances.
Layouts must have identical candidates strictly below the Hamming cutoff.
Only membership tied at that cutoff may change; the harness fails on other
membership differences and reports tie-dependent candidate/top-10 differences
without claiming exact identity. Exact L2 ties may reorder hits. The split
path's sorted top-10 distances must match its CTE result on each query.

The output also reports record equivalents at 1 and 5.1 vectors per record.
For example, 2M vector rows represent about 392k LongMemEval-shaped records
under future overflow chunking. These are size equivalents, not a benchmark
of unimplemented chunk grouping. Interpret measured crossings only within
the sampled range. Run each full sweep serially, preserve its raw JSON outside
git, and report all cells and any failures; do not change query counts or
choose only faster repetitions after observing results.

This sweep does not include the prior 1024-row layout. It neither confirms
nor refutes the old extrapolated CS64/1024 crossing near 1.2M vectors.
