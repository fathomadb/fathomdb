# Vec1bench vector corpora for FathomDB 0.8.28

This registry adds all four datasets in [SQLite's vec1 benchmark](https://sqlite.org/vec1bench.html)
as opt-in FathomDB vector retrieval tests. `datasets.json` pins the source URL,
SHA256, vector count, dimension, source distance, and conversion for each corpus.
No multi-GB dataset or generated database belongs in Git.

| Dataset | Indexed vectors | Dimensions | Evaluation distance |
| --- | ---: | ---: | --- |
| SIFT1M | 1,000,000 | 128 | Euclidean (L2) |
| ImageNet CLIP | 1,281,167 | 512 | L2 on already normalized vectors |
| Landmark DINO | 760,757 | 768 | Cosine, mapped to L2 by unit normalization |
| AG News MXBAI | 769,382 | 1,024 | Euclidean (L2) |

`source_distance` records the HDF5 attribute. In particular, VIBE labels the
ImageNet file `normalized`; [SQLite's measurement](https://sqlite.org/vec1bench.html)
uses L2 for it. Landmark's cosine distance is rank-equivalent to L2 after
unit-normalizing **both** indexed vectors and queries. The converter records
this transformation. It does not infer a model, embed text, or change the gold
IDs. The HDF5 `neighbors` array supplies source-computed exact top-10 neighbors;
row IDs are zero-based positions in `train`.

The SIFT input is an [ANN-Benchmarks-format HDF5 mirror](https://huggingface.co/datasets/hhy3/ann-datasets/blob/0bc1ed67fc2626bc6f110c8794a70572681b7395/sift-128-euclidean.hdf5)
of TEXMEX SIFT1M. It is **not** the original TEXMEX tarball. The other files
are from the [VIBE dataset repository](https://huggingface.co/datasets/vector-index-bench/vibe).
The VIBE listing labels its distributed files CC-BY-4.0; underlying images and
news content may carry separate terms. The SIFT mirror's stated use is
benchmarking and research, with original source terms retained. The benchmark
reads vector embeddings and source gold only; it does not redistribute source
files.

## Acquire and prepare one corpus

Use an external scratch volume with room for the HDF5 source, converted float32
files, and a FathomDB SQLite database. `fetch` does no network I/O without the
explicit `--allow-network` flag. Both acquisition and preparation verify the
registry SHA256; preparation also validates dimensions, data types, distance,
gold shape, neighbor IDs, and distances. Install `h5py` and `numpy` in a
dedicated preparation environment when needed.

```bash
export FATHOMDB_VEC1BENCH_ROOT=/absolute/path/on/large/scratch/vec1bench
python3 -m scripts.vec1bench.fetch \
  --dataset sift-128-euclidean \
  --output-root "$FATHOMDB_VEC1BENCH_ROOT" --allow-network
python3 scripts/vec1bench/prepare.py \
  --dataset sift-128-euclidean \
  --source "$FATHOMDB_VEC1BENCH_ROOT/raw/sift-128-euclidean.hdf5" \
  --output-root "$FATHOMDB_VEC1BENCH_ROOT"
```

Repeat with each other registered dataset ID. Already present source files are
hash-verified without networking. The prepared files (`train.f32`, `test.f32`,
`neighbors10.u32`, `prepared.json`) and the database stay under that external
root. `prepared.json` seals every converted file with SHA256. The Rust tests
verify the seals again before opening the database. Failed or interrupted
preparations cannot produce a new `prepared.json` seal.

## Run the performance tests

Run the full-scale tests in a **release** build and serially to avoid resource
contention. These tests are ignored by ordinary Cargo/CI runs. Invoking an
ignored full-scale test without `FATHOMDB_VEC1BENCH_ROOT`, or with an absent or
corrupt prepared corpus, **fails**. The default tiny real-Engine fixture can
run without a downloaded dataset.

```bash
cargo test --release -p fathomdb-engine --features test-hooks \
  --test vec1bench_corpora -- --ignored --test-threads=1 --nocapture
```

Each test ingests the **entire** train population using a file-backed custom
embedder, waits for every FathomDB vector projection, and searches 100 evenly
spaced held-out queries. It resumes an interrupted ingestion at the next
committed canonical row. Setup times are incremental; the result records how
many rows already existed at test start, so a reused database's near-zero seed
time is never mistaken for a full build measurement. Search uses the existing
vector-stage-only test seam
so FTS and rank fusion cannot inflate or displace vector recall. The regular
search pipeline may still perform an empty FTS lookup, whose overhead remains
inside the measured latency. It runs
separate top-1 and top-10 searches, reporting recall@1 and recall@10 against
the source's exact neighbor IDs, plus p50/p99 latency and raw samples for
each limit. Recall is diagnostic; no unratified quality floor is imposed.

Latency is the warm in-process `Engine::search_with_limit` call: it includes
the in-memory query-vector lookup, FathomDB reader dispatch, bit-KNN, float32
rerank, any empty FTS lookup, and result hydration. It excludes corpus download, ingestion,
projection drain, and gold lookup. Five warmup queries per limit precede the
100 measurements. A JSON result is written next to the prepared corpus. The
benchmarks on sqlite.org used vec1 0.7 on an AMD 5950X; compare numerical
latency only after rerunning both systems on the same host with equivalent
distance and retrieval boundaries. FathomDB's current candidate fanout is its
product default, not a tuned vec1 `K`/`nprobe` sweep.

The four full-scale datasets have **not** been downloaded or measured as part
of adding these tests. Source acquisition is a separate explicit action.
