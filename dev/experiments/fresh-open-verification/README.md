# Fresh verification cost and stored embedder metadata

Owner-requested measurement, 2026-10-31. Compare fresh verification on every
relevant open with the existing cached verdict. This does not change production
verification cadence, failure policy, thresholds or store format.

## Method fixed before measurement

Use the real pinned BGE model and **current default Mean pooling**, not the CLS
pooling used by historical U3 calibration. Mean is the settled production
choice: [accepted ADR](../../adr/ADR-0.6.0-default-embedder.md) and
[IR-C closeout](../../plans/runs/IR-C-retrieval-findings.md). The recorded A/B
compared Mean/no-prefix with CLS plus a query prefix; it was not a clean
pooling-only causal experiment. This timing run does not reopen that decision. Build in release mode. Run strict
`cuda:0` on an available RTX 3090 and `cpu` in separate processes. No fallback,
driver/toolkit change, cache flush or production-store writes. The runner first
checks all three cached assets against pins parsed from the current loader
source and refuses missing/corrupt assets. Every model construction asserts
zero downloaded bytes and exactly three loader cache hits; post-run hashes
must match. Cargo offline alone does not disable model fetching: the cache
preflight and observed loader outcomes establish this run's warm-cache scope.

The ignored test `slice50_fresh_open_cost.rs` seeds 300 distinct canonical
records in a temporary real store, drains them, checks that the 384-dimensional
mean is pinned, and establishes 45 references and the accepted cache marker.
Each timed open uses a private copy of the closed template database. Fresh
cells remove only the verdict marker before timing; cached cells keep it.
Assertions require 45 actual probe embeddings for fresh and zero for cached,
accepted verification, the same resulting marker, and zero embeddings for a
vector-less control. The wrapper delegates real batched embedding unchanged.

Two lifecycles, ten pairs each, with alternating fresh/cached order:

- `reuse`: keep a real provider loaded; isolate engine-open verification cost.
- `reload`: construct a new provider through the pinned loader for each open;
  measure model construction, engine open, close and provider drop separately.
  The instrument follows `CandleBgeEmbedder::new`'s exact resolve/load/construct
  sequence so it can assert loader outcomes. The fixture provider remains
  co-resident, so totals do not measure last-provider pool release/trim.
  This is a warm-process model reload, not cold-process startup: the fixture
  provider remains resident, weights/filesystem and CUDA initialization are warm.

Warm up both cells once per lifecycle, excluding warmups from the samples.
Fresh preparation performs a small SQLite write/checkpoint; cached preparation
reads only. The primary decision metric is paired `open_ms`, a warm end-to-end
open cost, rather than pure inference time. Store copy, SQL cache invalidation,
final marker inspection and compilation are
outside timed intervals. Record all samples and paired fresh-minus-cached
min/median/p95, not just a favorable repetition. No owner latency budget was
specified, so this is characterization, not an invented pass/fail threshold.
Device availability and machine load are recorded around each process; they
are not proof that no unrelated activity happened between those observations.

## Tools and rerun

- `src/rust/crates/fathomdb-engine/tests/slice50_fresh_open_cost.rs`: real-model
  measurement with counting wrapper and instrument assertions; ignored by default.
- `run.py`: strict device processes, provenance and raw output, completeness
  checks and paired analysis in `results.json`.

On this host, the RTX 3090 is compute capability 8.6. Build without changing
`/usr/local/cuda` or the installed driver:

```sh
PATH=/usr/local/cuda-12.6/bin:$PATH \
CUDA_PATH=/usr/local/cuda-12.6 \
LIBRARY_PATH=/usr/local/cuda-12.6/targets/x86_64-linux/lib \
CUDA_COMPUTE_CAP=86 \
cargo test --offline --release -p fathomdb-engine \
  --features test-hooks,embed-cuda --test slice50_fresh_open_cost --no-run
```

The runner inherits the same build variables. Use a new output directory:

```sh
python3 dev/experiments/fresh-open-verification/run.py \
  --out /tmp/fathomdb-fresh-open-new --pairs 10 --devices cuda:0 cpu
```

`FRESH_OPEN_PAIRS` and `FRESH_OPEN_ROWS` also parameterize the Rust instrument
(default ten pairs and 300 rows; at least 256 rows for a pinned mean).
Compilation with CUDA requires driver/device visibility for model execution;
a restricted executor failing `nvidia-smi` is not evidence that the host GPU is
unavailable. Device access must be explicit, never silently downgraded to CPU.

## Inspecting a store's metadata

There is no `dbo.system` view; these are internal SQLite tables. Use a read-only
SQLite connection without ordinary Engine admission:

```sh
sqlite3 -readonly /path/to/store.sqlite
```

```sql
SELECT profile, name, revision, dimension,
       mean_vec IS NOT NULL AS mean_pinned,
       length(mean_vec) AS mean_bytes
FROM _fathomdb_embedder_profiles;

SELECT kind, profile FROM _fathomdb_vector_kinds ORDER BY kind;

SELECT embedder_name, embedder_revision, dim, count(*) AS reference_count
FROM _fathomdb_embed_probe
GROUP BY embedder_name, embedder_revision, dim;

SELECT value AS cached_verification_fingerprint
FROM _fathomdb_open_state
WHERE key = 'vector_equivalence_verified_fingerprint';
```

The stored profile declares name/revision/dimension; nullable `mean_vec` stores
the centering vector (1536 bytes for 384 f32 dimensions), not the Mean/CLS
pooling selection. References contain
45 probe texts and uncentered f32 vectors, captured under that identity. The
cache marker hashes declared identity, stored mean/references, probe fixture,
thresholds and recipe. It does not identify the full running implementation.

`fathomdb doctor dump-profile --json /path/to/store.sqlite` is an existing
convenience command for identity/dimension/vectorized kinds. It currently uses
ordinary Engine admission, which can perform startup maintenance and has the
custom-profile/no-provider limitation described in the policy proposal.

CPU/GPU architecture, device UUID, Candle/CUDA/driver/library versions and
pooling choice are not pinned per store or per vector. Per-open device and
loader information in `OpenReport` is not a durable vector-writer history.
Profile metadata can exist before any provider runs; references can be captured
on a later open. Neither is an audit proof of every vector's generating runtime.

Default model identity: `fathomdb-bge-small-en-v1.5`, HF revision
`5c38ec7c405ec4b44b94cc5a9bb96e735b38267a`, dimension 384. The loader separately
pins/checks SHA-256 of configuration, tokenizer and model weight files in code;
these file hashes are not persisted per database. See embedder `loader.rs:50–79`.

A concrete identity gap is `CandleBgeEmbedder::with_pooling`: Mean/CLS can change
without changing the identity triple, despite incompatible vectors (documented
at `candle_bge.rs:692–697`). Current default construction uses Mean. This is a
reason to retain numerical verification and investigate a fuller embedding
function identity; it is not proof that a production store suffered such a swap.

## Results

[Findings and recommendation](findings.md) report all 80 measured opens and
recover the settled pooling decision. [Raw data](results/results.json) distinguish
verification cost from model construction. These runs do not establish CPU/CUDA
numeric equivalence or a fleet mismatch probability. Cache policy remains
pending owner review of the measured startup costs.
