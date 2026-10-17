---
title: FathomDB 0.8.28 Slice 40 — design
status: REVISED AFTER R1 (APPROVE-WITH-FIXES, F1–F6); § 2b Defect C and § 2c Unit D landed; § 2d–2f Units E, F, G added 2026-10-16 and landed; § 2g Unit H added 2026-10-16, code committed; § 2h Unit I added 2026-10-16, reviewed (APPROVE-WITH-FIXES, F1–F9 applied); § 2i Unit J added 2026-10-16
target_release: 0.8.28
observed_on: 2026-10-16
---

# Slice 40 design

Mechanisms for the three defects in [requirements.md](requirements.md).
A and B are small, local edits to pure functions; C (§ 2b) moves projected
search onto the body path's existing rank stream. No public signature,
schema, knob or wire shape changes.

## 1. Defect A: one term per arm per body

### 1.1 Mechanism

In `fuse_three_arms` (`src/rust/crates/fathomdb-engine/src/fusion.rs`),
each of the three per-arm loops keeps a set of the bodies that arm has
already credited. A hit whose body is in the set is skipped before
`accumulate` runs; otherwise the body is added and the hit is accumulated
with its positional `rank0` exactly as today.

```text
for each arm (vector, text, graph):
    credited = {}
    for (rank0, hit) in arm:
        if hit.body in credited: continue
        credited.add(hit.body)
        accumulate(hit, rank0, in_vector, weight)   # unchanged
```

The per-arm set holds **entry indices**, not bodies (R1 F6). `accumulate`
already resolves a hit to its accumulator entry by body hash plus an
equality check against the stored entry's body (the `BodySlot` scheme).
Each arm passes its own `HashSet<usize>` of entry indices it has credited:

- the body resolves to an existing entry already in the arm's set: skip;
- it resolves to an existing entry not in the set: add the term, insert
  the index;
- it is a new body: create the entry as today, insert its index.

This is R1's "hash, then confirm equality against the stored entry" with
the hashing done once, by the lookup that already exists: no body clone or
second hash per hit, and no false skip on a hash collision because the
equality check decides. The accumulator, the `BodySlot` collision
handling, the sort and the score write-back are otherwise untouched.

### 1.2 Decisions

- **Positional rank of the first occurrence, not a compacted rank.** A
  skipped duplicate still occupies its position, so the hit after it keeps
  its original rank. Reasons: (1) the explain sidecar's `body_rank_map`
  (`search.rs`, "keeps the FIRST occurrence") reports `i`, the positional
  index of the first occurrence; compacting would make fusion's rank and
  the reported `text_rank` disagree; (2) the hits below a duplicate pair
  really were ranked below two rows by FTS5; (3) it is the smallest change.
  Within one arm, both choices give the same order; they differ only in
  cross-arm score weights by at most one rank step per skipped duplicate.
  R1 ruled Q1: keep the positional first-occurrence rank.
- **Keep keying on `body`, not `write_cursor`.** The body is the merge
  identity across arms: a vector hit and a text hit of the same row are
  joined by body, and the recall harnesses and explain maps key on
  `hit.body` (`dev/design/slice-10-design.md`). Keying on `write_cursor`
  would stop collapsing duplicate rows into one result, which changes the
  result shape (two hits with the same body), changes every corpus with
  duplicate bodies far more than this fix, and breaks the edge-vs-node
  collapse that Slice 23's prefix test relies on. Out of scope.
- **The skip is per arm, not global.** A body in both the vector and text
  arms still receives one term from each (REQ40-02).
- **Edge bodies.** The text arm is node FTS hits followed by edge FTS hits.
  An edge body equal to a node body now adds nothing (the node occurrence
  ranks first). Before, it added a deep extra term; for example, the
  `slice23_text_limit_prefix_stability.rs` fixture's node 10 scored
  `3/41 + 3/81` and ranked first. That test asserts only prefix stability,
  so it stays green.
- **Graph-arm call site.** With `use_graph_arm`, `search.rs` calls
  `fuse_three_arms(two_arm_fused, vec![], graph_candidates)`; the first
  list is already body-unique, so only duplicates inside the graph
  candidates change.

### 1.3 What must not change

The constants (`RRF_K`, the three weights, `RECENCY_WEIGHT`); the sort key;
vector-first representative identity; the `fuse_rrf` delegation; the
`apply_recency_reweight` and `apply_importance_reweight` passes; the
candidate SQL and its ordering; `rerank_fused`.

## 2. Defect B: keep 1–2 character tokens

In `content_tokens` (`src/rust/crates/fathomdb-query/src/lib.rs`), replace
`token.len() < 3 || stop.contains(token)` with
`token.is_empty() || stop.contains(token)`. The `is_empty()` check is
required: splitting on non-alphanumeric characters yields empty fragments
("alpha--beta", runs of spaces), and the length filter was what removed
them.

Unchanged: `STOPWORDS`, lowercasing, the split predicate, first-seen
de-duplication, the `"tok" OR "tok"` content shape, and the fallback
branch of `compile_text_query`. The fallback now fires only when no
non-stopword alphanumeric token exists. A query such as "D3" moves from the
fallback (`"D3"`) to the content path (`"d3"`). FTS5's `unicode61`
tokenizer folds case, so both forms match the same rows; the expression
string differs.

The removed filter was a byte-length check (`len()` on `&str`), so it also
dropped one-character non-ASCII tokens such as "é". Those are now kept,
which is consistent with the rule.

Every caller of `compile_text_query` picks this up: the body FTS arm and
edge FTS arm (`search.rs`), projected-text search (`search.rs`,
`read_projected_text_in_tx`), the hybrid path (`search_api.rs`) and graph
seeding (`graph_expand/execution.rs`).

## 2b. Defect C: stream projected search in rank order

### 2b.1 Mechanism

`read_projected_text_in_tx` (`search.rs`) gets the body path's shape from
`read_search_in_tx`:

```text
node_predicate, filter_predicate, params   # built exactly as today
if not force_full_sort (test hook):
    stream_sql = property_fts_rank_stream_sql(node_predicate, filter_predicate, rank_mapping)
    streamed = tx.prepare(stream_sql)
                 .and_then(|s| collect_complete_rank_boundary(s.query(params), limit))
                 .ok()
match streamed:
    Some((hits, crossed_tie)) -> negate each hit.score (score = -bm25);
                                 route projected_rank_stream_tie_completed | _strict_boundary
    None -> run property_fts_rank_sql and the existing row loop, unchanged;
            route projected_full_sort_forced | projected_full_sort_fallback
```

`property_fts_rank_stream_sql` is a new builder in `filter.rs` next to
`property_fts_rank_sql`:

```sql
SELECT n.body, n.kind, p.write_cursor, bm25(property_search_index),
       n.logical_id, n.source_id
FROM property_search_index p JOIN canonical_nodes n ON n.write_cursor = p.write_cursor
WHERE p.attr_name = ?1 AND property_search_index MATCH ?2
  {validity}{eligibility}
  AND rank MATCH '{rank_mapping}'
ORDER BY rank
```

`rank_mapping` is `bm25()`; under `test-hooks` with
`FATHOMDB_FTS_FAIL_STREAM_FOR_TEST` set it is `missing_ranker()`, the body
path's hook, so the statement fails at execution. Pinning the per-query
`rank MATCH 'bm25()'` also overrides any persistent `rank` configuration on
the table, as on the body path. The parameters are the same vector as the
full sort's, so `?1`, `?2` and `?3` (`:now`) bind identically.

### 2b.2 Decisions

- **Collector reuse: reorder the stream statement's select list; do not add
  a row mapper.** `collect_complete_rank_boundary` reads rows through
  `rank_search_hit_from_row` (body, kind, write_cursor, bm25, logical_id,
  source_id) and `retain_complete_rank_boundary_candidates` sorts on
  `hit.score` ascending as **raw bm25**, then `write_cursor`. A mapper that
  produced the projected score `-bm25` would invert that sort, so the
  collector needs raw bm25 in `score` either way. Giving the new statement
  the body path's column order reuses the collector, its tie completion and
  its Slice 20 tests unchanged, and costs one negation loop in the caller.
  IEEE 754 negation is exact, so `-bm25` after collection is bit-identical
  to the fallback's `-bm25`. `id` comes from `derive_stable_id(logical_id,
  body)` in both mappings; `kind` is `n.kind` in both; `branch` is `Text`
  and `ce_score` `None` in both.
- **`property_fts_rank_sql` stays byte-identical.** It remains the fallback
  and is what `slice35_ranked_eligibility_sql_for_test` pins. The stream
  builder carries the same `{validity}{eligibility}` fragments.
- **No SQL `LIMIT`.** The collector stops reading at `limit` once the tie
  group at the boundary is complete; a SQL `LIMIT` would cut a tie group.
  The bound is the validated `limit`, so the body path's fixed 100-row
  window (and its limit-near-100 shortfall) is not copied.
- **Fallback in the same function and transaction**, as on the body path:
  any prepare or execution error of the stream yields `None` and the full
  sort runs. `FATHOMDB_FTS_FORCE_FULL_SORT_FOR_TEST=1` skips the stream, so
  tests can compare both statements.
- **Witnesses reuse the body path's machinery** (`record_fts_route_for_test`,
  `record_fts_query_plan_for_test`, both `test-hooks` only). Route names
  carry a `projected_` prefix so a test can tell the two paths apart. The
  plan is recorded for whichever projected statement runs, so the forced
  full sort's plan shows the temporary b-tree and the witness is not
  vacuous.
- **The regression-net tests run only under `test-hooks`.** AC40-27, 30
  and 31 need no hook, but they share `slice40_projected_fts_stream.rs`
  with the witness tests, and by crate convention a file that uses
  `test-hooks` is a `required-features = ["test-hooks"]` target, so the
  default `cargo test -p fathomdb-engine` skips the whole file. W5 runs the
  file explicitly with `--features test-hooks`.
- **Why it is correct.** FTS5 `ORDER BY rank` yields rows in ascending
  `bm25()` order; the node and filter predicates only remove rows, they do
  not change any row's bm25 (the score is a function of the FTS index, not
  of the join). So the first `limit` surviving rows plus the rest of the
  boundary tie group, re-sorted by (bm25, write_cursor), are exactly the
  first `limit` rows of the full sort.

### 2b.3 What must not change

The full-sort statement and its row loop; `property_fts_rank_sql`;
`collect_complete_rank_boundary`, `rank_search_hit_from_row` and
`retain_complete_rank_boundary_candidates`; the body path and hybrid
search; the projected API, its validation (declared `searchable` FTS
projection, snapshot attribute validation) and its result shape; the
projection cursor read.

### 2b.4 Risks

| Risk | Effect | Handling |
| --- | --- | --- |
| A tie group straddles the limit | Wrong rows if the stream stopped at `limit` | The collector completes the boundary tie group, then sorts by (bm25, write_cursor) and truncates; AC40-27 checks limits inside three tie groups |
| `.ok()` hides a real stream error | A broken stream statement silently falls back, so the slice ships green and slow | AC40-28 asserts the stream route and plan on every eligible call; a stream that always fails records `projected_full_sort_fallback` and turns AC40-28 RED |
| A genuine storage error in the stream | Swallowed, then the full sort hits the same error and returns it | Same behavior as the body path; the full sort's error is the one reported |
| Very large tie group at the boundary | The collector reads the whole group with bodies | Bounded by the group; same as the body path; still no sort of the full match set |
| `-0.0` vs `0.0` under `total_cmp` | Different tie order from SQLite | FTS5 bm25 of a matched row is strictly negative (IDF is floored above zero), so it does not arise; the body path shares this property |
| The planner puts `canonical_nodes` outside the FTS5 table in the inner join | `ORDER BY rank` would need a temporary b-tree again; same rows, no speed-up | The diagnosis's V8 plan shows FTS5 as the outer loop with no temporary b-tree; AC40-28's plan witness fails if that changes. If it is ever needed, `CROSS JOIN` pins the order without changing rows (open question for R1) |
| Frozen reads | Projected search under a frozen view | Same `FrozenView` predicate and parameters; no new clock read |

## 2c. Unit D: COMP-03 retrieval levers

### 2c.1 Where each part lives

| Part | Worktree and branch | Why |
| --- | --- | --- |
| D1 sweep, D2 answer v2 | `comp03-slice40`, `slice/0.8.28-40-comp03`, cut from `llm/comp-01` at `0e246ad31` | `release/0.8.28` carries `experiments/` but not the COMP harness (`comp_fts.py`, `comp03_answer.py`, their configs and tests); those exist only on the comp lineage |
| D3 stop list | `slice-0.8.28-40`, `slice/0.8.28-40-fts-ranking-defects` | product code in `fathomdb-query` |

`experiments/fathomdb_test_setup.py` exists on both lineages. D1 edits it
on the comp branch only (§ 2c.5), so the two copies diverge until the comp
lineage merges forward; noted as a risk.

### 2c.2 Store shapes per arm

R1 ruling Q-D2: the hybrid arm uses the fts arm's plain bodies, so the
pair is a pure vector-arm ablation over identical text.

| Sweep arm | `comp_fts` arm | Store | Query |
| --- | --- | --- | --- |
| `fathomdb-fts` | `fathomdb` (unchanged) | plain round text as the body, no embedder, no projection: exactly v1 | `search_text_only(question, limit=10)` |
| `fathomdb-hybrid` | `fathomdb-hybrid` (new) | `Engine.open(use_default_embedder=True)`; `configure_projections([ProjectionSpec(name="dense", roles=frozenset({SEARCHABLE}), vector=True)])` before writes; the same plain bodies; `drain`, then the coverage check | `search(question, limit=10)`: RRF of the vector and text arms (3:1 text), no rerank, no graph arm |
| `lancedb-fts` | `lancedb` (unchanged) | as v1 | as v1 |

- **Product finding.** A plain `doc` write under
  `use_default_embedder=True` embeds nothing: no vector kind is enrolled,
  `_fathomdb_vector_rows` stays empty, and `search` returns text hits only
  (orchestrator measurement on the CUDA wheel, 2026-10-16). Declaring a
  searchable `vector=True` projection is the public switch that enrols the
  node kind for dense projection. The test-hooks seam
  `_configure_vector_kind_for_test` is not used.
- **What is embedded.** The projection worker embeds `job.body`, the
  node's full `canonical_nodes.body` (`projection_worker.rs`,
  `embed_projection_batch` and the pending-job query), not a projected
  attribute. A projection named `dense`, absent from every body, therefore
  still gives one vector row per document (verified on the CPU dev wheel
  and on the CUDA wheel, 200 of 200).
- The `fathomdb-fts-projected` variant of the earlier draft is dropped: it
  measured a store-shape difference that no longer exists.

### 2c.3 Module layout (D1)

New `experiments/comp03_retrieval.py`; config
`experiments/configs/comp-03/retrieval-sweep.v1.json`.

- `CONFIG_KEYS`, `DEFAULT_CONFIG`, `Comp03RetrievalError`, `load_config`:
  schema `comp-03.retrieval-sweep.v1`; keys `schema_version`, `campaign`,
  `corpus` (equal to answer v1's), `selection` (`{"rule": "all"}`),
  `document_unit` (equal to v1's), `arms` (`id`, `product_arm`),
  `retrieval` (`top_k` 10), `metrics` (`ks` [5, 10], `bootstrap` {`seed`,
  `resamples` 10000}), `device` (the env variable name), `decision_rule`,
  `claim_boundary`.
- `convert_question`, `gold_for`: re-exported from `comp03_answer` (the
  same objects).
- `question_scores(ids, gold, ks)`, `bootstrap_ci(diffs, seed, resamples)`,
  `paired_deltas(rows, metric, treatment, control, seed, resamples)`,
  `evaluate_decision_rule(rule, deltas)`: pure.
- `require_vector_coverage(database_path, documents)`: reads the closed
  database, returns the count or raises.
- `sweep(config, dataset_path, attempt_root, max_questions=None)`: writes
  `questions/<qid>/<arm-id>/retrieval.json` through `comp03_answer.retrieve`
  (fresh store per question per arm, removed after), then `summary.json`
  and an `experiments.record.v1` receipt as the existing campaigns do.
- CLI: `python -m experiments.comp03_retrieval run CONFIG --dataset FILE
  --attempt-root DIR [--fathomdb-bin PATH] [--max-questions N]`.

### 2c.4 Metric definitions

Gold comes from `gold_for`: `session_relevant` is every round of every
answer session (round IDs `sNNN-rNNN`, so the session is the `sNNN`
prefix), `turn_relevant` every round with a `has_answer` turn. Abstention
questions are retrieved but not scored. Over the top-k IDs:

- **session Recall@k (any):** 1 if any round of any answer session is in
  the top k, else 0.
- **session Recall-all@k (new):** 1 if every answer session has at least
  one of its rounds in the top k, else 0. Two rounds of one answer session
  do not count for a second session. For a single-session question it
  equals Recall@k (any); it separates the two on multi-session questions.
- **session MRR@10:** 1 / rank of the first round of any answer session in
  the top 10, else 0.
- **turn Recall@k:** 1 if any `has_answer` round is in the top k, else 0;
  `None` when the question has no `has_answer` round.
- **Latency:** the first query and five repeats per store, as
  `comp03_answer.retrieve` does. For `fathomdb-hybrid` the query latency
  includes embedding the question.
- **Build:** `ingest_ack_s` (writes acknowledged), `build_return_s`
  (through `drain`), and `drain_s = build_return_s − ingest_ack_s`, which
  for the hybrid arm is the embedding time. `vector_rows` is recorded per
  question.
- **Paired bootstrap:** per question, `d = metric(treatment) −
  metric(control)`; 10,000 resamples of the paired `d` with
  `random.Random(seed)`, mean of each; the CI is the 2.5th and 97.5th
  percentiles (indices 249 and 9749, as `comp03_answer.paired_summary`).
  Overall and per question type.

### 2c.5 Device and completeness checks

- The requested device is `FATHOMDB_EMBED_DEVICE` (`auto`, `cpu`,
  `cuda:N`). The check that `prepare_test_database`'s `_default_open`
  already makes (a `cuda:` request must resolve to an effective CUDA device
  with a selected UUID) moves into a public
  `fathomdb_test_setup.require_embed_device(resolution, requested)`; both
  `_default_open` and the hybrid arm call it. It runs once per process on
  the first hybrid open; the summary records `{"requested", "effective"}`.
  A refused check stops the sweep: a GPU run that fell back to CPU is never
  reported.
- `drain` is a barrier, not proof of embedding (memory
  `embed-completeness-and-gpu-readiness.md`). After each hybrid build the
  sweep calls `require_vector_coverage`: `COUNT(DISTINCT write_cursor) FROM
  _fathomdb_vector_rows WHERE kind = 'doc'` on the closed database must
  equal the documents written. A mismatch fails the question with an
  error; it is never recorded as a result.

### 2c.6 Decision rule

Config `decision_rule`: `{"metric": "session_recall_any_at_5", "treatment":
"fathomdb-hybrid", "control": "fathomdb-fts", "focus_type":
"single-session-preference"}`. `evaluate_decision_rule(rule, deltas)`
returns `{"verdict", "clauses": {"a", "b", "c"}}`:

- (a) `per_type[focus].ci[0] > 0`;
- (b) for every other type, `ci[1] >= 0` (no upper bound below zero);
- (c) `overall.delta >= 0`;
- `adopt` iff all three; `not_evaluated` when the sweep lacks either arm.

Preference has 30 questions, so clause (a) needs a large effect to pass;
that is the rule as stated, not tuned.

### 2c.7 Answer run v2 (D2)

- `load_config` accepts `comp-03.answer.v1` exactly as today and
  `comp-03.answer.v2`. v2 arm entries carry `id`, `product_arm` (the
  `comp_fts.ARMS` key) and, for the hybrid arm, `requires_sweep_verdict:
  "adopt"`. `run` takes `sweep_summary=PATH`; a gated arm runs only when
  that summary's `decision.verdict` is `adopt`, and the receipt records the
  summary's SHA-256 and the arms excluded.
- **Arm order:** `arm_order(index, arms) = arms[index % n:] + arms[:index %
  n]`. For two arms this is v1's rule (even index A, B; odd B, A); for
  three, every arm takes every position once in each run of three
  questions. When the gate excludes the hybrid arm, the two remaining arms
  rotate as in v1.
- Metric keys follow `top_k` (`…_at_10`); answer accuracy is reported for
  every arm pair, each as `a − b` in config order.
- `fathomdb_test_setup` stores for the hybrid arm, when `--fathomdb-bin`
  is given, use `embedder="default"`, `warm_cache=True`.

### 2c.8 D3 stop list

Add the twelve words of REQ40-D5 to `STOPWORDS`. The fragments `s t ll re
ve m` are what splitting on the apostrophe leaves of 's, n't, 'll, 're,
've, 'm; they match most English rows, carry no topic, and on TC-5 `"s"`
alone matched 3,912 of 10,000 rows. `my me us am so up` are short function
words that Defect B turned into OR terms (41 of 60 COMP-03 questions carry
one). `d` stays content: it is the vitamin D case Defect B was for, and
`short_stopwords_are_still_dropped` pins it. No other compiler change.

### 2c.9 Risks

| Risk | Handling |
| --- | --- |
| Hybrid silently equals FTS (no vectors) | Coverage check per store; `vector` hits asserted in a test |
| CUDA run falls back to CPU | `require_embed_device` refuses; device recorded |
| Store shape confounds the hybrid delta | Removed by Q-D2: both arms index the same plain bodies |
| D3 hurts NFCorpus or recall | Measurement gate REQ40-D6; D3 in its own commit for revert |
| `possessive_and_contraction_fragments_are_content_tokens` contradicts D3 | Deleted in the D3 commit (Q-D1) |
| AC-038 mirror goes stale | Its private stop-list copy gains the twelve words in the D3 commit; fixture unchanged (Q-D3) |
| `fathomdb_test_setup.py` diverges between lineages | Recorded; merge-forward resolves |

## 2d. Unit E: SDK parity and the public-surface contract

### 2d.1 Where it lives

The parity suite extends the comp lineage's cross-SDK harness in
`comp03-slice40`: `experiments/sdk_benchmark.py`, `sdk_registry.py`, the
three drivers under `experiments/sdk_drivers/`, and their tests. It is not
forked. The interface and public docs change in this worktree.

### 2d.2 Building the arms from this slice's engine

The harness worktree's `src/` is the comp lineage's engine, not this
slice's. Every artifact is therefore built from a clean checkout of the
slice head (`slice-0.8.28-40`), outside both trees, and pinned by SHA-256
in its `sdk.arm.v1` manifest:

| Arm | Build | Pinned |
| --- | --- | --- |
| Python | `maturin build --release --compatibility linux --features default-embedder` from the slice head; wheel installed into a dedicated venv under `~/.cache/fathomdb-comp/` | the venv's `python`, `_fathomdb*.so`, wrapper digest |
| TypeScript | `npm ci`, then the N-API release build with `--features default-embedder`, from the slice head's `src/ts`; package copied outside the repository | `node`, the `.node` addon, the package wrapper digest |
| Rust | `experiments/sdk_drivers/rust/build_against.py --engine-checkout <slice head>`: copies the driver crate to a scratch directory, points `fathomdb-sdk` and the `third_party` patches at the slice checkout, then `cargo build --release --features default-embedder`; `build_manifest.py` attests | the driver binary |
| Bootstrap CLI | `cargo build --release -p fathomdb-cli --features default-embedder` from the slice head | the `fathomdb` binary |

All four are CPU builds (`cuda_compiled` false). The suite's treatment
environment forbids `FATHOMDB_*` overrides, so the dense case relies on the
CPU build, not on `FATHOMDB_EMBED_DEVICE`. A driver whose open report says
`cuda_compiled` true is refused.

**Engine identity.** No SDK has a build-commit accessor, and every build
reports `0.8.27`. Each arm's existing `engine_revision` field is therefore
set by the build step to the slice head's commit, with a clean-tree check,
and the manifest pins every artifact digest. A product build-info accessor
(commit, features, `cuda_compiled`) in all three SDKs is a follow-up (Q-E2).

Two harness changes follow:

- `wrapper_sha256("typescript")` hashes the harness tree's `src/ts` today.
  It gains a `package_root` like Python's `native_path`, so the TypeScript
  wrapper digest is the slice head's.
- `sdk_drivers/rust/Cargo.toml` keeps its in-tree path for the existing
  suite.

### 2d.3 Cases, fixtures and the oracle

`build_parity_fixture(case_id)` is deterministic; its output is written to
an external fixture file and pinned like the existing synthetic fixture.
`populate_parity_database(path, fixture)` builds each case's store:

| Case | Store | Call | Limit |
| --- | --- | --- | --- |
| `text-only-v1` | plain bodies; two or more byte-identical pairs; rounds with `vitamin d`, `d3`, `c`, `igf 1`, `ii`, possessives | `search_text_only`; queries carry every D3 word, short content tokens and an apostrophe | 10 |
| `projected-v1` | JSON bodies `{"title", "cohort"}`; `title` SEARCHABLE + FTS, `cohort` FILTERABLE; identical titles form a bm25 tie group across the limit | `search_projected_text`, unfiltered and with `cohort = c1` | 5 |
| `dense-v1` | plain bodies; `use_default_embedder`; `ProjectionSpec(name="dense", SEARCHABLE, vector=True)` declared before writes; coverage checked | `search` (no rerank, no graph arm) | 10 |

- **Protocol.** A request with `normalizer: "ordered_hits"` makes a driver
  return `hits: [{id, score, branch}]` per query, in addition to `ids`. A
  request without it is answered exactly as today.
- **New operations.** The drivers gain `engine.search_projected_text`
  (arguments `limit`, `view`, `name`, `filter`) and `engine.search`
  (`limit`, `view`, `use_default_embedder`). The registry moves both from
  `excluded_with_reason` to `measure` with normalizer `ordered_hits`.
- **Oracle.** After each SDK repetition, the coordinator opens the same
  database file with the in-process Python SDK and runs the same calls.
  The coordinator must run in the Python arm's venv: its native digest is
  checked against that arm's.
- **Comparison.** `compare_parity(oracle, observed)` lists every difference
  in query set, hit count, order, id, branch, or the exact score. Any
  difference fails the cell.
- **Scores.** Scores cross the protocol as JSON numbers. Python `repr`,
  JavaScript's shortest round-trip and `serde_json` all round-trip binary64
  exactly, so equal doubles compare equal.
- **Branches.** Branch names are the SDK wire strings (`text`, `vector`,
  `text_edge`, `graph_arm`). Each driver maps its SDK's representation to
  them.
- **Capability tests (Q-E1).** The Python, TypeScript and Rust driver suites
  each pin the capability set to the three current operations
  (`test_capabilities_advertise_only_implemented_operations`,
  `test_capabilities_is_one_protocol_response`,
  `capabilities_advertise_only_implemented_operations`). Implementing the
  two new operations changes that set, so those three assertions need a
  ruling before W4.

### 2d.4 Public-surface contract

These changes are made with W4.

- **`dev/interfaces/python.md`, `typescript.md`, `rust.md` and
  `rust-sdk.md`**, for each verb:
  - `search_text_only`: each fusion arm credits a body once; 1–2
    character tokens are kept; the stop list includes the twelve Unit D3
    words.
  - `search_projected_text`: rows are read in FTS5 rank order with
    boundary-tie completion, and the order contract (bm25 ascending, then
    write cursor ascending) is unchanged.
  - `search`: dense retrieval needs a declared searchable `vector=True`
    projection; a plain write under the default embedder embeds nothing;
    the full node body is embedded.
- **Public docs.** In `docs/guides/structured-search-hits.md`,
  `docs/guides/hybrid-search-filtering.md`, `docs/reference/python-api.md`
  and `docs/reference/typescript-api.md`, `Σ 1/(60 + rank)` becomes
  `Σ_branch w_branch / (30 + rank_branch)`, with `w` = 3 for text and 1
  for vector and graph. Lint with `./scripts/agent-lint-docs.sh`.
- **No doc-checker test.** No checker pins verb descriptions, so there is
  no RED test in this worktree for E4. Review and the docs lint are the
  check.

## 2e. Unit F: a known-run baseline

- **Manifest.** `experiments/configs/baselines/slice-40-baseline.v1.json`
  has schema `fathomdb.baseline.v1`, one `engine_commit` and
  `benchmarks[]`. Each benchmark has `id`, `inputs` (`identifier`,
  `sha256`), `engine_commit`, `wheel_build` (features, `CUDA_PATH`, `PATH`,
  `LIBRARY_PATH`, `compatibility: linux`), `command` (exact argv), `receipt`
  (an ID or `"to be recorded"`) and `metrics[]`.
- **Metrics.** Each metric is `{name, pointer, expected, tolerance, gate}`.
  `pointer` is a JSON Pointer into the attempt's `summary.json` or
  `report.json`. `tolerance` is one of:
  - `{"abs": x}`: nDCG@10 0.002, MRR 0.005;
  - `{"rel": x}`: latency p50 0.25, p95 0.35, quiet host;
  - `{"questions": k, "n": n}`: per-type session recall, k = 1; answer
    accuracy, k = 3, n = 60, `gate: false`.
- **Benchmarks.** COMP-01 NFCorpus (`comp01`), `comp01-hybrid`, the COMP-03
  retrieval sweep (three arms), COMP-03 answer v2, and 10k TC-5 latency
  (`comp02b_latency`). Expected values are first filled from the Unit D
  measurements and marked `provisional`. They are replaced by the
  orchestrator's re-measurement at the final slice head (REQ40-F5); until
  then every `receipt` reads "to be recorded".
- **`experiments/baseline_check.py`.** Pure functions `allowed_delta`,
  `check(entry, summary)` (rows with `name`, `expected`, `measured`,
  `tolerance` and `status` PASS, DRIFT or MISSING), `load_manifest`, and
  `main(argv)`. `main` prints one line per metric and returns 1 if any row
  is DRIFT or MISSING.
- **`experiments/comp02b_latency.py`.** The orchestrator's diagnostic
  script, receipt-producing:
  - build one 10k projected store (`text` SEARCHABLE + FTS, `bench_cohort`
    FILTERABLE);
  - one warm pass, then a seeded shuffled pass per cell;
  - the six `CELLS`, and percentiles exactly as the script (p50 the median,
    p95 `sorted[int(n × 0.95) − 1]`);
  - a summary and an `experiments.record.v1` receipt with engine identity.
- **Engine identity.** `comp_record.engine_identity(commit)` returns
  `{commit, native_sha256, version}`. The comp runners (`comp_fts run`,
  `comp03_retrieval run`, `comp03_answer run`, `comp02b_latency run`)
  require `--engine-commit` and write it into the receipt. Without a
  product accessor, the commit is declared, and the native digest makes the
  declaration checkable.
- **Re-measurement (REQ40-F5).** The pre-D3 sweep and the two NFCorpus runs
  have no receipts and ran on two engine commits. The baseline is measured
  at the final slice head, after Units E to G land, by the orchestrator,
  with receipts.

## 2f. Unit G: evidence budget, session completion and recency

Revised after the Unit G review (REJECT, findings F1–F10, 2026-10-16). The
review replayed the first rule on the 470 scored sweep questions × 3 arms
(pooled counts):

| Policy | any | all | has-answer | untruncated | chars |
| --- | ---: | ---: | ---: | ---: | ---: |
| 4000/20000/off (current) | 1383 | 1225 | 1321 | 1302 | 27.0M |
| 4000/20000/on | 1383 | 1225 | 1325 | 1304 | 28.1M |
| 4000/30000/off | 1388 | 1248 | 1334 | 1328 | 31.3M |
| 4000/30000/on | 1388 | 1248 | 1344 | 1338 | 41.8M |
| 2000/30000/on | 1388 | 1248 | 1353 | 572 | 41.3M |
| 1000/20000/off | 1388 | 1248 | 1334 | 118 | 13.7M |
| 1000/30000/on | 1388 | 1248 | 1369 | 118 | 36.2M |

The has-answer rounds number 893:

- median length 2,534 characters;
- 851 exceed 1,000 characters, 650 exceed 2,000, 8 exceed 4,000.

**Why the first rule failed.**

- `all` is bounded by Recall-all@10 and cannot move with expansion, so that
  rule fell to its fewest-characters tie-break and picked 1000/20000/off,
  which truncates nearly every answer round.
- Its `any` constraint is vacuous, because `any` is monotone in every grid
  direction.

The rule below replaces it.

### 2f.1 G1 and G2, offline (`comp03_retrieval`)

- **Inputs.**
  - The D3 sweep attempt root (`~/.cache/fathomdb-comp/comp03-sweep-d3`):
    each scored question's stored top-10 IDs, all three arms.
  - The dataset: the evaluator re-derives every round body with
    `convert_question`.
  - The verdict binds the dataset SHA-256 and the sweep summary SHA-256.
  - LanceDB's record for `gpt4_7fce9456` predates the blank-round filter
    (253 vs 251 documents). The evaluator renders from converted docs,
    where the two blank rounds contribute only whitespace, so the effect is
    nil.
- **Question sets.**
  - 470 scored questions; abstentions are excluded from every gold metric.
  - Selection uses the scored questions that are not among the 60 answer
    IDs: 414, because four of the 60 are abstentions, so 56 are scored.
  - The answer set (60 IDs, 56 scored) is reported separately, so the paid
    runs are not on the selection set.
  - Every count is derived from the dataset, the sweep records and the v2
    config's question IDs, never hard-coded.
- **`evidence_policy_grid()` and `policy_key(p)`**: twelve policies, keyed
  `"<per_round>/<total>/<on|off>"`.
- **`evidence_rounds(ids, docs, policy)`** returns `[(round_id, chars)]` in
  prompt order:
  - the top-k IDs in rank order;
  - with expansion on, then the remaining rounds of each hit session, the
    sessions ordered by their best hit's rank, rounds by position;
  - each round capped at `per_round_chars`, then cut to what remains of
    `total_chars`, stopping when nothing remains;
  - blank rounds are listed with 0 characters.

  This is `render_evidence`'s arithmetic: date headers are not counted.
- **`evidence_metrics(shown, gold, docs)`** gives, per question:
  - `gold_session_any_in_prompt`, `gold_session_all_in_prompt`;
  - `has_answer_in_prompt`: any has-answer round with at least one
    character shown;
  - `has_answer_untruncated_in_prompt`: any has-answer round shown in full;
  - `gold_chars_shown_fraction`: shown characters over full length, summed
    over the session-gold rounds;
  - `rounds_in_prompt` (rounds with at least one character) and `chars`.
- **`select_evidence_policy(pooled, baseline, fts=None)`** applies REQ40-G2:
  - maximise `has_answer_in_prompt` overall;
  - per type, `has_answer_untruncated_in_prompt` may not fall more than 3
    pooled counts (1 for `fathomdb-fts` alone) below the baseline policy;
  - ties go to fewer characters; when nothing qualifies, the baseline.

  It returns the policy and, per excluded policy, the failing types (as
  `type` for pooled, `fathomdb-fts:type` for the per-arm check). On the
  replay this rule would pick 4000/30000/on; the tests do not hard-code
  that.
- **Output.** `experiments/configs/comp-03/evidence-policy-verdict.v1.json`
  records:
  - the policy, the baseline and the rule;
  - the pooled and per-arm twelve-row tables for the selection set (414)
    and for the answer set (60 IDs, 56 scored);
  - the excluded policies;
  - `answer_set_expectation`: the chosen policy minus the baseline on the
    answer set, pooled and per arm, for every metric, the offline
    expectation the paid v3 result is read against;
  - the dataset and sweep summary SHA-256, the arms, and the question
    counts `{scored, selection, answer_set_ids, answer_set_scored}`.

### 2f.2 One evidence implementation (`comp03_answer`)

- `render_evidence(docs, ids, *, max_item_chars=None, max_total_chars=None,
  policy=None, chronological=False)`. A policy replaces both caps, so passing
  a policy together with either cap raises `ValueError`. Without a policy,
  both caps are required.
  - With `policy` given, the rounds come from
    `comp03_retrieval.evidence_rounds`, imported inside the function
    because `comp03_retrieval` imports `comp03_answer`.
  - Rounds with 0 characters are not rendered.
  - Without `policy` the v1/v2 path is unchanged.
- `evidence_record(case, ids, evidence_config)` returns the rendered text,
  its SHA-256 (`evidence_sha256`, covering the expanded text),
  `rounds_in_prompt` and `chars`. The answer run writes the last two into
  each answer row.
- `chronological=True` renders the selected rounds sorted by
  `(session date, session position, round position)`, newest last, with the
  `[Session date: …]` headers kept.
- `answer_messages(case, evidence, *, recency_instruction=False)`: when
  true, `RECENCY_SENTENCE` is appended to the prompt's first paragraph.
- Both off gives the v2 bytes.
- `evaluate_recency_rule(base_rows, new_rows, arm, seed)` pairs by question
  ID. It returns:
  - the per-type correct counts for both runs;
  - the knowledge-update plus temporal delta;
  - the overall delta with its bootstrap CI;
  - the verdict: adopt iff the KU plus temporal delta ≥ 0 and the overall
    delta ≥ −1 question.

### 2f.3 Configs and the best mix

- **Schemas.** v3, v4a, v4b and v4c use `comp-03.answer.v3`. Each is v2 plus
  `evidence {policy, policy_verdict_sha256, chronological,
  recency_instruction}` and `decision_rule {compare_to, rule, provisional:
  true, mde_net_discordant: 6}`:
  - v3: the verdict's policy, both flags off, `compare_to` v2 (report
    only);
  - v4a: v3 with `recency_instruction` true, `compare_to` v3;
  - v4b: v4a with `chronological` true, `compare_to` v4a;
  - v4c: v3 with `chronological` true alone, `compare_to` v3, run only in
    the REQ40-G6 contingency.
- **Arm of record.** Every decision rule names `arm_of_record:
  fathomdb-fts`; the other arms are reported.
- **Statistical caveat.** At n = 60 paired, the minimum detectable effect is
  about 6 net discordant questions, so every answer-run verdict is
  provisional. Adoption into the mix is gated on the per-type direction plus
  the 470-question offline metrics, not on significance.
- **`build_mix_config(v3_config, policy_verdict, v4a_verdict, v4b_verdict,
  v4c_verdict=None, *, arm="fathomdb-fts")`** is pure and returns
  `hermes-answer.mix.json`. The factor verdicts are per arm (`{"sha256",
  "arms": {arm: {"verdict", …}}}`), and only the named arm decides.
  - The verdict's policy.
  - `recency_instruction` iff v4a adopted on the arm.
  - With the sentence on, `chronological` iff v4b adopted.
  - With the sentence off and v4b adopting, `chronological` iff v4c
    adopted. Without a v4c verdict in that case the function raises.
  - `decision_rule {compare_to: hermes-answer.v2, arm_of_record, sources}`,
    with the verdicts' SHA-256 in `sources`.
- **Commands.** `comp03_answer recency-verdict` writes a factor verdict from
  two run summaries; `comp03_answer mix` writes the mix file. The
  orchestrator runs both after the paid runs.
- `load_config` accepts v1, v2 and v3 schemas. Receipts and
  classifications are made as for v2.

### 2f.4 Harness-side now, product-side later

| Lever | Harness (this slice) | Product equivalent (owner decision) |
| --- | --- | --- |
| Evidence budget | per-round and total caps in `render_evidence` | none; a caller concern |
| Session completion | session expansion after top-k in the harness | turn-level retrieval units in the engine, plus session expansion as a retrieval option |
| Recency | chronological rendering and one prompt sentence | a recency-aware ranking pass by fact date in the engine; the existing opt-in G12 recency reweight orders by write cursor, not by session date |

## 2g. Unit H: stop list, vector pool, cohort prefilter

### 2g.1 Order of work

**H1 superseded** (owner ruling 2026-10-16): the H1 words are not added to
the default `STOPWORDS`, which stays at 63 words. Slice 41 adds a
configurable extra stop-word list, empty by default, and the benchmarks pass
`had`, `long` and `suggest` through it. The H1 selection (steps 1 and 2)
ran and its verdict stays as evidence; the H1 engine steps below are struck.

H1 and H2 each needed a measurement before the engine change, so the
RED/GREEN order is:

1. Harness RED, then GREEN: `experiments/comp03_unit_h.py`, with its pure
   rule functions and the `stoplist` and `pool` commands. In the same
   step, the baseline manifest gains `comp02a-10000`, `comp02a-17272` and
   `comp02b-10000` (owner ruling Q-H5), with the manifest test renamed
   for nine ids and its new assertions RED first.
2. The selection runs:
   - `stoplist` runs on CPU, implementer;
   - `pool` runs on cuda:0, orchestrator, with a CUDA wheel built at
     `5b09afd44` (ruling Q-H4). An implementer worktree creates no GPU
     environment. The verdict records who ran it.

   Both verdict files are committed on the harness branch.
3. Engine RED, written against the verdicts: `slice40_vector_pool.rs` and
   `slice40_cohort_prefilter.rs` (~~the `fathomdb-query` H1 tests~~,
   superseded).
4. Engine GREEN in two commits, H2 then H3, so each gate can revert its
   own commit. The interface-doc and CHANGELOG edits for H2 go in the same
   commit as its code, because it changes documented public behaviour.
5. Scoped checks, plus the report-only c10 timing of § 2g.4, then stop
   for the code review.
6. The orchestrator measures each gate at its own commit, in ladder order
   (requirements, the Unit H gate table): H2 at the H2 commit against the
   final-head baseline, H3 at the H3 commit against H2. That costs one
   COMP-02b run more than measuring only at the end; it is the only way a
   failed gate can be traced to one commit. The H1-commit hybrid sweep at
   pool = limit is no longer run.
   - The H2 measurement also needs one extra limit-`P0` hybrid pass over
     the 500 sweep stores, on cuda:0 at the H2 commit, for REQ40-H6 (iv)
     (the sweep itself runs at limit 10). Budget it with the H2 sweep
     (re-review N3).

### 2g.2 H1, stop list

- **Harness (`comp03_unit_h.stoplist`).** For each sweep question, build
  the `fathomdb-fts` store once with `comp03_answer.convert_question`,
  plain bodies and no embedder. Run `search_text_only(limit=10)` for:
  - the D3 baseline (the question verbatim);
  - each candidate alone;
  - each combined list the pruning loop asks for.

  NFCorpus works the same way over one store, with `comp_fts.score_query`
  for nDCG@10 and MRR@10. Both corpora rebuild in under a minute on CPU.
  The pruning loop needs one query pass per step, not one rebuild.
- **The removal transform.** Lowercase the query, split it on
  non-alphanumeric characters, drop the words, and join with spaces.
  - The harness keeps a Python copy of the `5b09afd44` stop list in the
    config (`engine_stopwords`, 63 words). Two tests pin it:
    `test_stoplist_selection_config_states_the_candidates_and_rule`
    against the verbatim list, and the engine's
    `unit_h_words_are_in_the_stop_list_and_content_tokens_are_not`
    against `STOPWORDS.len()`.
  - The harness uses that copy to find queries that would lose every
    content token, against every list the rule evaluates: each word
    alone, and each pruning step's `L`. The simulation does not cover
    such a query, so the rule prunes until none remains (REQ40-H1).
  - `test_stripping_words_equals_compiling_with_them_as_stopwords` pins
    the equivalence as a `hypothesis` property.
- **Clause (c).** The combined list's paired Recall-all@10 delta against
  D3 goes through `comp03_retrieval.bootstrap_ci` (seed 404, 10,000
  resamples). Owner ruling N1 (2026-10-16), verbatim: *point delta ≥ +1
  question AND paired-bootstrap CI lower bound ≥ −1 question
  (bootstrap_ci, seed 404, 10,000 resamples) on Recall-all@10 over the
  selection set.* Both are required.
- **Engine (superseded, owner ruling 2026-10-16; Slice 41 makes the list
  configurable instead).** Append the verdict's words to `STOPWORDS`, and extend the
  doc comment with the reason: high-document-frequency question words,
  chosen by the Slice 40 H1 rule. Nothing else in `fathomdb-query`
  changes.
- **Self-check.** At the H1 gate, the post-change sweep must reproduce the
  simulation's `fathomdb-fts` IDs for every question (REQ40-H3 (i)). A
  difference would mean the simulation and the compiler disagree. The
  gate then fails, rather than recording a number the rule did not
  choose.

### 2g.3 H2, vector pool

- **Harness (`comp03_unit_h.pool`).** For each sweep question, build the
  hybrid store once, through the same functions `comp03_retrieval` uses
  (store build, device check, vector coverage), imported, not copied.
  Then call `comp03_answer.retrieve` with the `fathomdb-hybrid` arm at
  `top_k = P` for P ∈ {10, 20, 50, 100}, recording the IDs. Pool `P0` at
  k is `ids[P0][:k]` (§ 1.6), so the rule needs no re-fusion and no copy
  of the engine's fusion.
- **Engine (`search.rs`).** One line changes, in the vector hydration loop:

  ```rust
  // before
  if results.len() >= final_limit { break; }
  // after
  if results.len() >= final_limit.max(VECTOR_FUSION_POOL) { break; }
  ```

  `VECTOR_FUSION_POOL` is a `pub(crate) const` in `search_types.rs`, next
  to `TOP_K_BIT_CANDIDATES`, with a `test-hooks` accessor
  `vector_fusion_pool_for_test()` for the engine tests (review F10). It
  is always ≤ 192, so phase 1 and phase 2 already hold enough rows and no
  SQL changes.
- **What does not change:**
  - fusion gets the longer vector list and truncates to `final_limit` as
    before;
  - the `vector_stage_only` seam returns `vector_results` and is
    truncated to `final_limit`, so its output is unchanged;
  - rerank, the graph arm and the reweights read the fused list, which
    no longer depends on `limit`. Their outputs can change only where the
    vector list grew.
- **Why a constant floor and not a function of `limit`.** With
  `max(limit, P0)`, every limit up to `P0` sees the same vector list,
  which gives the prefix property. Limits above `P0` keep today's
  behaviour.
- **Explain.** `vector_hits` already counts the hydrated rows, so it now
  reports the pool.
- **Cost.** Each extra hydrated row is one indexed `canonical_nodes`
  lookup by `write_cursor` (`canonical_nodes_write_cursor_idx`). The
  comment in the loop that says there is no such index is out of date;
  W4 corrects it.

### 2g.4 H3, cohort prefilter

- **Where.** `read_projected_text_in_tx` (`search.rs`), before the Defect C
  stream. The steps:
  1. If `filter.attributes` is non-empty and the switch is not forced
     off, probe each pair with
     `SELECT count(*) FROM (SELECT 1 FROM canonical_attributes WHERE
     attr_name = ?1 AND attr_value = ?2 LIMIT 257)`.
  2. Take the first pair with the smallest count. If that count is
     ≤ 256, or the switch is forced on, run
     `property_fts_cohort_stream_sql` (new, `filter.rs`).
  3. Feed its rows to `collect_complete_rank_boundary(limit)`, as the
     Defect C stream does.
  4. If the statement fails, record `projected_cohort_stream_fallback`,
     then continue to the Defect C stream (which records its own route)
     and then the full sort, unchanged.
- **The new statement.** It is `property_fts_rank_stream_sql` plus one
  conjunct before `AND rank MATCH`:

  ```sql
  AND p.write_cursor IN (SELECT write_cursor FROM canonical_attributes
                         WHERE attr_name = ?{a} AND attr_value = ?{v})
  ```

  - `?{a}` and `?{v}` bind after the eligibility parameters.
  - The conjunct references only `p`, so SQLite tests it in `p`'s loop,
    before the `canonical_nodes` join.
  - SQLite builds the list subquery once, from the name-value index.
- **Why a prefiltered stream and not a sorted cohort.**
  - The probe in § 1.6 timed both. The sorted form was slower at every
    cohort size (8.6 against 7.7 ms at c01, 11.0 against 4.7 ms at c10).
  - The prefiltered stream reuses Defect C's tie completion. Result
    identity then follows from the conjunct being implied by the
    existing `EXISTS`; no second ordering implementation is needed.
- **Witnesses** (`test-hooks`), as for Defect C:
  - `record_fts_route_for_test` with
    `projected_cohort_stream_strict_boundary` or
    `projected_cohort_stream_tie_completed`;
  - `record_fts_query_plan_for_test` on the cohort statement;
  - `FATHOMDB_FTS_COHORT_PREFILTER_FOR_TEST` (`on` or `off`) and
    `FATHOMDB_FTS_FAIL_COHORT_FOR_TEST`, the latter replacing the cohort
    subquery's table with a missing one, so the statement fails to
    prepare.
- **If the plan witness fails** (review F9). The probe ran on Python's
  SQLite 3.45.1; the engine bundles a newer SQLite through
  `libsqlite3-sys`. With the `IN` conjunct, the planner may drive the loop
  from the list (per-cursor lookups) and sort by `rank` itself. That is
  correct, but it adds `USE TEMP B-TREE FOR ORDER BY`, and AC40-84 goes
  RED. The fallback, in this order:
  1. write the conjunct as `+p.write_cursor IN (...)`, which keeps the
     planner from using it as an index constraint;
  2. otherwise, an `EXISTS(SELECT 1 FROM canonical_attributes ca WHERE
     ca.write_cursor = p.write_cursor AND …)` keyed on `p` (probe row 4).

  Report which form shipped. The plan witness is never dropped to reach
  GREEN. Result identity (AC40-85, AC40-90) holds in every form.
- **Report-only c10 timing** (review F12, ruling Q-H3). An `#[ignore]`d
  test in `slice40_cohort_prefilter.rs`, `cohort_prefilter_timing_report`,
  times the cohort and Defect C routes at c01 and c10. It builds a store
  from `FATHOMDB_TC5_INPUTS` (the `tc5-10000` input pack) and forces each
  route with the `test-hooks` variable. The implementer runs it once
  before the code review and records the numbers in the test plan. `T`
  is not retuned from them without a ruling.
- **Not changed:**
  - the hybrid `search` path's attribute filtering;
  - `search_text_only`;
  - the attribute `EXISTS` keying (the row-key potential), unless the
    plan-witness fallback above needs it.

### 2g.5 Risks

| Risk | Effect | Handling |
| --- | --- | --- |
| A selected stop word carries meaning in some corpus ("long", "number", "total") | Recall loss there | The rule keeps only words with a measured net gain on COMP-03 and checks NFCorpus; the gate checks both corpora again on the engine |
| H1 changes hybrid's text branch as well | Hybrid numbers move before H2 | Moot: H1 is superseded (owner ruling 2026-10-16) and makes no engine change |
| Two selected words empty a query only jointly | The engine takes the raw fallback where the simulation did not | The emptiness check runs on every list the rule evaluates (REQ40-H1) |
| The pool is chosen on the `5b09afd44` engine | The H2 engine differs only by the pool | The H2 gate measures at the H2 commit, against the final-head baseline, and steps the pool down on failure |
| The bundled SQLite plans the cohort statement differently from 3.45.1 | AC40-84 RED on the plan witness | The fallback forms in § 2g.4; the witness stays |
| A larger pool lowers multi-session Recall@10 (analysis: .810 → .777 at 100) | The adopted hybrid arm changes | The rule picks the largest pool that keeps every type within one question; 10 is always available |
| Pool hydration adds latency | Up to `P0 − limit` extra indexed lookups per hybrid query | Latency is a gate clause; the pool steps down on failure |
| Existing hybrid tests encode pool = limit | They fail at GREEN | Analysed at GREEN and reported for a ruling; not edited to pass. The review found none that pins `vector_hits == limit` (review F15) |
| The cohort probe costs time on large cohorts | At most 257 index entries per attribute pair | The gate checks c10, c33 and unfiltered |
| A stale attribute row (superseded node) inflates the probe count | A plan choice only | The node predicates still apply; AC40-89 |

### 2g.6 Docs changed by Unit H

Each edit lands in the commit whose behaviour it describes.

- `dev/interfaces/{python,typescript,rust,rust-sdk}.md`, "Search behaviour
  in 0.8.28 (Slice 40)":
  - ~~the stop-list sentence lists the H1 words~~ (H1 superseded);
  - a pool bullet under dense retrieval;
  - a cohort-prefilter sentence under `search_projected_text`, with the
    order contract unchanged.
- `docs/guides/hybrid-search-filtering.md`: one sentence on the vector
  pool and the prefix guarantee.
- `CHANGELOG.md`: three entries under 0.8.28:
  - ~~wider query stop list~~ (H1 superseded);
  - hybrid results prefix-stable up to the pool;
  - faster projected search under selective attribute filters.
- `dev/plans/0.8.28/features/slice-40/status.md`: the filter-first
  potential moves to Unit H; the row-key potential stays.

## 2h. Unit I: one page cache per connection

### 2h.1 Mechanism

Requirements § 1.7 gives the source reading and the probes. In short:

- `-DSQLITE_ENABLE_MEMORY_MANAGEMENT` forces `pcache1` into one global
  page-cache group for every connection in the process.
- A cache that is at its own `cache_size` limit recycles the group's
  least recently used page, which can belong to another connection. So
  the reader that runs a heavy read takes the others' pages.
- Once the group holds its sum of limits, a page unpinned by a cache
  below its own limit is freed at once. The robbed readers stay at
  about one page until the large cache gives pages back.

The defect is the build flag, not FathomDB's reader code. No reader
setting fixes it inside mode 2:

- `cache_size` changes only the group's sum and the point at which a
  cache starts recycling (§ 1.7: `-2000` reproduces it, `-8000` delays
  it).
- `PRAGMA shrink_memory` or `sqlite3_db_release_memory` empties every
  connection's cache.

### 2h.2 Options

| Option | What it does | For | Against |
| --- | --- | --- | --- |
| **A. Drop the flag** (chosen) | Remove one line from the vendored `build.rs`; `pcache1` then gives each connection its own group | Removes the cause. No runtime code. Each connection keeps its own 2,000 KiB, so the ceiling is unchanged. No group mutex on page fetch and unpin. Tegra and Jetson build the same script | Changes the vendored-tree digest (three pins). Reaches only builds from this repository, not published Rust crates (§ 2h.4) |
| B. `LIBSQLITE3_FLAGS=-U…` in `.cargo/config.toml` `[env]` | The same compile result without editing the vendored crate | Same effect as A, no digest change | The flag lives outside the authenticated vendored tree. A caller's environment can override it, and the checker does not authenticate it. Same reach as A |
| C. Fixed `PRAGMA cache_size` on every reader | Sets each reader's limit | One line in the reader open loop | Does not fix it (§ 1.7). A larger value moves the threshold and raises the ceiling by eight times the increase |
| D. Shrink after a heavy read | `sqlite3_db_release_memory` when a reader's `CACHE_USED` passes its budget | Works with the registry crate | Empties every connection's cache, so every reader starts cold. Needs a `CACHE_USED` probe after each read. Treats the symptom |
| E. Custom page cache (`SQLITE_CONFIG_PCACHE2`) | Install FathomDB's own `sqlite3_pcache_methods2` before `sqlite3_initialize` | Works with any `libsqlite3-sys`; could also remove the global mutex | 400–600 lines of unsafe FFI by the 0.7.0 estimate (`ADR-0.7.0-ac020-architectural-lever.md`, option 1). It re-implements SQLite's cache and adds a crash risk on the read path. That ADR's premise, that canonical SQLite cannot drop the `pcache1` hit-path mutex without a custom allocator, holds only under the build flag; see the successor note [ADR-0.8.28-sqlite-per-connection-page-cache.md](../../../../adr/ADR-0.8.28-sqlite-per-connection-page-cache.md) |
| F. `SQLITE_CONFIG_PAGECACHE` | A start-time page buffer | — | Does not change the group choice: under the flag, `separateCache` is 0 regardless |
| G. Soft heap limit | `sqlite3_soft_heap_limit64` puts the cache under pressure, so every cache recycles from the group | No build change | Has no effect today: the malloc alarm runs only with memory statistics on, and the default `Performance` mode turns them off. In `Diagnostics` mode the whole process would be capped, embedder included |

Option A is chosen. Option D or E is the only route to published Rust
crates without their build flags. Ruling Q-I1 (option (a), document
only) wants neither for Unit I. The review (F3) also rejected:

- a published `libsqlite3-sys` fork, which needs a `rusqlite` fork and a
  different `links` key, so a consumer that also uses stock `rusqlite`
  would link two SQLite images with colliding symbols;
- a build-script check, because `fathomdb-engine`'s build script cannot
  see `libsqlite3-sys`'s flags. Only a runtime read
  (`sqlite3_compileoption_used`) works, and a surface for it is a ledger
  follow-up.

### 2h.3 The change

Option A matches upstream. The flag entered rusqlite's bundled build in
rusqlite PR #197 (merged 2016-12-31), copied from System.Data.SQLite's
compile-option list with no rusqlite-specific reason. Upstream removed it
on master in PR #1900 (commit
`f31f7324a82894be13e5de1665cb9af45dc5f869`, merged 2026-09-04: it "forces
the unified page cache … nothing in rusqlite itself needs it"). No
`libsqlite3-sys` release on crates.io contains the removal yet (0.38.2
and `rusqlite` 0.40.2 predate it). The vendored change is therefore
temporary. It is dropped, with the rest of this vendored crate's
reasons reviewed, when FathomDB moves to a `libsqlite3-sys` release
that contains `f31f7324`. That exit condition is written into
`FATHOMDB-PATCH.md` and the ledger.

- `third_party/libsqlite3-sys-0.38.1/build.rs`: delete
  `.flag("-DSQLITE_ENABLE_MEMORY_MANAGEMENT")` in the bundled branch.
  The SQLCipher and WASI branches do not add it, so nothing else
  changes.
- Pins (REQ40-I1): recompute the vendored-tree digest with the
  checker's `vendor_tree_digest` (all files but `FATHOMDB-PATCH.md`,
  sorted relative paths, each file's SHA-256). Write it to:
  - `FATHOMDB-PATCH.md`;
  - `scripts/pinned-override-rot.json` (`vendor_tree_sha256`, plus one
    sentence in `rationale`);
  - `SQLITE_VENDOR_TREE_SHA256` in `scripts/check-pinned-override-rot.py`.

  The amalgamation digest does not change. `FATHOMDB-PATCH.md` gains a
  section "Per-connection page caches". It gives the removed flag, the
  `pcache1Init` comment that explains it, the upstream removal (rusqlite
  PR #1900, commit `f31f7324a82894be13e5de1665cb9af45dc5f869`), the exit
  condition (drop this change when the pinned `libsqlite3-sys` release
  contains that commit), and a pointer to this design.
- The flag is not replaced by a FathomDB compile-option marker. The
  Windows patch needed `FATHOMDB_WIN_VERBATIM_SHM_SAFE` because no
  SQLite option told the two builds apart.
  `sqlite3_compileoption_used("ENABLE_MEMORY_MANAGEMENT")` already
  does, and AC40-95 reads it.
- No engine source changes. `open.rs`, `reader_pool.rs` and
  `connection_runtime.rs` stay as they are (Q-I1 ruled (a)).

### 2h.4 Public surface and docs

- **No knob.** Each connection's limit stays SQLite's default, and the
  sum is today's. A reader `cache_size` knob would be a public-surface
  change needing parity work in three SDKs and an interface update. It
  would also be a memory decision for Tegra and Jetson targets.
  Neither is needed to fix the defect. If a larger reader cache is
  wanted later, it needs its own measurement and ruling.
- **Memory ceiling.** Before: up to the group sum in total (twelve
  2,000 KiB budgets per engine by default, about 24 MB with the
  per-page header; requirements § 1.7), all of which one reader can
  hold. After: each connection up to its own 2,000 KiB, and the same
  total. On the 10k TC-5 store the eight readers can now hold about
  16 MB of distinct pages between them. Before, one reader held most of
  the group and the others held almost nothing.
- **Resident memory can rise** (review F2). In mode 2 idle connections'
  pages are recycled by the active one, so resident page memory tends
  toward the active working set. In mode 1 each connection keeps up to
  its own budget, so a long-running engine can approach the unchanged
  ceiling of about 24 MB. On Tegra and Jetson that is small, but it is
  stated in the CHANGELOG.
- **Docs** (REQ40-I4, in the Unit I commit):
  - `dev/interfaces/rust.md` and `dev/interfaces/rust-sdk.md`, one
    paragraph under "SQLite runtime configuration";
  - `CHANGELOG.md` `### Fixed`: *"A large read no longer slows later
    reads on the same engine. Each SQLite connection now has its own
    page cache (the bundled build drops
    `SQLITE_ENABLE_MEMORY_MANAGEMENT`). The page-cache ceiling is
    unchanged (2,000 KiB per connection, twelve connections per engine
    by default), but resident page memory can now rise toward it, up to
    about 24 MB per engine. Rust builds that resolve the registry
    `libsqlite3-sys` need
    `LIBSQLITE3_FLAGS=-USQLITE_ENABLE_MEMORY_MANAGEMENT` for the same
    behaviour."*;
  - `dev/adr/ADR-0.8.28-sqlite-per-connection-page-cache.md` (new,
    successor note to the 0.7.0 AC-020 ADR's page-cache premise), a
    dated addendum in `ADR-0.7.0-ac020-architectural-lever.md` pointing
    to it, and a decision-index row;
  - `dev/design/pinned-override-rot-guard.md`: the vendored crate now
    carries two changes;
  - `FATHOMDB-PATCH.md` (§ 2h.3).

  The Python and TypeScript interface docs do not change. Their
  artifacts are built in this workspace and no call changes.
- **Published Rust crates.** Their builds keep the shared cache unless
  the consumer sets the flag. The defect is a latency loss; results are
  correct. Unit I documents the flag and stops there (Q-I1 ruled (a)).
  The upstream removal reaches them with the first `libsqlite3-sys`
  release that contains `f31f7324`; the `LIBSQLITE3_FLAGS` override
  covers the gap.
  A consumer-facing report of the effective mode is a ledger follow-up.

### 2h.5 Tests

- `tests/slice40_reader_page_cache.rs` (new, `test-hooks`, `[[test]]`
  entry in `fathomdb-engine/Cargo.toml`):
  - **AC40-95**: reads `rusqlite::ffi::sqlite3_compileoption_used`;
  - **AC40-96 and AC40-97**, the store and the reader passes:
    - eight warm sets of 40 documents with about 3 KB bodies, each
      found by its own term;
    - one heavy set of 100 documents with about 40 KB bodies, found by
      one term;
    - drained, closed, reopened;
    - `next_reader_worker_index_for_test` pins each query to a reader:
      eight consecutive queries visit the eight readers in order;
    - the counters come from `cache_status_per_worker_for_test`.

    The same shape as the engine probe in § 1.7 goes RED today on both
    assertions and GREEN with the flag undefined. The counter tests
    share a file-level `Mutex`, as the Defect C and H3 files do: in
    mode 2 every connection in the test process shares one group, so
    parallel tests would perturb each other's counters (review F4).
- `reader_page_cache_timing_report` (`#[ignore]`, REQ40-I5) in the same
  file. It follows the `diag_order_effect` probe in `diag-lat/`.
- The pinned-override checker and its fixture tests cover the digests
  (AC40-98).
- No existing test changes. The only other user of these counters is
  the `#[ignore]`d G.3.5 diagnostic in `perf_gates.rs`, which asserts
  only the reader count.

### 2h.6 Measurement

The orchestrator measures three engines with the harness fixed in
`comp03-slice40` (fresh open per cell and per cohort):

- `5b09afd44`, the baseline (for the record);
- the H3 commit (the reference);
- the Unit I commit.

REQ40-I6 is the gate. The fixed harness measures the clean state, so it
shows only side effects of Unit I there, such as the dropped group
mutex. The defect itself shows in the timing report (gate clause (i)).
It shows in a single-open harness too, which the fix retired on
purpose: every number there depended on cell order.

### 2h.7 Slice 41

Slice 41 also edits the engine open path (the configurable extra
stop-word list) and rebases onto Slice 40. Unit I as drafted touches
`build.rs`, the three pins, docs and a new test file, but no open-path
source, so the rebase sees no source conflict there. Two things change
under Slice 41 after the rebase:

- the vendored-tree digest;
- latency baselines measured before Unit I. Slice 41 benchmark numbers
  taken on a pre-Unit I engine are not comparable with numbers after
  the rebase, so any Slice 41 receipt is taken after the rebase.

### 2h.8 Risks

| Risk | Effect | Handling |
| --- | --- | --- |
| Mode 1 frees memory less eagerly across connections | An idle reader keeps its 2,000 KiB while another could use it | Bounded by the unchanged per-connection limit; total unchanged |
| A connection relied on recycling other caches' pages | Its own hit rate drops to what its own 2,000 KiB gives | That is the design point of `cache_size`; the gate measures every benchmark |
| The engine probe's numbers depend on fixture sizes | The regression test could pass for the wrong reason | AC40-97 also asserts a zero-miss pass before the heavy read; AC40-96 bounds the heavy reader; AC40-95 pins the build |
| Published Rust crates keep the defect | Rust consumers see the slowdown | Documented flag; Q-I1 |
| A later `libsqlite3-sys` bump reintroduces the flag | The defect returns | AC40-95 fails on any build that has it |

## 2i. Unit J: guard raw-first tests against the runtime latch

- **Mechanism** (requirements § 1.8). The first SQLite initialization in
  a process decides whether FathomDB can still configure the runtime. A
  test that opens a raw `rusqlite` connection first latches `TooLate`
  for every later `Engine::open` in that binary.
- **Choice: option A of the investigation.** A test-side
  `configure_runtime(RuntimeSqliteMode::Performance)` before the raw
  open, the convention of `pr_g0_identity.rs`. The tests still run in
  `Performance`, the mode an implicit open would choose.
- **Rejected:**
  - (B) tolerating an initialized SQLite on the implicit path records an
    unreadable mode and runs silently in `Diagnostics`, which the
    accepted ADR rejects;
  - (C) a process-start constructor removes the embedder's choice of
    mode;
  - (D) serializing the binaries hides the ordering and slows the suite;
  - (E) routing the SQL through an engine hook changes what the tests
    assert.
- **Where.** The five sites of REQ40-J1. In each file, add a
  `configure_test_runtime()` helper with the latch comment from
  `pr_g0_identity.rs`. Call it at the top of the raw-first test, or at
  the top of `wal_pair` in `tc57_governed_write_race.rs`. Only the
  raw-first tests are guarded: once any `Engine::open` or explicit call
  has run, the state is latched `Configured`, and later raw opens are
  harmless.
- **Message and docs** (REQ40-J2, J3): one `Display` arm in
  `runtime_configuration.rs`, and one sentence each in
  `docs/reference/config.md` and `dev/interfaces/rust.md`. The
  `RuntimeConfigurationError` doc comment ("Changing modes requires a
  process restart") stays true and is not changed.
- **Interaction with Unit I.** None in code. Unit I is a compile-time
  change, so it does not move when `sqlite3_config` returns `MISUSE`.
  Unit J lands first so that Unit I's test runs cannot fail for this
  reason.
- **Not in Unit J:** `slice135_robustness_first`'s fd-count failure (a
  different defect, pre-existing); a lint for unguarded raw opens (an
  owner call, not requested).

## 3. Files touched

| File | Change |
| --- | --- |
| `src/rust/crates/fathomdb-engine/src/fusion.rs` | per-arm credited set in `fuse_three_arms`; doc comment states "each arm contributes at most once per body, at its first occurrence's rank" |
| `src/rust/crates/fathomdb-query/src/lib.rs` | `is_empty()` in place of `len() < 3`; doc comments of `content_tokens` and `compile_text_query` |
| `src/rust/crates/fathomdb-query/README.md` | "tokens shorter than three characters" removed |
| `src/rust/crates/fathomdb-engine/src/search.rs` (Defect C, W4 of phase 3) | `read_projected_text_in_tx`: stream first, fallback second, score negation, `test-hooks` route and plan witnesses |
| `src/rust/crates/fathomdb-engine/src/filter.rs` (Defect C) | new `property_fts_rank_stream_sql`; `property_fts_rank_sql` unchanged |
| `src/rust/crates/fathomdb-query/src/lib.rs` (Unit D3, own commit) | twelve words added to `STOPWORDS` |
| `comp03-slice40`: `experiments/comp03_retrieval.py` (new), `configs/comp-03/retrieval-sweep.v1.json` (new), `configs/comp-03/hermes-answer.v2.json` (new), `comp03_answer.py`, `comp_fts.py` (new arms and campaign only), `fathomdb_test_setup.py` (`require_embed_device`) | Unit D1, D2 |
| ~~`src/rust/crates/fathomdb-query/src/lib.rs` (Unit H1, own commit)~~ | superseded: no `STOPWORDS` change on Slice 40 (owner ruling 2026-10-16) |
| `src/rust/crates/fathomdb-engine/src/search_types.rs`, `search.rs` (Unit H2, own commit) | `VECTOR_FUSION_POOL`; the vector hydration loop stops at `max(final_limit, VECTOR_FUSION_POOL)`; the stale index comment corrected |
| `src/rust/crates/fathomdb-engine/src/search.rs`, `filter.rs` (Unit H3, own commit) | bounded cohort probe, `property_fts_cohort_stream_sql`, cohort route and plan witnesses |
| `comp03-slice40`: `experiments/comp03_unit_h.py` (new), `configs/comp-03/stoplist-selection.v1.json`, `vector-pool-selection.v1.json` and their verdicts (new) | Unit H1 and H2 selection |
| `third_party/libsqlite3-sys-0.38.1/build.rs`, `FATHOMDB-PATCH.md`; `scripts/pinned-override-rot.json`, `scripts/check-pinned-override-rot.py` (Unit I, own commit) | drop `SQLITE_ENABLE_MEMORY_MANAGEMENT`; new vendored-tree digest in the three pins |
| `src/rust/crates/fathomdb-engine/tests/slice40_reader_page_cache.rs` (new), `fathomdb-engine/Cargo.toml` `[[test]]` (Unit I) | AC40-95 to AC40-97 and the report-only timing |
| `dev/interfaces/rust.md`, `dev/interfaces/rust-sdk.md`, `dev/design/pinned-override-rot-guard.md`, `CHANGELOG.md` (Unit I) | per-connection page caches; published-crate reach |
| `comp03-slice40`: `experiments/comp02b_latency.py`, `comp_fts.py`, `comp03_classification.py`, `configs/comp-02/filtered-10000.v2.json` (new) (Unit I harness, `d52d5d4a5`) | fresh open per latency cell and per COMP-02b cohort; summary v2, config v2 |
| `tests/pr_g8_dangling_edges.rs`, `tc57_governed_write_race.rs`, `pr_g3_read_collection.rs`, `truncate_wal.rs`; `src/runtime_configuration.rs` (`TooLate` message); `docs/reference/config.md`, `dev/interfaces/rust.md` (Unit J, own commit, before Unit I) | runtime guard before raw-first opens; remedy in the message and docs |
| `dev/adr/ADR-0.8.28-sqlite-per-connection-page-cache.md` (new), addendum in `ADR-0.7.0-ac020-architectural-lever.md`, decision-index row 64 (Unit I docs) | successor note on the page-cache premise |
| `CHANGELOG.md` | two entries under "0.8.28 — unreleased" (R1 F3): fusion credits a body once per arm (result order can change on corpora with duplicate bodies); the compiler keeps 1–2 character tokens |

Defect C tests (phase 3): `tests/slice40_projected_fts_stream.rs` (new)
and its `[[test]]` entry in `fathomdb-engine/Cargo.toml`
(`required-features = ["test-hooks"]`). The CHANGELOG gains a third Fixed
entry for Defect C at its W4.

Already changed in W2 (tests, see [test-plan.md](test-plan.md)):
`tests/slice40_fts_ranking.rs` (new), the `fathomdb-query` test module,
and `tests/fts5_injection_safety.rs` (AC-038: reference mirror, two
short-token corpus rows, short tokens in the query alphabet).

## 4. Risks

| Risk | Effect | Handling |
| --- | --- | --- |
| Hybrid and text ranking shift on corpora with duplicate bodies | Duplicates fall back to their FTS rank; non-duplicate corpora are byte-identical (AC40-09) | Intended. Agent-memory corpora (Memex) hold many duplicates; note it in the 0.8.28 release notes as a ranking fix |
| Recall harness baselines (eu7, IR-C) move | A number recorded on a duplicate-bearing corpus can shift | eu7 measures the vector stage only (`vector_stage_only` skips fusion); others are experiment records, not gates |
| Short tokens add OR terms | More FTS5 work per query; tail latency rises slightly on long queries | The measured gain (0.3228 to 0.3260) outweighs it; the stop list stays, which was the large latency factor |
| Short tokens add false matches (single letters) | Small; bm25's IDF discounts frequent letters | Measured net gain on NFCorpus |
| Possessive and contraction fragments become OR terms (R1 F2) | "Cow's Milk" compiles to `"cow" OR "s" OR "milk"`, "don't" to `"don" OR "t"`. "s" and "t" occur in most English rows, so they enlarge the OR candidate set (latency) with near-zero IDF signal | Measured on NFCorpus: nDCG@10 0.3228 to 0.3260 and p95 2.24 to 2.30 ms with short tokens kept, so the net is positive. No stop-list change here; pinned by `possessive_and_contraction_fragments_are_content_tokens` and `text_only_possessive_query_ranks_topical_row_first`; adding s, t, ll, re, ve to `STOPWORDS` is a follow-up needing its own measurement |
| An existing test encodes the old behaviors | It fails at W4 | W5 runs the full gate; a failing test is analysed, not edited to pass, unless it encodes the ruled-defective behavior (then reported) |
| Explain and fusion ranks disagree | — | Positional first-occurrence rank keeps them equal (§ 1.2) |

## 5. Docs that describe the old behavior

Grep run on 2026-10-15 over `dev/adr`, `dev/interfaces`, `dev/design`,
`docs/` and the crate sources for "keyed on body", "RRF", "stopword",
"3 char", "short token", "shorter than three", "≥3", "sub-3":

- **Must change (W4):** `fathomdb-query/README.md` line 23 ("tokens shorter
  than three characters"); `content_tokens` doc comment ("≥3 chars");
  `compile_text_query` doc comment ("sub-3-char queries"); the
  `fuse_three_arms` doc comment (add the once-per-arm rule).
- **Already correct, no change:** `dev/design/slice-10-design.md` § 2 (one
  term per branch); `docs/guides/hybrid-search-filtering.md` ("over the
  branches that surfaced it"); `docs/guides/structured-search-hits.md`,
  `docs/reference/python-api.md` and `docs/reference/typescript-api.md`
  ("deduplicated on body"); the ADRs mention RRF only as
  `Σ 1/(rank + k)` over lists. No ADR or `dev/interfaces/` file states the
  three-character rule or within-arm accumulation, so no ADR or interface
  change is needed.
- **Stale for another reason, follow-up:** the public docs give
  `Σ 1/(60 + rank)`; the code uses `RRF_K = 30` and 3:1 text weighting.
- **Not changed:** `tests/support/ir_retrieval.rs` keeps its own ≥3-char
  copy; it is a frozen experiment harness.
