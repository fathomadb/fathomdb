# COMP-02b result — filtered FTS at measured selectivity, FathomDB 0.8.27 vs SurrealDB 3.3.0

Run 2026-10-15 under the [side plan](plans/2026-10-15-comp-side-plan.md).
Same host, arms, and SurrealDB protocol as
[COMP-02a](2026-10-15-comp-02a-ingest-ready-scale-result.md).

## Method

- **Corpus and cohorts.** The COMP-02a 10k TC-5 corpus with a deterministic
  `bench_cohort` from a hash of each document ID. The groups are disjoint:

  | Cohort | Documents | Observed selectivity |
  | --- | ---: | ---: |
  | `c01` | 92 | 0.92% |
  | `c10` | 968 | 9.68% |
  | `c33` | 3,327 | 33.27% |
  | `rest` | 5,613 | 56.13% |

- **FathomDB:** each document body is `{"text", "bench_cohort"}`.
  - Projections: `text` is `SEARCHABLE` with FTS; `bench_cohort` is
    `FILTERABLE`.
  - Query:
    `search_projected_text(q, "text", SearchFilter(attributes=(("bench_cohort", c),)), limit=10)`.
- **SurrealDB:** the COMP-02a full-text index plus a btree index on `cohort`.
  - Query:
    `WHERE body @0,OR@ $q AND cohort = $c ORDER BY search::score(0) DESC LIMIT 10`.
- **Per cell** (four fresh databases per arm, `AB/BA/BA/AB`): for each cohort,
  an untimed warm pass of the 100 TC-5 queries, then a timed pass in a seeded
  shuffle. Every returned ID is checked for cohort membership.
- **Not run:** a filtered vector KNN case. FathomDB 0.8.27 has no public
  vector-only query, so the KNN case is recorded as unsupported and has no
  numbers.

## Results

All 8 cells (24 arm × cohort × repetition passes) completed. **Zero
membership violations** in 4,800 filtered queries per arm.

Medians over four repetitions:

| Cohort | FathomDB p50 / p95 (ms) | SurrealDB p50 / p95 (ms) | FathomDB queries with < 10 hits | SurrealDB queries with < 10 hits |
| --- | ---: | ---: | ---: | ---: |
| `c01` (0.92%) | 45.2 / 75.4 | **4.2** / 5.7 | 6 | 0 |
| `c10` (9.7%) | 45.5 / 76.5 | **22.7** / 27.5 | 0 | 0 |
| `c33` (33.3%) | **51.7** / 86.8 | 73.9 / 88.3 | 0 | 0 |

Repetition p50 ranges:

- FathomDB: 43.8–46.4 ms (`c01`), 44.8–46.0 ms (`c10`), 50.2–86.2 ms
  (`c33`; one outlier repetition).
- SurrealDB: 3.6–4.3 ms, 19.9–35.3 ms, 71.4–76.9 ms.

## Interpretation

- **Correctness.** Both arms apply the filter before the top-10 cut, with no
  out-of-cohort hits. FathomDB's six short `c01` results come from its query
  translation, not from a candidate cap. After stop-word removal, only 0–7
  `c01` documents contain any content term of those queries; for example,
  q-095 `anniversary, mortgage, pay` has 0. SurrealDB's analyzer keeps stop
  words, so "the" and "what" match almost every document and fill all ten
  slots.
- **Scaling with selectivity.** SurrealDB's latency grows with the number of
  rows that match the filter, because it scores every OR candidate within the
  filtered set (see COMP-02a). It is about 11× faster than FathomDB at 1%,
  about 2× faster at 10%, and about 1.4× slower at 33%. FathomDB's latency is
  nearly flat across selectivity.
- **FathomDB's cost is in the projected-FTS path, not the filter.** A
  follow-up on one 10k projected database (40 queries, p50):

  | Query | p50 (ms) |
  | --- | ---: |
  | `search_projected_text`, no filter | 62.4 |
  | `search_projected_text`, with `c01` filter | 47.9 |
  | `search_projected_text`, with `c33` filter | 52.5 |
  | `search_text_only` on the same database | 9.7 |
  | Projected search, single term | 15.2 |

  The attribute filter does not add cost; it slightly reduces it. Projected
  property FTS costs about 6× body FTS. This is the main FathomDB lead from
  COMP-02b, and its mechanism has not been diagnosed.
- Unfiltered SurrealDB FTS on this corpus is about 149 ms p50 (COMP-02a).
  Its filtered queries are faster because the btree prefilter shrinks the
  set it scores.

## Artifacts

`~/.cache/fathomdb-comp/comp02b-10000/`: the manifest and eight cell JSONs
with per-cohort ordered IDs and timings. Summarize with
`python -m experiments.comp02_report filtered --out <dir>`.
