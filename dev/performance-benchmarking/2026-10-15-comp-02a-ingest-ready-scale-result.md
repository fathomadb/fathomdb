# COMP-02a result — ingest-to-queryable and bounded scale, FathomDB 0.8.27 vs SurrealDB 3.3.0

Run 2026-10-15 under the [side plan](plans/2026-10-15-comp-side-plan.md).
Same host as [COMP-01](2026-10-15-comp-01-nfcorpus-fts-result.md). CPU only.

## Method

- **Inputs:** the qualified SCALE-02 TC-5 pack in pack order, two tiers:
  10,000 and 17,272 real documents, plus the 100 TC-5 queries. Built with
  `experiments/comp02_inputs.py` into `~/.cache/fathomdb-comp/tc5-*`.
- **Arms:**
  - **FathomDB:** the repo 0.8.27 build, Python `Engine`, no embedder,
    `search_text_only(limit=10)`.
  - **SurrealDB:** the 3.3.0 Rust crate, embedded RocksDB, through
    `experiments/comp_drivers/surrealdb`. It uses crud-bench's analyzer
    (`blank,class` + `lowercase,ascii`, BM25, the same crate pin) and the
    native OR match `body @0,OR@ $q ORDER BY search::score(0) DESC LIMIT 10`.
  - This is **our runner using the crud-bench SurrealDB protocol**, not a
    crud-bench run. crud-bench cannot load an external corpus.
  - The FTS index is declared before ingest on both arms.
- **Per cell** (four fresh databases per arm and tier, serial, `AB/BA/BA/AB`
  order):
  - Batched writes of 256 documents.
  - **Ingest-ack:** time to the last acknowledged write.
  - **Build-return:** FathomDB `drain`; SurrealDB `INFO FOR INDEX` reports
    `ready`.
  - **Query-ready:** five probe documents (first, quartiles, last) are each
    returned for their rarest token. The last probe is the late-inserted
    sentinel.
  - Close, reopen, and one first query after reopen.
  - A cold pass of 100 queries, a declared warm-up of 100, then a steady pass
    of 1,000 queries (100 × 10, seeded shuffle).
- **Timing:**
  - FathomDB: the in-process Python call through materialized IDs.
  - SurrealDB: the engine-side `db.query` call through materialized results,
    measured inside the Rust driver. Pipe overhead is excluded and recorded
    separately as `wall_ms`.
- **RSS:**
  - FathomDB: the whole Python cell process, including the loaded corpus.
  - SurrealDB: its engine process high-water mark only.

## Results

Medians over four repetitions. All 16 cells completed. Every probe passed
on the first attempt, the sentinel was returned after reopen in every cell,
and no steady query returned zero hits.

| Metric | FathomDB 10k | SurrealDB 10k | FathomDB 17,272 | SurrealDB 17,272 |
| --- | ---: | ---: | ---: | ---: |
| Ingest-ack (s) | 1.49 | 4.64 | 2.64 | 7.80 |
| True query-ready (s) | 1.50 | 4.64 | 2.65 | 7.80 |
| Reopen (s) | 0.009 | 1.84 | 0.009 | 0.55 |
| First query after reopen (ms) | 0.58 | 2.17 | 0.50 | 2.13 |
| Cold pass p50 (ms) | 3.86 | 153 | 5.72 | 266 |
| Steady p50 (ms) | **4.13** | **149** | **6.92** | **258** |
| Steady p95 (ms) | 12.4 | 166 | 21.8 | 283 |
| Steady p99 (ms) | 16.8 | 179 | 27.3 | 302 |
| Disk (MiB) | 74.6 | 22.8 | 129.6 | 40.8 |
| Peak RSS (MiB) | 163 | 326 | 275 | 450 |

Per-repetition steady p50 ranges:

- FathomDB: 4.07–4.15 ms at 10k and 6.37–13.78 ms at 17,272. Repetition 1,
  the first cell of that tier, measured 13.8 ms; the other three measured
  6.4–7.4 ms.
- SurrealDB: 146.7–151.6 ms at 10k and 254.0–260.8 ms at 17,272.

## Interpretation

- **Readiness.** Both engines are queryable as soon as writes are
  acknowledged, because the FTS index is maintained at write time on both.
  FathomDB reaches query-ready about 3× sooner at both tiers.
- **Steady latency.** FathomDB is about 36× faster at the median at 10k and
  about 37× faster at 17,272. Both grow roughly linearly with corpus size
  across these two points.
- **SurrealDB's latency was checked.** At 10k, on 40 queries:
  - Native OR as measured: p50 142 ms.
  - OR with stop words stripped: 105 ms.
  - The same OR match without the score sort: 1.4 ms.
  - AND match: 0.45 ms, but it returns almost nothing for sentence queries.
  - Single rarest term: 4.1 ms.

  `EXPLAIN` shows `FullTextScan → Compute search::score → SortTopK`, so
  every OR candidate is scored before the top-10 cut. This is the product's
  ranked-OR cost, not a harness artifact. Stop-word removal does not close
  the gap.
- **Resources.** SurrealDB uses about 3.2× less disk. FathomDB's measured RSS
  is lower even though it includes the Python runtime and the loaded corpus.
- **Envelope.** 17,272 documents is the largest tier measured. It is not a
  capacity limit, and nothing here extrapolates beyond it.

## Artifacts

Under `~/.cache/fathomdb-comp/`: `tc5-10000/` and `tc5-17272/` (inputs,
probes, manifests), and `comp02a-10000/` and `comp02a-17272/` (manifest plus
eight cell JSONs each, with per-query ordered IDs and timings). Summarize with
`python -m experiments.comp02_report scale --out <dir> …`.
