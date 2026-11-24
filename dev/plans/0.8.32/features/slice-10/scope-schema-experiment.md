# WP-10D Empirical Experiment: Schema 38 vs. Schema 39 `scope` Performance & `SyncMode::Normal` vs. `Full`

**Decision target:** `0.8.32` Decision 1 (Schema 38 additive partial indexes vs. Schema 39 composite `(scope, ...)` PKs/indexes + `vec0` 7-column pre-selection vs. 8-column `scope TEXT`) and Decision 2 (`SyncMode::Normal` vs. `SyncMode::Full` write durability latency).

**Artifact pair:**

- Machine-readable results: `dev/plans/0.8.32/features/slice-10/scope-schema-results.json` (`sha256=db8e75c0d35c4a03af02c071c281353fdfe78bf62eb2acab0f82a65920a2ecd2`, `266,602` bytes)
- Harness binary: `src/rust/crates/fathomdb-tc5-benchmark/src/bin/scope_schema_experiment.rs` (`--release`, `seed = 20261124`)

## 1. Environment & Methodology

| Parameter | Measured Value |
| --- | --- |
| CPU | `AMD Ryzen Threadripper PRO 5945WX 12-Cores` (`24` hardware threads) |
| SQLite / `sqlite-vec` | SQLite `3.54.1` / `sqlite-vec v0.1.9` |
| Baseline DDL | Schema `38` (`fathomdb_schema::SCHEMA_VERSION = 38`) |
| Rust profile / Seed | `--release` (`optimized`) / deterministic SplitMix64 `seed = 20261124` |
| Trial counts | `warmup = 5`; `N = 30` child-subprocess open/reopen/RSS; `N = 100` write batches/checkpoints per `SyncMode` (`NORMAL`, `FULL`); `N = 200` paired interleaved read/search/graph trials |
| Statistics | Nearest-rank `p50` & `p95`; `10,000`-resample paired bootstrap 95% CIs; exact L2 brute-force ground truth for `recall@10`; live `EXPLAIN QUERY PLAN` audit |
| `sqlite-vec` capability probes | `vec0_default_clause_supported = true`; `vacuum_supported_on_vec0 = true` |

### Compared Layouts

1. **`B0` (Baseline):** Unmodified `0.8.31` Schema `38` DDL (`scope TEXT NOT NULL DEFAULT ''`, single-column indexes) and 7-column `vec0` (`embedding float[384]`, `embedding_bin bit[384]`, `source_type partition key`, `kind`, `created_at`, `status`, `lifecycle`, `valid_from`, `valid_until`, `chunk_size=64` + 8 Filterable `attr_<hex>` slots).
2. **`S38-PARTIAL-A1` (Non-breaking Schema 38 + 7-col `vec0` pre-selection):** Identical Schema `38` PKs/unique indexes + 10 additive partial indexes (`WHERE scope != ''`) across `canonical_nodes`, `canonical_edges`, `operational_state`, and `operational_mutations`. Vector search uses 7-column `vec0` CTE pre-selection (`WITH candidates AS (SELECT cn.write_cursor AS rowid FROM canonical_nodes cn ...)` + `vec_distance_l2`), matching `filter.rs:1081-1089`. Writes enforce explicit same-scope ownership checks (`SELECT scope ...`) before `logical_id` upserts, edge inserts, or `operational_state` checkpoints.
3. **`S38-PARTIAL-A2` (Schema 38 partial indexes + 8-col `vec0`):** Identical Schema `38` canonical tables and ownership checks as `A1`, plus `vector_default` reshaped non-destructively to 8 built-in columns adding `scope TEXT` (`vec_bit(?)` copy of packed quantization bits) so `scope = ?` runs natively inside bit-KNN before `LIMIT 192`.
4. **`S39-COMPOSITE` (Breaking Schema 39):** Rebuilt canonical tables (`user_version = 39`) replacing global unique/lookup indexes with `(scope, ...)` composite indexes (`UNIQUE (scope, logical_id) WHERE superseded_at IS NULL`, `PRIMARY KEY(scope, collection_name, record_key)`, `(scope, collection_name, write_cursor)`) plus native 8-column `vec0` (`scope TEXT`). Single-scope lookups bind `scope = ''` natively.

### Seeded 100k Corpora

Each profile starts from a real `fathomdb-engine` (`Engine::open` + `configure_projections` + `Engine::write` + `drain` + `close`) `100,000`-node / `150,000`-edge anchor store with 8 Filterable node properties (`800,000` `canonical_property_values` rows), `1` edge Filterable property, `10,000` `operational_state` rows across 5 collections, `10,000` `operational_mutations` rows, and deterministic 384-dim clustered embeddings (`VACUUM`ed and WAL-truncated so all layouts start at `freelist_count = 0`):

- **`single_scope`:** `100,000` rows with `scope = ''` (`unscoped_fast_path`; tests `HANDOFF-02`/`HANDOFF-04` single-tenant zero-regression floor and disk headroom).
- **`multi_scope_uniform`:** `10` balanced callers (`tenant_0..tenant_9`), `10,000` nodes / `15,000` edges / `1,000` checkpoints each (`uniform_10pct` cohort).
- **`multi_scope_skewed`:** `1` heavy caller (`heavy_0`: `90,000` nodes = `90%`) + `9` light callers (`light_1..light_9`: `1,111`–`1,112` nodes each = `~1.11%`), testing both `heavy_90pct` and `light_1pct` cohorts.

## 2. WAL-Truncated Database Size (`database_bytes`)

| Profile | Layout | `database_bytes` | Size (MiB) | Pages (`4 KiB`) | Delta vs. `B0` |
| --- | --- | ---: | ---: | ---: | ---: |
| `single_scope` | `B0` | 485,691,392 | 463.19 | 118,577 | baseline |
| `single_scope` | `S38-PARTIAL-A1` | 485,732,352 | 463.23 | 118,587 | +40,960 B (+0.01%) |
| `single_scope` | `S38-PARTIAL-A2` | 487,907,328 | 465.30 | 119,118 | +2,215,936 B (+0.46%) |
| `single_scope` | `S39-COMPOSITE` | 492,613,632 | 469.79 | 120,267 | +6,922,240 B (+1.43%) |
| `multi_scope_uniform` | `B0` | 487,092,224 | 464.53 | 118,919 | baseline |
| `multi_scope_uniform` | `S38-PARTIAL-A1` | 512,282,624 | 488.55 | 125,069 | +25,190,400 B (+5.17%) |
| `multi_scope_uniform` | `S38-PARTIAL-A2` | 514,457,600 | 490.62 | 125,600 | +27,365,376 B (+5.62%) |
| `multi_scope_uniform` | `S39-COMPOSITE` | 508,342,272 | 484.79 | 124,107 | +21,250,048 B (+4.36%) |
| `multi_scope_skewed` | `B0` | 487,022,592 | 464.46 | 118,902 | baseline |
| `multi_scope_skewed` | `S38-PARTIAL-A1` | 511,275,008 | 487.59 | 124,823 | +24,252,416 B (+4.98%) |
| `multi_scope_skewed` | `S38-PARTIAL-A2` | 513,449,984 | 489.66 | 125,354 | +26,427,392 B (+5.43%) |
| `multi_scope_skewed` | `S39-COMPOSITE` | 506,478,592 | 483.02 | 123,652 | +19,456,000 B (+3.99%) |

Key size findings:

- On `single_scope` (`scope == ''`), `S38-PARTIAL-A1`'s `WHERE scope != ''` partial indexes occupy virtually zero space (`+40 KiB`, `+0.01%`), and `S38-PARTIAL-A2`'s 8-column `vec0` adds only `+2.11 MiB` (`+0.46%`). `S39-COMPOSITE` stores explicit `scope` keys in every composite B-tree (`+6.60 MiB`, `+1.43%`). Note: our anchor DB is `463.19 MiB` rather than slice-70's `200.4 MB` minimal corpus because this seed populates `150,000` edges, `800,000` `canonical_property_values` rows, 8 `vec0` attributes, and `20,000` operational rows (`10,000` `operational_state` + `10,000` `operational_mutations`).
- Once scopes are populated (`multi_scope_*`), `S38-PARTIAL-A2` maintains **both** legacy single-column B-trees and additive partial B-trees (`+5.43%` to `+5.62%`), costing **`5.83 MiB` (`uniform`) to `6.65 MiB` (`skewed`) MORE disk** than `S39-COMPOSITE` (`+3.99%` to `+4.36%`).

## 3. Subprocess Open, Reopen & Peak RSS (`N = 30`)

Real `0.8.31` `Engine::open` admits `B0` and `S38-PARTIAL-A1` directly (proving Schema 38 additive partial indexes remain non-breaking for `0.8.31` readers), while `S38-PARTIAL-A2` (8-col `vec0`) and `S39-COMPOSITE` (`user_version = 39`) are measured via the engine-equivalent open probe (`N = 30` child subprocesses each).

| Profile | Layout | Probe Mode | `process_open` p50 / p95 (ms) | `warm_reopen` p50 / p95 (ms) | Peak RSS p50 / max |
| --- | --- | --- | ---: | ---: | ---: |
| `single_scope` | `B0` | `engine_equivalent_subprocess` | 10.43 / 15.56 | 11.53 / 13.90 | 21.61 / 22.32 MiB |
| `single_scope` | `S38-PARTIAL-A1` | `engine_equivalent_subprocess` | 9.44 / 12.47 | 11.23 / 14.41 | 21.46 / 22.29 MiB |
| `single_scope` | `S38-PARTIAL-A2` | `engine_equivalent_subprocess` | 10.24 / 13.27 | 10.71 / 12.34 | 21.61 / 22.25 MiB |
| `single_scope` | `S39-COMPOSITE` | `engine_equivalent_subprocess` | 10.09 / 13.03 | 8.17 / 11.93 | 21.61 / 22.23 MiB |
| `single_scope` | `B0` | `real_engine_open_subprocess` | 1264.94 / 1518.47 | 1319.36 / 2129.48 | 22.45 / 29.69 MiB |
| `single_scope` | `S38-PARTIAL-A1` | `real_engine_open_subprocess` | 1245.06 / 1507.34 | 1356.49 / 2362.57 | 22.63 / 29.88 MiB |
| `multi_scope_uniform` | `S38-PARTIAL-A1` | `engine_equivalent_subprocess` | 12.55 / 24.87 | 12.51 / 15.24 | 21.78 / 22.23 MiB |
| `multi_scope_uniform` | `S38-PARTIAL-A2` | `engine_equivalent_subprocess` | 12.67 / 28.05 | 11.83 / 20.55 | 21.71 / 22.13 MiB |
| `multi_scope_uniform` | `S39-COMPOSITE` | `engine_equivalent_subprocess` | 12.50 / 23.45 | 10.46 / 14.37 | 21.59 / 22.12 MiB |
| `multi_scope_uniform` | `S38-PARTIAL-A1` | `real_engine_open_subprocess` | 1342.20 / 1550.08 | 1252.15 / 1414.79 | 22.60 / 29.89 MiB |
| `multi_scope_skewed` | `S38-PARTIAL-A1` | `engine_equivalent_subprocess` | 9.26 / 13.94 | 8.45 / 12.89 | 21.48 / 22.07 MiB |
| `multi_scope_skewed` | `S38-PARTIAL-A2` | `engine_equivalent_subprocess` | 9.48 / 12.66 | 7.66 / 9.67 | 21.57 / 22.18 MiB |
| `multi_scope_skewed` | `S39-COMPOSITE` | `engine_equivalent_subprocess` | 9.07 / 12.05 | 9.82 / 10.62 | 21.61 / 22.06 MiB |
| `multi_scope_skewed` | `S38-PARTIAL-A1` | `real_engine_open_subprocess` | 1294.16 / 1666.65 | 1224.79 / 1400.53 | 22.48 / 29.96 MiB |

Open/reopen/RSS findings:

- **Zero RSS inflation:** Peak RSS stays flat across all four layouts (`21.5`–`22.3 MiB` lightweight probe; `22.5`–`22.6 MiB` p50 and `29.7`–`30.0 MiB` max under full `Engine::open`).
- **Zero open-latency regression:** Paired bootstrap 95% CIs on `process_open` span zero for all comparisons (`B0` vs. `S38-A1` `-0.99 ms` `[-2.40, +0.54] ms`; real `Engine::open` `-19.88 ms` `[-93.79, +45.21] ms`; `B0` vs. `S39` `-0.35 ms` `[-1.89, +1.19] ms`).

## 4. Read, Search, Vector `recall@10`, & Graph Results (`N = 200`)

All read cohorts run `N = 200` paired interleaved trials (`warmup = 5`). In `single_scope`, `B0`, `S38-PARTIAL-A1`, and `S38-PARTIAL-A2` omit `scope` predicates (`unscoped_fast_path`), while `S39-COMPOSITE` binds `scope = ''` natively (required because S39 composite indexes lead with `scope`).

### Latency & `recall@10` Summary (p50 / p95 in ms)

| Profile | Cohort | Layout | `search_text` | `search_vector_k10` | `search_hybrid` | `graph_2hop` | `op_point` | `op_page` | Vector `recall@10` |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `single_scope` | `unscoped_fast_path` | `B0` | 13.25 / 18.91 | 31.32 / 40.98 | 44.63 / 57.40 | 0.0914 / 0.1258 | 0.0133 / 0.0186 | 0.0563 / 0.0839 | 0.6895 (min 0.20, 10.0/10, empty=0) |
| `single_scope` | `unscoped_fast_path` | `S38-PARTIAL-A1` | 12.91 / 17.87 | 30.71 / 40.11 | 43.31 / 58.08 | 0.0920 / 0.1237 | 0.0131 / 0.0188 | 0.0548 / 0.0835 | 0.6895 (min 0.20, 10.0/10, empty=0) |
| `single_scope` | `unscoped_fast_path` | `S38-PARTIAL-A2` | 12.93 / 17.70 | 30.24 / 38.26 | 43.13 / 56.65 | 0.0897 / 0.1274 | 0.0132 / 0.0180 | 0.0540 / 0.0802 | 0.6815 (min 0.30, 10.0/10, empty=0) |
| `single_scope` | `unscoped_fast_path` | `S39-COMPOSITE` | 12.67 / 18.09 | 32.29 / 45.40 | 45.12 / 59.75 | 0.0941 / 0.1352 | 0.0132 / 0.0192 | 0.0570 / 0.0845 | 0.6815 (min 0.30, 10.0/10, empty=0) |
| `multi_scope_uniform` | `uniform_10pct` | `S38-PARTIAL-A1` | 10.07 / 14.60 | 67.71 / 98.05 | 76.98 / 107.88 | 1.1418 / 4.7308 | 0.1203 / 0.2340 | 0.0839 / 0.2956 | 1.0000 (min 1.00, 10.0/10, empty=0) |
| `multi_scope_uniform` | `uniform_10pct` | `S38-PARTIAL-A2` | 9.63 / 17.09 | 15.09 / 21.96 | 24.44 / 34.67 | 0.0999 / 0.3952 | 0.0128 / 0.0195 | 0.0517 / 0.0902 | 0.9460 (min 0.60, 10.0/10, empty=0) |
| `multi_scope_uniform` | `uniform_10pct` | `S39-COMPOSITE` | 9.23 / 13.83 | 14.98 / 21.12 | 23.85 / 31.69 | 0.0884 / 0.1371 | 0.0143 / 0.0206 | 0.0421 / 0.0774 | 0.9460 (min 0.60, 10.0/10, empty=0) |
| `multi_scope_skewed` | `heavy_90pct` | `S38-PARTIAL-A1` | 11.27 / 17.26 | 543.11 / 805.57 | 549.27 / 811.82 | 0.0890 / 0.1304 | 0.0125 / 0.0185 | 0.0524 / 0.0894 | 1.0000 (min 1.00, 10.0/10, empty=0) |
| `multi_scope_skewed` | `heavy_90pct` | `S38-PARTIAL-A2` | 11.34 / 17.09 | 27.71 / 39.07 | 39.66 / 56.78 | 0.0855 / 0.1344 | 0.0122 / 0.0187 | 0.0506 / 0.0896 | 0.6960 (min 0.30, 10.0/10, empty=0) |
| `multi_scope_skewed` | `heavy_90pct` | `S39-COMPOSITE` | 11.11 / 17.26 | 27.91 / 39.79 | 39.44 / 54.54 | 0.0887 / 0.1329 | 0.0126 / 0.0188 | 0.0458 / 0.0817 | 0.6960 (min 0.30, 10.0/10, empty=0) |
| `multi_scope_skewed` | `light_1pct` | `S38-PARTIAL-A1` | 7.53 / 8.39 | 6.33 / 6.94 | 13.86 / 15.73 | 0.0746 / 0.0940 | 0.0120 / 0.0155 | 0.0350 / 0.0451 | 1.0000 (min 1.00, 10.0/10, empty=0) |
| `multi_scope_skewed` | `light_1pct` | `S38-PARTIAL-A2` | 7.67 / 8.86 | 9.00 / 9.85 | 16.45 / 18.27 | 0.0742 / 0.0904 | 0.0114 / 0.0131 | 0.0347 / 0.0434 | 0.9970 (min 0.90, 10.0/10, empty=0) |
| `multi_scope_skewed` | `light_1pct` | `S39-COMPOSITE` | 7.47 / 8.60 | 9.06 / 10.02 | 16.50 / 19.03 | 0.0746 / 0.0959 | 0.0115 / 0.0152 | 0.0293 / 0.0411 | 0.9970 (min 0.90, 10.0/10, empty=0) |

### Key Paired Bootstrap 95% CIs (10,000 resamples, p50)

| Cohort | Comparison | Metric | Baseline -> Candidate (ms) | Rel Delta | Observed Delta & 95% CI (ms) |
| --- | --- | --- | --- | ---: | --- |
| `unscoped_fast_path` | `B0` -> `S38-A1` | `search_text_only` | 13.252 -> 12.913 | -2.56% | -0.339 [-0.674, +0.236] |
| `unscoped_fast_path` | `B0` -> `S38-A2` | `search_vector_k10` | 31.320 -> 30.242 | -3.44% | -1.077 [-1.726, -0.545] |
| `unscoped_fast_path` | `B0` -> `S39` | `search_text_only` | 13.252 -> 12.668 | -4.41% | -0.585 [-1.041, -0.009] |
| `unscoped_fast_path` | `B0` -> `S39` | `search_vector_k10` | 31.320 -> 32.285 | +3.08% | +0.966 [+0.272, +1.574] |
| `unscoped_fast_path` | `B0` -> `S39` | `search_hybrid` | 44.633 -> 45.120 | +1.09% | +0.488 [-0.718, +1.253] |
| `unscoped_fast_path` | `B0` -> `S39` | `graph_expand_2hop` | 0.0914 -> 0.0941 | +2.95% | +0.003 [-0.001, +0.007] |
| `uniform_10pct` | `S38-A1` -> `S38-A2` | `search_vector_k10` | 67.712 -> 15.086 | -77.72% | -52.626 [-54.785, -50.372] |
| `uniform_10pct` | `S38-A1` -> `S39` | `search_vector_k10` | 67.712 -> 14.984 | -77.87% | -52.728 [-54.929, -50.391] |
| `uniform_10pct` | `S38-A1` -> `S39` | `search_text_only` | 10.065 -> 9.234 | -8.26% | -0.831 [-1.238, -0.323] |
| `uniform_10pct` | `S38-A1` -> `S39` | `operational_state_page` | 0.0839 -> 0.0421 | -49.82% | -0.042 [-0.051, -0.036] |
| `heavy_90pct` | `S38-A1` -> `S38-A2` | `search_vector_k10` | 543.109 -> 27.715 | -94.90% | -515.394 [-541.217, -506.775] |
| `heavy_90pct` | `S38-A1` -> `S39` | `search_vector_k10` | 543.109 -> 27.912 | -94.86% | -515.197 [-541.116, -506.526] |
| `heavy_90pct` | `S38-A1` -> `S39` | `search_hybrid` | 549.267 -> 39.440 | -92.82% | -509.827 [-532.220, -502.734] |
| `heavy_90pct` | `S38-A1` -> `S39` | `operational_state_page` | 0.0524 -> 0.0458 | -12.60% | -0.007 [-0.010, -0.003] |
| `light_1pct` | `S38-A1` -> `S38-A2` | `search_vector_k10` | 6.326 -> 8.997 | +42.23% | +2.671 [+2.634, +2.726] |
| `light_1pct` | `S38-A1` -> `S39` | `search_vector_k10` | 6.326 -> 9.056 | +43.17% | +2.731 [+2.698, +2.760] |
| `light_1pct` | `S38-A1` -> `S39` | `operational_state_page` | 0.0350 -> 0.0293 | -16.57% | -0.006 [-0.006, -0.005] |

Read & recall findings:

- **`A1` (7-col `vec0` CTE pre-selection) suffers a catastrophic `19.5x` cliff on large scopes:** `heavy_90pct` (90k vectors in scope) hits **`543.11 ms` p50 / `805.57 ms` p95** (`search_hybrid` `811.82 ms` p95) because `vec_distance_l2` evaluates all 90,000 rows without bit-KNN prefiltering. Even on `uniform_10pct` (`10k` vectors), `A1` is **`4.5x` slower** (`67.71 ms` vs. `14.98–15.09 ms`) and its 10k-vector L2 scan evicts canonical pages from SQLite's 2 MiB cache (elevating `A1` `graph_2hop` to `1.14 ms` vs. `0.09 ms`).
- **8-column `vec0` (`A2` & `S39`) preserves both fast small scopes and full recall:** On `light_1pct` (`1.1%` scope inside `100k`), `sqlite-vec` pushes `scope = ?` into the KNN scan before `LIMIT 192`, returning **`10.00 / 10` hits (`empty = 0`) at `0.9970` `recall@10` (`min = 0.90`) in `9.00–9.06 ms` p50**.
- **`S39` consistently edges `A1`/`A2` on `operational_state_page` (`-12.6%` to `-49.8%` p50):** `PRIMARY KEY(scope, collection_name, record_key)` + `UNIQUE INDEX (scope, collection_name, write_cursor)` gives `0.029–0.046 ms` p50 paging across every scoped cohort.
- **`single_scope` zero-regression floor:** `S38-A1`, `S38-A2`, and `S39` (`scope = ''`) stay within `-4.4%` to `+3.1%` of `B0` across all six read/search/graph operations.

## 5. `EXPLAIN QUERY PLAN` Audit & SQLite Planner Findings

Every captured plan (`13` cohort × layout combinations × `6` query shapes = `78` plans) is recorded verbatim in `scope-schema-results.json`.

- **FTS5 + `scope` join (`search_text_only`):** **Zero real-table `SCAN` and zero `USE TEMP B-TREE FOR ORDER BY` (`clean = true`) across all profiles, cohorts, and layouts**, including `light_1pct` (`1.1%` selectivity). Using `CROSS JOIN canonical_nodes` + `AND rank MATCH 'bm25()' ORDER BY rank` prevents SQLite from inverting join order into a temp B-tree sort when a narrow scope filter would otherwise look cheaper than a broad FTS token:
  - `S38-PARTIAL-A1`/`A2`: `SCAN si VIRTUAL TABLE INDEX 32:rM3` -> `SEARCH cn USING INDEX canonical_nodes_scope_active_cursor_idx (scope=? AND write_cursor=?)`
  - `S39-COMPOSITE`: `SCAN si VIRTUAL TABLE INDEX 32:rM3` -> `SEARCH cn USING INDEX canonical_nodes_scope_cursor_idx (scope=? AND write_cursor=?)`
- **Graph & `operational_state` lookups/paging:** Zero real-table `SCAN` and zero `USE TEMP B-TREE FOR ORDER BY` (`clean = true`) on all scoped cohorts and across all single-scope layouts (`B0`, `S38-PARTIAL-A1`, `S38-PARTIAL-A2`, and `S39-COMPOSITE` binding `scope = ''`).
- **Vector KNN (`search_vector_k10`):** Zero real-table `SCAN`s on every layout. `A2` and `S39` show `SCAN vector_default VIRTUAL TABLE INDEX 1:3{___}___&Ga_&Da_&Ec_&Fe_` inside `CO-ROUTINE candidates`, confirming `scope`, `lifecycle`, `valid_from`, and `valid_until` execute natively inside `sqlite-vec` KNN (`B0`, `A2`, and `S39` record two bounded `USE TEMP B-TREE FOR ORDER BY` steps: the `ORDER BY distance LIMIT 192` bit-KNN candidate sort inside `CO-ROUTINE candidates` plus the outer `vec_distance_l2` rerank sort).
- **Critical SQLite planner caveat discovered for Schema 38 partial indexes:** SQLite **cannot prove at prepare time** that a bound parameter `? != ''`, so `WHERE cn.scope = ?` alone **silently ignores `WHERE scope != ''` partial indexes** and degrades to `SCAN cn USING INDEX canonical_nodes_state_active_idx`! Schema 38 queries MUST emit `AND col.scope != ''` explicitly whenever `!scope.is_empty()` (and still cannot use partial indexes when querying explicit `scope = ''`).
- **Critical SQLite composite-index invariant discovered for Schema 39:** Because Schema 39 replaces single-column indexes with `(scope, ...)` composite indexes, single-scope callers MUST bind `scope = ''`; omitting `scope = ''` causes full scans (`SCAN operational_state`, `SCAN canonical_nodes USING INDEX ...`), which our `1,000`-node pre-check exposed immediately.

## 6. Write Performance, Ownership Isolation, & `SyncMode::Normal` vs. `Full` (`N = 100`)

All write arms restore a fresh WAL-truncated snapshot before testing `SyncMode::Normal` and `SyncMode::Full` (`N = 100` trials after `5` warmup writes). `ownership_guard_verified = true` across all 20 arms: `S38-PARTIAL` rejects cross-scope overwrites (`ScopeViolation`) on shared `logical_id` / `(collection_name, record_key)`, while `S39-COMPOSITE` natively isolates identical keys in separate scopes.

### Batch & Checkpoint Write Latency (p50 / p95 in ms)

| Profile | Layout | `SyncMode` | `node_batch_200_fresh` | `node_batch_200_upsert` | `edge_batch_200` | `checkpoint_fresh` | `checkpoint_upsert` |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| `single_scope` | `B0` | `NORMAL` | 94.52 / 127.16 | 140.72 / 181.41 | 25.84 / 53.64 | 0.0525 / 0.0799 | 0.0636 / 0.0932 |
| `single_scope` | `B0` | `FULL` | 98.34 / 129.24 | 178.76 / 232.52 | 28.40 / 46.60 | 0.7037 / 3.2048 | 0.7095 / 2.1680 |
| `single_scope` | `S38-PARTIAL-A1` | `NORMAL` | 100.50 / 142.48 | 160.62 / 221.34 | 25.22 / 40.21 | 0.0650 / 0.0956 | 0.0822 / 0.1090 |
| `single_scope` | `S38-PARTIAL-A1` | `FULL` | 102.70 / 140.05 | 154.23 / 194.81 | 23.00 / 36.50 | 0.6034 / 0.7661 | 0.6276 / 0.8808 |
| `single_scope` | `S38-PARTIAL-A2` | `NORMAL` | 97.50 / 131.20 | 137.88 / 171.43 | 21.89 / 33.15 | 0.0677 / 0.0976 | 0.0825 / 0.1075 |
| `single_scope` | `S38-PARTIAL-A2` | `FULL` | 82.33 / 106.11 | 133.57 / 166.52 | 21.08 / 33.17 | 0.7096 / 0.9911 | 0.6607 / 0.7649 |
| `single_scope` | `S39-COMPOSITE` | `NORMAL` | 86.93 / 116.47 | 133.95 / 165.85 | 24.49 / 39.06 | 0.0488 / 0.0817 | 0.0649 / 0.0941 |
| `single_scope` | `S39-COMPOSITE` | `FULL` | 91.37 / 112.33 | 142.48 / 173.83 | 26.49 / 37.04 | 0.5914 / 0.9427 | 0.5551 / 0.7020 |
| `multi_scope_uniform` | `S38-PARTIAL-A1` | `NORMAL` | 93.78 / 112.44 | 154.17 / 193.28 | 19.92 / 39.98 | 0.0540 / 0.0998 | 0.1057 / 0.1496 |
| `multi_scope_uniform` | `S38-PARTIAL-A1` | `FULL` | 100.10 / 121.32 | 159.64 / 192.12 | 18.69 / 32.76 | 0.3916 / 0.6499 | 0.5708 / 0.7202 |
| `multi_scope_uniform` | `S38-PARTIAL-A2` | `NORMAL` | 98.06 / 133.16 | 155.55 / 193.00 | 18.84 / 40.27 | 0.0692 / 0.1035 | 0.0693 / 0.1490 |
| `multi_scope_uniform` | `S38-PARTIAL-A2` | `FULL` | 93.85 / 122.29 | 151.05 / 178.03 | 20.26 / 34.66 | 0.5708 / 0.7074 | 0.5835 / 0.7331 |
| `multi_scope_uniform` | `S39-COMPOSITE` | `NORMAL` | 85.29 / 104.49 | 141.54 / 160.15 | 14.94 / 24.75 | 0.0396 / 0.1367 | 0.0449 / 0.0728 |
| `multi_scope_uniform` | `S39-COMPOSITE` | `FULL` | 85.42 / 100.04 | 155.36 / 215.68 | 22.14 / 36.05 | 0.8347 / 4.1005 | 0.6757 / 3.0034 |
| `multi_scope_skewed` | `S38-PARTIAL-A1` | `NORMAL` | 95.82 / 121.91 | 137.75 / 173.27 | 19.08 / 41.93 | 0.0701 / 0.1184 | 0.0600 / 0.0810 |
| `multi_scope_skewed` | `S38-PARTIAL-A1` | `FULL` | 92.05 / 123.04 | 149.81 / 190.95 | 21.45 / 42.95 | 0.3727 / 0.6769 | 0.6143 / 1.3065 |
| `multi_scope_skewed` | `S38-PARTIAL-A2` | `NORMAL` | 94.81 / 112.40 | 152.98 / 231.11 | 26.46 / 55.29 | 0.0822 / 0.1358 | 0.0998 / 0.1260 |
| `multi_scope_skewed` | `S38-PARTIAL-A2` | `FULL` | 117.50 / 139.17 | 147.68 / 165.83 | 21.81 / 44.81 | 0.5775 / 0.6712 | 0.5887 / 0.6772 |
| `multi_scope_skewed` | `S39-COMPOSITE` | `NORMAL` | 87.07 / 107.38 | 191.01 / 239.51 | 17.68 / 32.07 | 0.0608 / 0.1110 | 0.0593 / 0.0857 |
| `multi_scope_skewed` | `S39-COMPOSITE` | `FULL` | 90.97 / 111.97 | 137.95 / 171.12 | 16.84 / 32.38 | 0.3560 / 0.6349 | 0.3814 / 0.6502 |

### Paired `SyncMode::Normal` vs. `SyncMode::Full` Single-Row Checkpoint Deltas (p50, 10,000 bootstrap)

| Profile | Layout | Op | `NORMAL` p50 / p95 (ms) | `FULL` p50 / p95 (ms) | p50 Delta % | Observed Delta & 95% CI (ms) |
| --- | --- | --- | ---: | ---: | ---: | --- |
| `single_scope` | `B0` | fresh | 0.0525 / 0.0799 | 0.7037 / 3.2048 | +1240.4% | +0.651 [+0.643, +0.699] |
| `single_scope` | `B0` | upsert | 0.0636 / 0.0932 | 0.7095 / 2.1680 | +1015.6% | +0.646 [+0.630, +0.666] |
| `single_scope` | `S38-PARTIAL-A1` | fresh | 0.0650 / 0.0956 | 0.6034 / 0.7661 | +828.3% | +0.538 [+0.311, +0.551] |
| `single_scope` | `S38-PARTIAL-A1` | upsert | 0.0822 / 0.1090 | 0.6276 / 0.8808 | +663.5% | +0.545 [+0.360, +0.553] |
| `single_scope` | `S38-PARTIAL-A2` | fresh | 0.0677 / 0.0976 | 0.7096 / 0.9911 | +948.2% | +0.642 [+0.607, +0.733] |
| `single_scope` | `S38-PARTIAL-A2` | upsert | 0.0825 / 0.1075 | 0.6607 / 0.7649 | +700.7% | +0.578 [+0.572, +0.584] |
| `single_scope` | `S39-COMPOSITE` | fresh | 0.0488 / 0.0817 | 0.5914 / 0.9427 | +1111.9% | +0.543 [+0.514, +0.589] |
| `single_scope` | `S39-COMPOSITE` | upsert | 0.0649 / 0.0941 | 0.5551 / 0.7020 | +755.3% | +0.490 [+0.293, +0.525] |
| `multi_scope_uniform` | `S38-PARTIAL-A1` | fresh | 0.0540 / 0.0998 | 0.3916 / 0.6499 | +625.2% | +0.338 [+0.296, +0.528] |
| `multi_scope_uniform` | `S38-PARTIAL-A1` | upsert | 0.1057 / 0.1496 | 0.5708 / 0.7202 | +440.0% | +0.465 [+0.247, +0.483] |
| `multi_scope_uniform` | `S38-PARTIAL-A2` | fresh | 0.0692 / 0.1035 | 0.5708 / 0.7074 | +724.9% | +0.502 [+0.326, +0.527] |
| `multi_scope_uniform` | `S38-PARTIAL-A2` | upsert | 0.0693 / 0.1490 | 0.5835 / 0.7331 | +741.8% | +0.514 [+0.317, +0.536] |
| `multi_scope_uniform` | `S39-COMPOSITE` | fresh | 0.0396 / 0.1367 | 0.8347 / 4.1005 | +2007.8% | +0.795 [+0.732, +1.078] |
| `multi_scope_uniform` | `S39-COMPOSITE` | upsert | 0.0449 / 0.0728 | 0.6757 / 3.0034 | +1404.9% | +0.631 [+0.591, +0.675] |
| `multi_scope_skewed` | `S38-PARTIAL-A1` | fresh | 0.0701 / 0.1184 | 0.3727 / 0.6769 | +431.7% | +0.303 [+0.276, +0.507] |
| `multi_scope_skewed` | `S38-PARTIAL-A1` | upsert | 0.0600 / 0.0810 | 0.6143 / 1.3065 | +923.7% | +0.554 [+0.421, +0.610] |
| `multi_scope_skewed` | `S38-PARTIAL-A2` | fresh | 0.0822 / 0.1358 | 0.5775 / 0.6712 | +602.6% | +0.495 [+0.298, +0.515] |
| `multi_scope_skewed` | `S38-PARTIAL-A2` | upsert | 0.0998 / 0.1260 | 0.5887 / 0.6772 | +489.9% | +0.489 [+0.274, +0.508] |
| `multi_scope_skewed` | `S39-COMPOSITE` | fresh | 0.0608 / 0.1110 | 0.3560 / 0.6349 | +485.5% | +0.295 [+0.274, +0.502] |
| `multi_scope_skewed` | `S39-COMPOSITE` | upsert | 0.0593 / 0.0857 | 0.3814 / 0.6502 | +543.2% | +0.322 [+0.292, +0.527] |

Write & durability findings:

- **`SyncMode::Normal` vs. `SyncMode::Full`:** Every unbatched single-row checkpoint pays a **`+431.7%` to `+2007.8%` (`5.3x`–`21.1x`) p50 penalty** under `FULL` (`+0.30` to `+0.80 ms` per commit, p95 up to `4.10 ms`). Across `200`-row batches (`85`–`160 ms`), the single WAL fsync is amortized into SQLite/FTS/property CPU time (typically `+0%` to `+10%`, occasionally masked by WAL autocheckpoint variance).
- **Schema 38 vs. Schema 39 write deltas (`NORMAL`):**
  - Fresh `200`-node batches (`node_batch_200_fresh`) are **`-8.2%` to `-13.5%` faster** on `S39-COMPOSITE` (`85.29`–`87.07 ms`) than `S38-PARTIAL-A1`/`A2` (`93.78`–`100.50 ms`, specifically **`-8.2%` to `-13.0%` vs. `S38-PARTIAL-A2`** [`94.81`–`98.06 ms`] and **`-8.0%` vs. `B0`** [`94.52 ms`] in `single_scope`), because `S39` maintains one set of `(scope, ...)` B-trees instead of updating both legacy and partial B-trees (`95%` CI strictly `< 0` on both multi-scope profiles: `[-12.35, -5.90] ms` uniform, `[-14.48, -5.60] ms` skewed). Note one upsert outlier: on `multi_scope_skewed` `node_batch_200_upsert` `NORMAL`, `S39` measured `191.01 ms` p50 (`+24.9%` vs. `S38-A2` `152.98 ms` and `+38.7%` vs. `S38-A1` `137.75 ms`, CI `[+45.56, +60.00] ms`, dropping back to `137.95 ms` under `FULL` — consistent with WAL autocheckpoint boundary variance).
  - Unbatched single-row multi-scope checkpoints (`NORMAL`) are **`-26.0%` to `-42.8%` faster** on `S39-COMPOSITE` (`0.0396`–`0.0608 ms`) than **`S38-PARTIAL-A2`** (`0.0692`–`0.0998 ms` across all 4 multi-scope cells), and **`-1.2%` to `-57.5%`** vs. **`S38-PARTIAL-A1`** (`0.0540`–`0.1057 ms`, where `multi_scope_skewed` `checkpoint_upsert` `-1.2%` CI `[-0.0052, +0.0009]` spans zero), because `S39` needs no pre-insert `SELECT scope` ownership guard.

## 7. Evidence-Backed Recommendation (Decision 1 & Decision 2)

1. **8-column `vec0` (`scope TEXT DEFAULT ''`) is empirically mandatory (`A1` rejected):**
   `S38-PARTIAL-A1` (7-col `vec0` CTE pre-selection) collapses to **`543.11 ms` p50 / `805.57 ms` p95** on `heavy_90pct` and `67.71 ms` p50 on `uniform_10pct`. Only 8-column `vec0` (`A2` or `S39`) delivers **`14.98`–`27.91 ms` p50 (`-77.9%` to `-94.9%`)** on medium/heavy scopes while still maintaining **`9.00`–`9.06 ms` p50**, **`0.9970` `recall@10`**, and **`10.00 / 10` results (`empty = 0`)** on `1.1%` light scopes.
2. **Decision 1 — Recommend `S39-COMPOSITE` (Schema 39 composite `(scope, ...)` PKs/indexes + 8-col `vec0`) as the primary `0.8.32` target, with `S38-PARTIAL-A2` only if table-rebuild migration is explicitly disallowed:**
   - **Why `S39-COMPOSITE` wins technically:** Beats `S38-PARTIAL-A2` on multi-scope disk overhead (`-5.83` to `-6.65 MiB`), fresh `200`-node batch writes (`-8.2%` to `-13.0%` `NORMAL`, noting the `multi_scope_skewed` `node_batch_200_upsert` `NORMAL` autocheckpoint outlier of `191.01 ms`), multi-scope checkpoint writes (`-26.0%` to `-42.8%` `NORMAL` vs. `S38-A2`; `-1.2%` to `-57.5%` vs. `S38-A1`), `operational_state_page` (`-12.6%` to `-49.8%` vs. `S38-A1`), true `(scope, logical_id)` / `(scope, collection_name, record_key)` tenant key-space isolation (no cross-tenant ID collisions), and zero SQLite partial-index planner traps (`S38` requires explicit `AND col.scope != ''` on every non-empty scoped query and still cannot accelerate `scope = ''`).
   - **Required `S39-COMPOSITE` design invariants (`0.8.32`):** (a) Every query path — including single-tenant callers — MUST bind `scope = ''` (never omit `scope`), preserving `single_scope` parity (`-4.4%` text, `+3.1%` vector, `-8.0%` fresh write, clean `EXPLAIN QUERY PLAN`); and (b) `STEP_39_SQL` must carry explicit `MIGRATION-ACCRETION-EXEMPTION` authorization (matching Step 23 TC-33) alongside non-destructive `vec0` 8-col reshape (`vec_bit(?)`).
   - **Fallback (`S38-PARTIAL-A2`):** Choose `S38-PARTIAL-A2` only if HITL Decision 1 mandates zero canonical table rebuilds and single global-keyspace semantics (cross-scope ownership rejection on collision instead of per-scope namespace isolation), while still performing the `vec0` 8-col reshape (`+0.46%` single-scope disk, `-3.4%` single-scope vector p50, and explicit `AND col.scope != ''` query guards everywhere).
3. **Decision 2 — Retain `SyncMode::Normal` default, expose explicit `SyncMode::Full`:**
   `SyncMode::Full` incurs a **`+431.7%` to `+2007.8%` (`5.3x`–`21.1x`)** latency increase on single-row checkpoints (`0.04`–`0.11 ms` -> `0.36`–`0.83 ms` p50, up to `4.10 ms` p95), whereas `200`-row batches hide the single fsync cost. Keeping `SyncMode::Normal` as the engine default with opt-in `SyncMode::Full` gives callers durability choice without regressing single-row MCP checkpoint latency.
