---
title: FathomDB 0.8.28 Slice 40 — requirements and acceptance criteria
status: REVISED AFTER R1 (APPROVE-WITH-FIXES, F1–F6); Defect C and Unit D landed; Units E, F, G added 2026-10-16 (owner ruling) and landed; Unit H added 2026-10-16 (owner ruling), code committed; Unit I added 2026-10-16 (owner ruling), plan reviewed (F1–F9 applied); Unit J added 2026-10-16 (owner ruling), docs for re-check
target_release: 0.8.28
observed_on: 2026-10-16
---

# Slice 40 requirements and acceptance criteria

Slice 40 fixes three defects in FathomDB's text retrieval. Defects A and B
are ranking bugs found by the COMP-01 benchmark. Defect C, added by owner
ruling on 2026-10-16, is a latency bug in `search_projected_text` found by
COMP-02b; its fix must not change which rows are returned or their order.
Unit D, added by owner ruling on 2026-10-16, is the COMP-03 retrieval-lever
work: a retrieval-only sweep (D1), a three-arm answer run (D2) and a
stop-list extension (D3). Units E (SDK parity and the public-surface
contract), F (a known-run baseline) and G (evidence budget, session
completion and recency in the COMP-03 answer harness) were added by owner
ruling on the same day. Unit H, added by owner ruling on the same day,
makes three engine changes: a wider query stop list chosen by a pre-stated
rule (H1), a hybrid vector candidate pool decoupled from `limit` (H2), and
a cohort prefilter for selective attribute filters on projected search
(H3). Unit I, added by owner ruling on the same day, gives each SQLite
connection its own page cache, so one heavy read no longer slows every
later read on the same engine. Unit J, added by owner ruling on the same
day and landed before Unit I, guards the tests that open a raw SQLite
connection before any engine, so the runtime latch no longer makes test
results depend on test order. [design.md](design.md) gives the mechanism,
[test-plan.md](test-plan.md) the tests.

## 1. Evidence

COMP-01 ran BEIR NFCorpus (3,633 documents, 323 test queries, official
graded qrels) through FathomDB 0.8.27 `Engine.search_text_only(limit=10)`
and LanceDB 0.40 native FTS. Result note (read-only, in the `comp-01`
worktree):
`dev/performance-benchmarking/2026-10-15-comp-01-nfcorpus-fts-result.md`.

| Arm | nDCG@10 |
| --- | ---: |
| FathomDB 0.8.27 `search_text_only` | 0.3087 |
| LanceDB 0.40 FTS | 0.3265 |
| Raw SQLite FTS5, FathomDB tokenizer and query translation, no engine post-processing | 0.3228 |
| Raw SQLite FTS5 as above, 1–2 character tokens kept | 0.3260 |

The orchestrator's follow-up diagnosis (scripts `diag2.py`, `fts5exp.py`,
inputs under `~/.cache/fathomdb-comp/`) split the 0.018 gap into two causes.
The result note's own interpretation ("body deduplication is not the
cause") predates that diagnosis and is superseded by it.

- **Defect A, within-arm double counting in RRF.** About 0.014 of the gap.
  NFCorpus has 40 pairs of byte-identical bodies. In run 1, 53 of 323
  queries had a duplicate-body document at rank 1; it was relevant in 4.
  Duplicate documents appeared 164 times in FathomDB top-10s against 65 for
  LanceDB.
- **Defect B, 1–2 character tokens dropped by the query compiler.** About
  0.003 (0.3228 to 0.3260 on raw FTS5; p95 latency 2.24 to 2.30 ms). It deletes "C" from "Vitamin
  C-Enriched Bacon", "D", "D2" and "D3" from vitamin D queries, "II" from
  "Physicians' Study II" and "1" from "IGF-1".

### 1.1 Defect A, read from the code

`fuse_three_arms` (`src/rust/crates/fathomdb-engine/src/fusion.rs`) adds
`weight / (RRF_K + rank)` to one accumulator per distinct `SearchHit.body`,
for every hit of every arm. Two rows with the same body that land in the
same arm both add to that body's score. With `RRF_K = 30` and
`RRF_WEIGHT_TEXT = 3`:

- the true rank-1 hit scores `3/31 = 0.0968`;
- a pair at text ranks 13 and 14 scores `3/43 + 3/44 = 0.1380` and jumps to
  rank 1;
- even a pair at ranks 27 and 28 (`0.1048`) beats rank 1.

The documented contract does not ask for this. `fuse_three_arms`'s doc
comment and `dev/design/slice-10-design.md` give the score as
`Σ_branch weight / (RRF_K + rank_branch(b))`, one term per branch;
`docs/guides/hybrid-search-filtering.md` says "the sum … over the branches
that surfaced it". The explain sidecar's per-arm `body_rank_map`
(`search.rs`) already records one rank per body per arm, the first
occurrence. The code is the outlier.

The same arm can also hold a body twice when an edge body equals a node
body: the edge FTS hits are appended to the text arm after the node hits
(`text_results.extend(edge_candidates)`).

### 1.2 Defect B, read from the code

`content_tokens` (`src/rust/crates/fathomdb-query/src/lib.rs`) skips any
token with `token.len() < 3` (a byte length) before the stopword check.
The same filter also removes the empty fragments that splitting on
non-alphanumeric characters produces, so the fix must keep an
`is_empty()` check. The stopword list already holds the one-character
words that should go ("a", "i").

### 1.3 Defect C, read from the code

Evidence: [comp02b-projected-fts-latency.md](comp02b-projected-fts-latency.md).
On the 10k TC-5 corpus, `search_projected_text` runs at 82 ms p50 against
13.9 ms for `search_text_only` on the same database. The median query
matches 3,806 of 10,000 property rows. Raw sqlite3 timings of the engine's
statement: 52.0 ms as shipped, 5.2 ms with the same predicate and the FTS5
rank stream (`rank MATCH 'bm25()' ORDER BY rank LIMIT 10`). The attribute
filter is not the cost (it makes the query faster by shrinking the sort);
the two dependency-closure `NOT EXISTS` subqueries cost about 6 ms and are
not the cause.

`read_projected_text_in_tx` (`search.rs`) prepares `property_fts_rank_sql`
(`filter.rs`): the FTS5 match joined to `canonical_nodes`, the full
`FrozenView::node_sql` predicate and the attribute-filter predicate,
selecting `n.body`, `ORDER BY bm25(property_search_index) ASC,
p.write_cursor ASC`, with no `LIMIT`. The Rust loop stops after `limit`
rows, but SQLite has already sorted every matched row, body included, in a
temporary b-tree. The body path had the same shape until SCALE-02 (HITL
`seq-268`); `read_search_in_tx` now runs a rank-stream statement
(`AND rank MATCH 'bm25()' ORDER BY rank`), collects it with
`collect_complete_rank_boundary`, and keeps the full sort only as a
fallback (`.ok()` on prepare or execute). The projected path never got
that change.

The public contract is the doc comment on `Engine::search_projected_text`
(`search_api.rs`): results "are ordered by property-FTS bm25 ascending then
write cursor ascending". Scores are `-bm25`.

### 1.4 Unit D evidence (COMP-03)

Source: the COMP-03 result note
(`dev/performance-benchmarking/2026-10-16-comp-03-hermes-answer-result.md`
on the `llm/comp-01` lineage) and the orchestrator's review of it.

- **Answer accuracy:** FathomDB 0.8.27 and LanceDB both 42/60 on the 60
  seeded LongMemEval-S questions (top-5 rounds, gpt-5.4 answerer and judge).
- **FathomDB's 18 misses:**
  - 6 retrieval misses, 5 of them preference questions (a vocabulary gap
    between the question and the round that holds the preference);
  - 10 with the gold round in the prompt but a wrong answer: 4 multi-session
    counts, 3 knowledge-update, 2 preference, 1 temporal;
  - 2 missed abstentions.
- **Retrieval-only rerun on this slice's engine** (Defects A and B fixed):
  top-5 changed on 19 of 60 questions; session Recall@5 0.893 → 0.911;
  session MRR 0.848 → 0.841.
- **Short tokens after Defect B:** 41 of 60 questions carry a 1–2 character
  content token, nearly all "my", "me", "so", "up", "m", "ve". They are
  function words and contraction fragments, not the content tokens ("C",
  "D3", "II", "1") that Defect B set out to keep.
- **Prior finding behind the pre-stated rule:** EARP found hybrid retrieval
  null-or-negative on real traces (memory
  `earp-tiered-run-program-20260809-complete.md`). Hybrid is therefore
  adopted for the answer run only by a rule stated before the sweep runs
  (REQ40-D3), not by inspection of its results.
- **Dataset:** LongMemEval-S cleaned, 500 questions (30 abstention), SHA-256
  `d6f21ea9d60a0d56f34a05b609c79c88a451d2ae03597821ea3d5a9678c3a442`,
  pinned in `hermes-answer.v1.json`. Per type (non-abstention):
  temporal-reasoning 127, multi-session 121, knowledge-update 72,
  single-session-user 64, single-session-assistant 56,
  single-session-preference 30.

### 1.5 Units E, F and G evidence

- **Unit E, SDK parity.** Slice 40 changed behaviour under three public
  verbs without changing a signature:
  - `search_text_only`: per-arm dedupe (A), short tokens (B), the stop list
    (D3);
  - `search_projected_text`: the rank stream (C);
  - `search`: dense retrieval through the `vector=True` projection switch
    (Unit D).

  The existing cross-SDK suite
  (`dev/performance-benchmarking/2026-10-13-cross-sdk-fts-comparison-result.md`
  on the comp lineage) checks only ordered IDs from `search_text_only` on 100
  synthetic single-term documents, against fixture oracles. Facts read from
  the code on 2026-10-16:
  - The TypeScript SDK has `useDefaultEmbedder`, `ProjectionSpec.vector`,
    `searchProjectedText(query, name, filter?, {limit, …view})` and `search`.
    The Rust SDK has `use_default_embedder`, `configure_projections`,
    `search`, `search_text_only` and `search_projected_text`. No product gap
    blocks the three parity cases.
  - No SDK exposes the engine's build commit. Every build reports version
    `0.8.27`.
  - The public docs give the RRF score as `Σ 1/(60 + rank)` in
    `docs/guides/structured-search-hits.md`,
    `docs/guides/hybrid-search-filtering.md`,
    `docs/reference/python-api.md` and `docs/reference/typescript-api.md`.
    The code uses `RRF_K = 30` with weight 3 for text and 1 for vector and
    graph.
  - No checker pins the interface docs' verb descriptions.
    `scripts/check-sdk-surface-parity.py` checks operation names and
    signatures only.
- **Unit F, baseline.** The Unit D numbers came from runs at three engine
  commits:
  - NFCorpus D3 at `5b09afd44`;
  - `comp01-hybrid` and the pre-D3 sweep at `a3830681d`;
  - the D3 sweep and answer v2 at `5b09afd44`.

  The pre-D3 sweep and both NFCorpus runs have no receipts. The 10k TC-5
  latency figures came from an unversioned scratchpad script. A later run
  cannot be checked against them.
- **Unit G, the COMP-03 misses** (orchestrator's miss analysis of answer
  v2). The 11 FathomDB FTS misses break down as:
  - 4 where the gold was cut by the 20,000-character cap or the session
    was only partly shown;
  - 4 with the gold present but a wrong answer, including one stale
    knowledge-update value;
  - 2 retrieval misses;
  - 1 abstention.

  The cap was reached on 46 of 60 questions, and the median question showed
  9 of its 10 rounds. Only 6 of 10 multi-session questions had all their
  gold sessions in the prompt.

### 1.6 Unit H evidence

Sources:

- The orchestrator's read-only analysis of 2026-10-16, run on the
  `5b09afd44` wheel. Its report is `improve/report.md` in the session
  scratchpad (§ 2, § 3b, § 4, § 5), next to `sweep_query.py`,
  `sweep100.py`, `refuse.py` and `nfc_check.py`.
- A raw-SQL plan probe run while writing this document (`h3probe/probe2.py`
  in the same scratchpad).
- The code, read on 2026-10-16.

**H1, query stop list.**

- Questions keep high-document-frequency function words that survive the
  current list: "how MANY siblings DO I HAVE", "CAN you RECOMMEND SOME".
- Dropping a list of about 50 such words from the question before
  `search_text_only`, on the same stores, gives session Recall-all@k:

  | Type | all@5 | all@10 | all@20 |
  | --- | --- | --- | --- |
  | preference | .733 → .900 | .900 → .933 | .967 → .967 |
  | multi-session | .545 → .562 | .744 → .785 | .868 → .893 |
  | temporal | .724 → .717 | .819 → .811 | .890 → .882 |
  | knowledge-update | .944 → .958 | .986 → .986 | .986 → .986 |

  Paired Recall-all@10: 7 wins, 2 losses.
- NFCorpus nDCG@10 moves 0.3265 → 0.3253 (−0.0013). 23 of 323 queries
  change: 7 wins, 5 losses.
- The list holds `d`, which ruling D3 keeps as content, and `ve`, which D3
  already made a stopword.
- `compile_text_query` is the only query compiler. Its callers are:
  - `search_text_only` (`search_api.rs`);
  - the text branch of `search` (`search_api.rs`, and `search.rs` for
    frozen reads);
  - `search_projected_text` (`search.rs`);
  - the graph arm's query seed (`graph_expand/execution.rs`).

  The vector branch embeds the raw query. Document indexing never reads
  the list.

**H2, hybrid vector pool.**

- **The pool is `limit`.** In the vector branch of `search`
  (`search.rs`), phase 1 and phase 2 already materialise
  `max(candidate_limit, final_limit, TOP_K_BIT_CANDIDATES = 192)`
  candidates in exact-L2 order. Hydration then stops at `final_limit`
  surviving rows (`if results.len() >= final_limit`).
- **The text branch has no cap.** Under `search` it returns every FTS
  match: 178 hits at limit 5 in the trace.
- **Measured effect:**
  - The trace shows `vector_hits` = limit at limits 5, 10 and 100.
  - `ids@10 == ids@100[:10]` holds for 16 of 500 questions.
  - Session Recall-all@10 on multi-session: limit 10 gives .810;
    limit 100 cut to 10 gives .777.
  - At k = 20, limit 100 reaches .934, against .868 for FTS.
- **Fusion weights.** Re-fusing the per-hit ranks with other weights
  (1:1, `K` = 60, vector-heavy) found no setting that dominates text 3,
  vector 1, `K` = 30.
- **Why a pool can be measured on today's engine.** Below 192 the vector
  list for a pool `P` is the first `P` surviving rows of one fixed
  ranking. The text branch does not depend on `limit`, and fusion runs
  before truncation. A pool of `P` at limit `k ≤ P` therefore ranks
  exactly as today's `search(limit=P)` cut to `k`, with rerank depth 0, no
  graph arm and the reweights off.

**H3, selective attribute filters on projected search.**

- At the final head, 10k TC-5, FathomDB p50 on the projected cells is:
  - c01 (92 of 10,000 documents): 29.8 ms, against SurrealDB's 4.1 ms;
  - c10 (968): 9.1 ms;
  - c33 (3,327): 8.2 ms;
  - unfiltered: 2.9 ms.
- At 1% the Defect C rank stream walks about a thousand ranked rows for
  ten survivors. Each walked row pays the node join and the attribute
  `EXISTS` probe.
- `property_search_index` stores `write_cursor` as an `UNINDEXED` column
  under FTS5's own rowids, so the FTS5 index cannot be entered from a
  cohort cursor. The cohort is cheap to read from
  `canonical_attributes_name_value_idx`.
- **Probe.** The probe opened the final-head 10k store read-only. It ran
  Python's SQLite 3.45.1 over the 100 TC-5 queries, compiled with the D3
  list, using a simplified node predicate. Warm p50 in ms:

  | Plan | c01 | c10 | c33 |
  | --- | ---: | ---: | ---: |
  | Defect C rank stream (today) | 14.0 | 3.9 | 3.3 |
  | Rank stream plus `p.write_cursor IN (cohort)` | 7.7 | 4.7 | 7.0 |
  | `p.write_cursor IN (cohort)`, sorted, no rank stream | 8.6 | 11.0 | 15.6 |
  | Attribute `EXISTS` keyed on `p.write_cursor` (the row-key potential) | 12.5 | 4.0 | 3.2 |

  - Every plan returned the same IDs in the same order for every query
    and cohort.
  - The prefilter wins at 92 cohort rows and loses at 968, because it
    materialises the whole cohort.
  - The probe's node predicate is lighter than the engine's, so its
    absolute times are lower than the engine's 29.8 ms.

### 1.7 Unit I evidence

Sources:

- The orchestrator's diagnosis of 2026-10-16 on the H3 engine. Its probe
  scripts are in the session scratchpad under `diag-lat/` (`cells.py`,
  `dose.py`, `reopen.py`, `cheap.py`, `rawsql.py`, and the
  `cache_status_per_worker_for_test` probe in `zz_diag_order.rs`).
- The bundled SQLite 3.53.2 source
  (`third_party/libsqlite3-sys-0.38.1/sqlite3/sqlite3.c`) and its build
  script, read on 2026-10-16.
- Two probes run while writing this document, in the same scratchpad: a
  raw-SQLite probe (`pcache-probe/`, eight reader connections) and an
  engine probe (`zz_unit_i_probe.rs`, run on this branch and then
  removed). Each ran once with the vendored build flags and once with
  `LIBSQLITE3_FLAGS=-USQLITE_ENABLE_MEMORY_MANAGEMENT`.

**What the orchestrator measured** (10k TC-5 store, 100 queries):

- After one selective projected search (c01, the Defect C route, about
  1,000 ranked rows with large bodies), `search_text_only` p50 on the
  same open engine went from 3.6 ms to about 9 ms. It stayed there for
  every later pass until the engine was reopened. After a reopen it was
  3.6 ms again.
- One c01 query is enough. The slowdown is per page read, so it is
  about 2.5× across query shapes.
- The H3 cohort route reduces the effect but does not remove it: body
  p50 after one c01 query on the cohort route is 6.0 ms.
- `cache_status_per_worker_for_test` after the c01 query: the reader
  that ran it holds about 22.5 MB of page cache. Most of the other seven
  hold about one page, and they do not regrow.
- Results are unchanged. The page cache changes only whether a page
  comes from SQLite's memory or from the operating system.

**Mechanism, read from the SQLite source.**

- The build script passes `-DSQLITE_ENABLE_MEMORY_MANAGEMENT`
  (`build.rs`, line 135). `pcache1Init` then sets
  `pcache1.separateCache = 0` unconditionally. Every page cache in the
  process joins one global `PGroup` ("mode 2"). The source comment:
  *"Always use a unified cache (mode-2) if ENABLE_MEMORY_MANAGEMENT"*.
  Without the flag, a thread-safe build with the default configuration
  gives each connection its own `PGroup` ("mode 1").
- In mode 2 there is one LRU list of unpinned pages for all
  connections. `PGroup.nMaxPage` is the sum of every connection's
  `cache_size` page limit.
- `pcache1FetchStage2`, step 4: when a cache needs a new page and holds
  `nMax` pages or more, it recycles the tail of the **group** LRU. That
  is the least recently used unpinned page of any connection. The
  page moves to the fetching cache, so its page count grows past its
  own `nMax`. The other cache shrinks.
- `pcache1Unpin`: when the group holds more pages than `nMaxPage`, an
  unpinned page is freed at once instead of joining the LRU.
- A starved cache is below its own `nMax`, so it never recycles. It
  allocates a new page, the group goes over `nMaxPage`, and the page is
  freed again on unpin. It stays at about one page until the large
  cache gives pages back. That happens only on that connection's close,
  a `cache_size` change or a shrink.
- FathomDB's production open path sets no `cache_size`, so each
  connection has SQLite's default (`-2000`, 2,000 KiB). An engine keeps
  twelve connections open by default: one writer, eight readers
  (`READER_POOL_SIZE`, `reader_pool.rs`), the projection dispatcher and
  two projection workers (`scheduler_runtime_threads` defaults to 2,
  `runtime_configuration.rs`; the roles are listed in
  `native_state_expected_roles`, `wal_attribution.rs`). Runtime probes
  open and close transient connections. Twelve 2,000 KiB budgets are
  24,000 KiB, which bounds the 22.5 MB one reader held.

**Probes.**

- Raw SQLite, 5,000 rows of 4 KB. Each of eight readers first scans
  its own 300 rows; then reader 0 scans the whole table:

  | Build | Reader 0 cache after the scan | Readers 1–7 cache after it | Later pass, misses per reader 1–7 |
  | --- | ---: | --- | --- |
  | vendored flags (mode 2) | 13,167 KiB | 5 KiB each; 3 of 7 still about 5–136 KiB after three more passes | 0, 0, 0, 0, 274, 304, 305 |
  | flag undefined (mode 1) | 2,053 KiB | 1,297–1,301 KiB, unchanged | 0 for every reader |

- The same probe with `PRAGMA cache_size` set on every reader. At
  `-2000` (the default, set explicitly) the result is the same as
  mode 2 above. At `-8000` reader 0 takes 19,338 KiB and leaves the
  others at 5 KiB. They regrow only because the group's sum of limits
  (eight times 8 MB) is larger than this 20 MB table. On the 150 MB
  TC-5 store it would not be, so a larger `cache_size` only delays
  the effect.
- `sqlite3_db_release_memory` on reader 0 after the heavy scan frees
  every unpinned page in the group, its own and every other reader's.
- Engine probe: a store of eight 40-document warm sets and one
  100-document heavy set with 40 KB bodies. Each reader runs its own
  warm query three times; then one reader runs the heavy query, then
  every reader runs its warm query again:

  | Build | Heavy reader's cache after the heavy query | Other readers' misses on the next warm pass |
  | --- | ---: | --- |
  | vendored flags (mode 2) | 4,378 KiB | 108, 108, 108, 16, 0, 0, 0 |
  | flag undefined (mode 1) | 2,053 KiB | 0 for every reader |

  The third warm pass had 0 misses on every reader in both builds.

**What else the flag does in this build** (as read):

- `sqlite3_release_memory()` is a no-op without it. FathomDB never calls
  it. rusqlite's `Connection::release_memory` calls the per-connection
  `sqlite3_db_release_memory`, which SQLite documents as working without
  the flag.
- With it, a failed allocation retries after a malloc alarm. That runs
  only when a soft heap limit is set and memory statistics are on.
  Without it the soft heap limit stays advisory (memory pressure still
  makes caches recycle, per connection) but no longer frees memory
  actively; the hard heap limit is enforced either way.
  `tests/runtime_configuration.rs` sets a 1,024-byte hard limit and
  expects an oversized `sqlite3_malloc64` to fail; that test must still
  pass.
  FathomDB sets no heap limit, and its default `Performance` runtime
  mode turns memory statistics off (`SQLITE_CONFIG_MEMSTATUS = 0`,
  `runtime_configuration.rs`).
- In mode 2 every page fetch and unpin takes the global
  `SQLITE_MUTEX_STATIC_LRU`. In mode 1 the group mutex is null. The
  0.7.0 performance notes attributed a measured share of concurrent
  read cycles to `pcache1` mutex acquires
  (`dev/notes/performance-whitepaper-notes.md` § 12.4,
  `ADR-0.7.0-ac020-architectural-lever.md` option 1). Unit I does not
  claim a gain from this; the gate measures it.

**Reach.** The workspace patch `libsqlite3-sys =
{ path = "third_party/libsqlite3-sys-0.38.1" }` covers every artifact
built from this repository: the Python wheel, the npm addon and the CLI.
Root patches do not reach downstream users of the published Rust crates
(`Cargo.toml`). Those builds resolve the registry `libsqlite3-sys`
0.38.1, which keeps the flag. That build script honours
`LIBSQLITE3_FLAGS`, and a `-U` entry is passed through after the
built-in `-D` list (`build.rs`, lines 120–131).

**Upstream** (external research, 2026-10-16,
`sqlite-mm-research.md` in the session scratchpad). The flag entered
rusqlite's bundled build in rusqlite PR #197 (merged 2016-12-31), copied from System.Data.SQLite's
compile-option list with no rusqlite-specific reason. Upstream removed it
on master in PR #1900 (commit
`f31f7324a82894be13e5de1665cb9af45dc5f869`, merged 2026-09-04: it "forces
the unified page cache … nothing in rusqlite itself needs it"). No crates.io release carries the removal yet: the
latest, `libsqlite3-sys` 0.38.2 and `rusqlite` 0.40.2, predate it. Field
reports of `SQLITE_MUTEX_STATIC_LRU` contention under the flag:
BrokkAi/bifrost #21 and cognica-io/uqa-engine #348.

**Effect on published numbers.** Both latency harnesses timed several
cells on one open engine:

- `comp02b_latency.py` timed six cells in the order unfiltered, c01,
  c10, c33, body, single-term. The manifest's `body-text-only` p50
  (8.76 ms) and the c10 and c33 values were therefore measured after
  c01.
- The COMP-02b campaign ran c01, then c10, then c33 on one open.

The harness fix (`comp03-slice40`) opens the store afresh for each cell
and each cohort (test-plan § 3h.4). Its numbers are a new measurement
method and are not comparable with the manifest's values.

### 1.8 Unit J evidence

Source: the investigation of 2026-10-16
(`pr-g8/proposal.md` in the session scratchpad, with `prototype-A.diff`
and `prototype-A-tc57.diff`), run on a private export of `65a6e29b3`, and
a sweep of this branch's test files made while writing this section.

- `configure_runtime_locked` (`runtime_configuration.rs`) calls
  `sqlite3_config(SQLITE_CONFIG_MEMSTATUS)` and then
  `sqlite3_initialize`. If SQLite is already initialized,
  `sqlite3_config` returns `SQLITE_MISUSE`. The engine maps that to
  `RuntimeConfigurationError::TooLate` and latches it, so every later
  `Engine::open` in the process fails. That is the behaviour
  `ADR-0.8.25-sqlite-runtime-configuration.md` requires (fail closed, no
  silent fallback, no `sqlite3_shutdown`).
- The first raw `rusqlite::Connection::open` in a process initializes
  SQLite. A test binary whose tests run in parallel latches `TooLate`
  when a test opens a raw connection before any other test has opened
  an `Engine`. The engine tests in that binary then fail with
  `open: RuntimeConfiguration(TooLate)`.
- Measured on the export (binary looped directly, 24-core host):

  | Build | Target | Parallel runs failed | `--test-threads=1` |
  | --- | --- | ---: | ---: |
  | default features | `pr_g8_dangling_edges` | 19 of 30 | 0 of 30 |
  | `test-hooks` | `pr_g8_dangling_edges` | 29 of 30 | not run |
  | `test-hooks` | `tc57_governed_write_race` | 30 of 30 | not run |

  With the guard of option A in place (the next bullet), it was 0 of
  50 in each case.
- The repository already has a convention for this:
  `tests/pr_g0_identity.rs` (`configure_test_runtime`, with a comment
  naming the latch) and seven other integration files call
  `configure_runtime(RuntimeSqliteMode::Performance)` before their
  first raw open. Slice 30 applied the same remedy to fourteen in-crate
  tests at `6f5aec196`.
- `TooLate` displays as *"SQLite runtime configuration is too late;
  restart required"*. A process that latched it does need a restart,
  but the remedy is ordering. A Rust consumer that opens its own
  `rusqlite` connection before the first `Engine::open` shares the
  same `libsqlite3-sys` and hits this. No test asserts the message
  text (grep over `src/`, the bindings and their tests, 2026-10-16).
- `slice135_robustness_first`'s fd-count failure is a different defect
  (its raw open follows an `Engine::open`). It is not part of Unit J
  and stays recorded as pre-existing.

## 2. Requirements

**REQ40-01 — once per arm.** In `fuse_three_arms`, each arm (vector, text,
graph) contributes at most one RRF term to a given body. The term uses the
positional rank (1-based index in that arm's list) of the body's first
occurrence in that arm. Later occurrences in the same arm contribute
nothing and do not change other hits' ranks.
*Rationale:* removes the double counting of § 1.1 and keeps fusion's
per-arm rank equal to the explain sidecar's.

**REQ40-02 — cross-arm agreement kept.** A body surfaced by more than one
arm still accumulates one term from each of those arms.
*Rationale:* agreement boosting is the purpose of hybrid fusion (ADR
`ADR-0.8.0-agent-memory-retrieval-and-identity.md`, Q3).

**REQ40-03 — everything else unchanged.** For inputs in which no arm holds
a body twice, the output of `fuse_three_arms` (order, score bits,
representative hit) is unchanged. The representative of a body is still its
first occurrence (vector arm first, then text, then graph); the sort is
still score descending, then vector-first, then first-insertion order;
`fuse_rrf(v, t) == fuse_three_arms(v, t, [])` still holds.

**REQ40-04 — short tokens kept.** `compile_text_query` keeps alphanumeric
tokens of every length, including 1 and 2 characters, as content tokens.
Empty split fragments are still discarded.

**REQ40-05 — stop list and query shape kept.** The `STOPWORDS` list is
unchanged and still applied. Lowercasing, splitting on non-alphanumeric
characters, first-seen de-duplication and the `"tok" OR "tok"` shape are
unchanged. Removing the stop list cost 0.004 nDCG@10 and doubled tail
latency in the diagnosis, so it stays.

**REQ40-06 — injection safety kept (AC-038).** Every emitted term is one
FTS5 string literal; content-path terms are non-empty and alphanumeric.

**REQ40-07 — fallback kept.** A query with no content token (only
stopwords, only symbols) still takes the raw whitespace-token OR fallback,
byte-for-byte as today. A query whose only tokens are short non-stopwords
now has content tokens and takes the content path.

**REQ40-08 — documentation matches.** Every doc that states the old
behavior is corrected in the change that fixes it (list in
[design.md](design.md) § 5).

**Defect C requirements (R-C1 to R-C6).** The fix changes how projected
search reaches its rows, never which rows it returns or their order.

**REQ40-C1 — streamed rank path.** `read_projected_text_in_tx` first runs
the projected statement with `AND rank MATCH 'bm25()' ORDER BY rank` and
collects it with the boundary-tie completion
(`collect_complete_rank_boundary(limit)`), so SQLite never sorts the full
match set.

**REQ40-C2 — identical ordered results.** For every input, the ordered
`(write_cursor, score, id, kind, body, source_id)` list equals what the
full-sort statement returns today, including rows tied on bm25 at the
limit boundary (ties order by `write_cursor` ascending).

**REQ40-C3 — limit honoured exactly.** The collector is bounded by the
validated `limit` argument (1..=100), not a fixed window. When at least
`limit` rows match, exactly `limit` come back.

**REQ40-C4 — filter semantics unchanged.** The streamed statement keeps the
full `FrozenView::node_sql` predicate and the attribute-filter predicate
from `append_node_eligibility_sql`, byte-identical to the full-sort
statement's.

**REQ40-C5 — fallback.** If the streamed statement fails to prepare or
execute, the existing full-sort statement runs, unchanged, in the same read
transaction, exactly as the body path does.

**REQ40-C6 — nothing else changes.** `search_text_only`, hybrid `search`,
`property_fts_rank_sql` and its `slice35_ranked_eligibility_sql_for_test`
seam, the public API and the schema are unchanged.

**Unit D requirements.** D1 and D2 change the COMP harness only; D3
changes the product.

**REQ40-D1 — retrieval sweep (D1).** A new harness module runs every
LongMemEval-S question through the arms `fathomdb-fts`
(`search_text_only`, v1 store shape), `fathomdb-hybrid` (default embedder,
`Engine.search`, no rerank, no graph arm, over the same plain bodies plus a
searchable `vector=True` projection declaration) and `lancedb-fts`
(reference). `fathomdb-fts` vs `fathomdb-hybrid` is a pure vector-arm
ablation over identical text (R1 ruling Q-D2). Per question and arm it records the top-10 IDs, query latency,
and build time, with embedding time recorded apart. It reports, overall and
per question type: session Recall@5 and @10 (any), session Recall-all@5
and @10, session MRR@10, turn Recall@5 and @10, and a paired bootstrap 95%
CI for every `fathomdb-hybrid − fathomdb-fts` delta (seed in the config).

**REQ40-D2 — sweep discipline.** A fresh store per question per arm; the
gold-leak discipline of `comp03_answer` (its `convert_question` and
`gold_for`, reused, not copied); resumable per question (a finished
question does no work on rerun); the dataset digest checked against the
config; the embedding device read from `FATHOMDB_EMBED_DEVICE`, recorded,
and a CUDA request that resolved to CPU refused; every hybrid store's
vector coverage checked on the closed database (distinct embedded doc
cursors == documents written) and a mismatch refused, not reported.

**REQ40-D3 — pre-stated decision rule.** The rule lives in the sweep config
and is evaluated by a pure function whose verdict is written into the sweep
summary. Verbatim: *adopt hybrid for the answer run iff (a) preference
session Recall@5 delta CI lower bound > 0, (b) no other type's delta CI
upper bound < 0, (c) overall delta ≥ 0.* The delta is
`fathomdb-hybrid − fathomdb-fts` on session Recall@5 (any) for all three
clauses.

**REQ40-D4 — three-arm answer run (D2).** `comp03_answer` gains a v2 config
(`hermes-answer.v2.json`): the same 60 question IDs, models, prompts, caps
and judge as v1, `top_k` 10 for every arm, arms `fathomdb-fts`,
`fathomdb-hybrid`, `lancedb-fts`. The hybrid arm runs only when the D1
sweep verdict is `adopt`. Arm order rotates by question so that, for two
arms, it is exactly v1's alternation. The v1 config, its receipt and its
code path are unchanged. A `fathomdb-hybrid` arm is also registered in
`comp_fts` (campaign `comp01-hybrid`) so the NFCorpus informational row can
run; the existing `fathomdb` arm is unchanged in every byte.

**REQ40-D5 — stop-list extension (D3).** `STOPWORDS` in `fathomdb-query`
gains exactly twelve words, in two groups:

- possessive and contraction fragments left by splitting on the apostrophe:
  `s` ('s), `t` (n't), `ll` ('ll), `re` ('re), `ve` ('ve), `m` ('m);
- short function words: `my`, `me`, `us`, `am`, `so`, `up`.

`d`, `c`, `d3`, `ii`, `1` and every other short token stay content.
Nothing else in the compiler changes.

**REQ40-D6 — D3 measurement gate (orchestrator, after W4).** Verbatim:
*NFCorpus nDCG@10 paired delta vs 0.3260 with CI not below zero, COMP-03
per-type recall unchanged within CI, 10k TC-5 p95 not higher.* If the gate
fails, D3 is reverted; it is therefore one commit of its own.

**Unit E requirements.**

**REQ40-E1 — parity cases.** The cross-SDK suite gains three cases, each
run through the Python, TypeScript and Rust SDKs:

- **(1) `search_text_only`.** The corpus has duplicate bodies. Queries carry
  1–2 character content tokens, possessive and contraction fragments, and
  every D3 word.
- **(2) `search_projected_text`.** Run with and without an attribute
  filter, on a corpus whose bm25 tie group straddles the limit.
- **(3) `search` with the `dense` projection declared.** Default embedder,
  CPU, fixed corpus.

**REQ40-E2 — exact parity.** For every case and repetition, each SDK's
ordered hits equal the oracle's in `(id, score, branch)`, scores compared
bit-for-bit. The oracle is the Python SDK run in the coordinator on the
same database file the SDK just queried. The engine computes every score,
vector scores included, so any difference is a binding defect, not
numerical noise.

**REQ40-E3 — the engine under test is this slice's.** All three SDK
artifacts (Python wheel, TypeScript N-API addon, Rust driver) and the
bootstrap CLI are built from one recorded engine commit and pinned by
SHA-256. No SDK exposes a build commit, so the arm manifest records the
source commit, its clean state at build time and the artifact digests.

**REQ40-E4 — public-surface contract.** `dev/interfaces/python.md`,
`typescript.md`, `rust.md` and `rust-sdk.md` state, for the three verbs:

- per-arm dedupe;
- short tokens kept and the extended stop list;
- the projected rank stream, with the order contract unchanged;
- the dense-projection switch, and that the full node body is embedded.

The public docs' RRF formula is corrected to `RRF_K = 30` with 3:1
text-to-vector weighting. `./scripts/agent-lint-docs.sh` passes.

**Unit F requirements.**

**REQ40-F1 — baseline manifest.** `experiments/configs/baselines/slice-40-baseline.v1.json`
lists, per benchmark:

- inputs identifier and SHA-256;
- engine commit;
- wheel build recipe (features, `CUDA_PATH`/`PATH`/`LIBRARY_PATH`,
  `--compatibility linux`);
- exact command line;
- receipt ID or "to be recorded";
- expected metrics with tolerances: nDCG@10 ±0.002; MRR ±0.005;
  per-type session recall ±1 question; latency p50 ±25% and p95 ±35% on a
  quiet host; answer accuracy ±3 questions, recorded as noise, not a gate.

The benchmarks are COMP-01 NFCorpus (fts vs LanceDB), `comp01-hybrid`, the
COMP-03 retrieval sweep (three arms), COMP-03 answer v2, and 10k TC-5
projected and body latency.

**REQ40-F2 — latency harness.** `experiments/comp02b_latency.py` turns the
orchestrator's diagnostic script into a receipt-producing command with the
same cells (projected unfiltered, cohorts c01, c10 and c33, body
`search_text_only`, projected single term) and the same percentiles. It has
no SurrealDB arm.

**REQ40-F3 — drift check.** `experiments/baseline_check.py` takes the
manifest, a benchmark ID and an attempt's summary or report. It prints PASS,
DRIFT or MISSING per metric, with measured value, expected value and
tolerance, and exits non-zero on any DRIFT or MISSING.

**REQ40-F4 — engine identity in receipts.** Every COMP receipt from now on
records the engine commit (declared at run time, validated as a 40-hex
SHA), the installed native module's SHA-256 and the version string.

**REQ40-F5 — measured at the final head.** The baseline is re-measured by
the orchestrator, with receipts, at the final slice head after Units E to
G land. The Unit D runs are not the baseline.

**Unit G requirements** (revised after the Unit G review, findings F1–F10,
2026-10-16). All of G is harness-side this round.

**REQ40-G1 — offline evidence evaluator.** `comp03_retrieval` evaluates
evidence policies on the stored D3-sweep records
(`~/.cache/fathomdb-comp/comp03-sweep-d3`), with no retrieval and no model
calls.

- **Policies.** A policy is `per_round_chars` ∈ {4000, 2000, 1000},
  `total_chars` ∈ {20000, 30000} and `session_expansion` ∈ {off, on}:
  twelve policies.
- **Question set.** The 470 scored questions; abstentions are excluded from
  every gold metric.
  - The policy is selected on the scored questions that are not among the 60
    answer questions: 470 − 56 = 414, because the 60 answer IDs include four
    abstentions (`f685340e_abs`, `e5ba910e_abs`, `bc8a6e93_abs`,
    `f4f1d8a4_abs`), so 56 of them are scored.
  - The answer set (60 IDs, 56 scored) is reported separately, so the paid
    runs are not on the selection set.
  - The implementation derives every count from the data and hard-codes
    none.
- **Bodies.** Round bodies are re-derived from the dataset with
  `convert_question`. The verdict binds the dataset's and the sweep
  summary's SHA-256.
- **LanceDB records.** LanceDB's record for `gpt4_7fce9456` predates the
  blank-round filter (253 vs 251 documents). The evaluator renders from
  converted docs, where the two blank rounds contribute only whitespace, so
  the effect is nil.
- **Prompt order.** Rounds enter the prompt in rank order, per-round cap
  first, then the total cap, as `render_evidence` does.
  - Session expansion appends, after the top-k rounds, the remaining rounds
    of each hit session. Sessions go in the order of their best hit's rank,
    rounds within a session in position order, until the total cap.
  - Blank rounds are enumerated at 0 characters and not rendered.
- **Metrics** per policy, per type and overall, per arm and pooled over the
  three arms:
  - `gold_session_any_in_prompt`, `gold_session_all_in_prompt`;
  - `has_answer_in_prompt` (at least one character of a has-answer round
    shown);
  - `has_answer_untruncated_in_prompt` (a has-answer round shown in full);
  - `gold_chars_shown_fraction` (shown characters over full length, summed
    over the gold rounds);
  - rounds in prompt, and characters.

**REQ40-G2 — pre-stated policy rule** (in the config): *maximise
`has_answer_in_prompt` subject to, per type, `has_answer_untruncated_in_prompt`
not lower than the current policy's (4000 / 20000 / off) by more than the
tolerance; ties broken by fewer chars.*

- **Tolerances.**
  - Pooled over the three arms: 3 counts, one question in every arm.
  - `fathomdb-fts` alone: 1 count. The per-type constraint must hold both
    pooled and for `fathomdb-fts` alone.
  - If the pooled winner breaks the `fathomdb-fts` constraint, the best
    policy that satisfies both is chosen.
- **Rule inputs.** `gold_session_any_in_prompt` and
  `gold_session_all_in_prompt` are reported, not rule inputs.
  - `any` is monotone in every grid direction, so a constraint on it is
    vacuous.
  - `all` is bounded by Recall-all@10 and cannot move with expansion.
- **Verdict.** It carries the pooled and per-arm tables for the selection
  set and for the answer set.
- **Expected answer-set delta.** Next to the selection result, the verdict
  records the offline delta of the chosen policy over the current one on
  the answer set, pooled and per arm, so the paid v3 result is read against
  that expectation. The reviewer's replay picked 4000/30000/on. On the answer
  set, expansion adds no has-answer round, so v3 vs v2 is expected near-null
  for the expansion and small for the 30,000 cap.
- **Renderer caps.** An evidence policy passed to `render_evidence` replaces
  `max_item_chars` and `max_total_chars`, so passing both is an error.

**REQ40-G3 — recency rendering, as two single factors.** The answer harness
can:

- (a) add one sentence to the answerer prompt: *When the history states
  different values for the same fact at different dates, use the most
  recent one.*
- (b) render evidence in chronological order by session date, newest last,
  with dates kept. Rounds on the same date order by session position, then
  round position.

Both are off by default. With both off, the rendering and prompt are
byte-identical to v2's. Chronological order reorders the rounds the
rank-order cap selected; it does not change which rounds are selected.

**REQ40-G4 — one implementation of the evidence rounds.** `comp03_answer`
renders evidence from a policy by calling `comp03_retrieval`'s
evidence-rounds function, so the evaluator and the paid runs share one
definition. `evidence_sha256` covers the expanded text, and every answer row
records `rounds_in_prompt` and `chars`.

**REQ40-G5 — answer configs and rules.**

- **Configs.** Each is v2 (three arms, k = 10, the same 60 IDs and models)
  plus:
  - `hermes-answer.v3.json`: the G1/G2 policy, bound by the verdict file's
    SHA-256;
  - `hermes-answer.v4a.json`: v3 with the sentence;
  - `hermes-answer.v4b.json`: v4a with chronological order;
  - `hermes-answer.v4c.json`: v3 with chronological order alone, the
    contingency run of REQ40-G6.
- **Attribution.**
  - v4a − v3 attributes the sentence.
  - v4b − v4a attributes chronological order given the sentence.
  - v3 − v2 is reported per arm.
- **Adoption rule** (each factor, paired by question): adopt iff
  knowledge-update plus temporal correct does not drop and overall does not
  drop by more than 1 question. The report gives per-type counts and the
  overall bootstrap CI for every arm.
- **Arm of record.** Each factor's adoption is decided on `fathomdb-fts`, the
  product arm of record; the other arms are reported in the verdict, not
  used.
- **Statistical caveat.** At n = 60 paired, the minimum detectable effect is
  about 6 net discordant questions. Every v3, v4a and v4b verdict is
  labelled provisional. Adoption into the best mix is gated on the per-type
  direction plus the 470-question offline metrics, not on answer-run
  significance.

**REQ40-G6 — the best mix.** A pure function builds
`hermes-answer.mix.json` from the verdicts, on the arm of record
(`fathomdb-fts` by default, passed explicitly):

- the evidence policy from G1/G2;
- the sentence on iff v4a adopts;
- with the sentence on, chronological order on iff v4b adopts, since v4b
  measured chronological order only together with the sentence;
- **v4c contingency.** If v4a rejects the sentence but v4b − v4a adopts
  chronological order, the orchestrator runs v4c (chronological order only,
  vs v3) before the mix is finalised. Chronological order alone is enabled
  only by a v4c adopt; without a v4c verdict in that case, building the mix
  is refused.

That config is the owner's follow-on best-mix run.

**REQ40-G7 — product-side equivalents, named only.** The design states the
engine-side equivalents (turn-level retrieval units with session expansion;
a recency-aware ranking pass), so the owner can decide on a product unit.
None is built in this slice.

**Unit H requirements** (added by owner ruling 2026-10-16). Three changes,
H1 to H3. Each one's list, pool size or threshold comes from a rule stated
here before anything is measured. Each engine change lands as its own
commit, and each gate is measured at its own commit against the commit
before it (review F3), so a failed gate reverts only that change.

Owner ruling 2026-10-16 (H1 superseded): the H1 words are **not** added to the engine's default `STOPWORDS`, which stays at 63 words. Slice 41 adds a configurable extra stop-word list, empty by default, and the benchmarks pass `had`, `long` and `suggest` through it. H1 therefore makes no engine change on Slice 40. Its selection rule
(REQ40-H1) ran and its verdict (`stoplist-verdict.v1.json`, harness
`3f0789271`: adopt `had`, `long`, `suggest`) stays as recorded evidence for
Slice 41. REQ40-H2 and REQ40-H3 and their ACs are superseded, kept for
numbering. The engine commits are H2 then H3:

| Commit | Gate | Measured there | Reference |
| --- | --- | --- | --- |
| ~~H1~~ (superseded) | ~~REQ40-H3~~ | none; the H1-commit hybrid sweep at pool = limit is not run | — |
| H2 | REQ40-H6 | COMP-03 sweep; `comp01-hybrid`; COMP-02b 10k (reference for H3); the Unit E cross-SDK suite | the final-head baseline |
| H3 | REQ40-H9 | tc5-10k-latency; COMP-02b 10k | the H2-commit measurements |

The H3 commit is the final commit, so REQ40-H10's re-runs reuse its
measurements where they exist.

**REQ40-H1 — stop-list selection rule (offline, before the engine
change).** The rule lives in `experiments/configs/comp-03/stoplist-selection.v1.json`
(harness branch). A pure function evaluates it and writes
`stoplist-verdict.v1.json`.

- **Candidates (47):** `many`, `much`, `have`, `had`, `has`, `can`,
  `could`, `would`, `should`, `some`, `any`, `recommend`,
  `recommendations`, `suggest`, `suggestions`, `give`, `tell`, `know`,
  `think`, `want`, `need`, `get`, `got`, `total`, `number`, `different`,
  `also`, `been`, `being`, `just`, `still`, `currently`, `recently`,
  `there`, `all`, `based`, `help`, `please`, `going`, `make`, `use`,
  `used`, `using`, `far`, `long`, `often`, `again`. This is the analysis
  list without `d` (content by ruling D3) and `ve` (already a stopword).
- **Data:**
  - The COMP-03 sweep questions on the `fathomdb-fts` arm, with stores
    built as in the D3 sweep.
  - The selection set is the 414 scored questions outside the 60-question
    answer set, the set Unit G used.
  - NFCorpus: the 323 test queries over the `comp01` inputs.
- **Simulation.** The harness removes the candidate words from the query
  text, then calls `search_text_only` on the `5b09afd44` engine. That
  equals the engine compiling with the extended list, as long as the query
  keeps at least one content token.
  - The config pins `engine_stopwords`, the engine's 63-word list at
    `5b09afd44` (the 51 original words plus the twelve of REQ40-D5),
    verbatim. A test pins the copy (AC40-70). The engine test pins
    `STOPWORDS.len()` as 63 plus the verdict's count (AC40-74), so a drift
    on either side is a RED test, not a failed measurement (review F5).
  - **Emptiness is checked against every list the rule evaluates** (review
    F2): each `w` alone, and every `L` the pruning loop produces. A query
    with no content token under a list would take the engine's raw-token
    fallback, which the simulation does not reproduce. A word that empties
    a question or NFCorpus query on its own is excluded from `K`. While
    some query is empty under the current `L`, the rule drops from `K` the
    word with the lowest `ΔnDCG` among the `K` words in such a query (ties:
    lower `Δall`, then alphabetical). Two words can empty a query only
    jointly, so this check runs on `L`, not just per word.
- **Per-word step.** Add each candidate `w` alone to the D3 list and
  measure two deltas:
  - `Δall(w)`: on the selection set, questions gained minus questions lost
    on session Recall-all@10;
  - `ΔnDCG(w)`: the NFCorpus nDCG@10 change.
- **Keep.** `K = {w : Δall(w) ≥ 1}`. A word with no measured gain is not
  admitted (ruling Q-H1).
- **Combined check.** `L` is the D3 list plus `K`. Three conditions:
  - (a) NFCorpus nDCG@10 ≥ 0.3265 − 0.002 and MRR@10 ≥ 0.5312 − 0.005
    (the final-head baseline and its tolerances);
  - (b) for every type, the selection-set session Recall-any@5 count is at
    least the D3 count minus 1;
  - (c) owner ruling N1, 2026-10-16, verbatim: *point delta ≥ +1
    question AND paired-bootstrap CI lower bound ≥ −1 question
    (bootstrap_ci, seed 404, 10,000 resamples) on Recall-all@10 over the
    selection set.* The delta is `L` against D3, in questions;
    `comp03_retrieval.bootstrap_ci` gives the CI of the per-question mean,
    so the bound in questions is the lower bound times the number of
    paired questions. This is review F4 as a non-regression bound in D3's
    sense (re-review N1 option B): it rejects a list whose losses could
    plausibly exceed its wins and admits a small net gain.
- **Pruning.** First apply the emptiness check. Then, while (a) or (b)
  fails, drop from `K` the word with the lowest `ΔnDCG` (ties: lower
  `Δall`, then alphabetical), apply the emptiness check again, and recheck.
  Clause (c) is checked once, on the final `L`.
- **Outcome.** If `K` empties or (c) fails, the verdict is "not adopted"
  and H1 makes no engine change.
- **Verdict file.** It records the per-word table, `L`, the combined
  metrics on both corpora, and the 56 scored answer-set questions. Those
  are reported, not used.

**REQ40-H2 — stop-list engine change. Superseded** by the owner ruling of
2026-10-16 (Unit H intro): no change to `STOPWORDS` on Slice 40; the
configurable list is Slice 41's. The text below is kept as written.
`STOPWORDS` in `fathomdb-query`
gains exactly the verdict's words.

- **Unchanged:**
  - query compilation stays the only place the list applies;
  - document indexing;
  - the vector branch's query embedding;
  - `d`, `c`, `d3`, `ii`, `1` and `b12` stay content, and the twelve
    Unit D words stay stopwords.
- **Updated in the same commit:**
  - the AC-038 private stop-list copy in `fts5_injection_safety.rs`
    gains the same words, its fixture unchanged (as ruling Q-D3 did for
    D3);
  - the interface docs' stop-list sentence, because H1 changes documented
    public behaviour;
  - a CHANGELOG entry.

**REQ40-H3 — H1 gate. Superseded** with REQ40-H2: there is no H1 commit to
gate. The text below is kept as written. (Orchestrator, at the H1 commit,
after the code review.) Verbatim: *(i) the post-change sweep's `fathomdb-fts` top-10 IDs
equal the simulation's for every question; (ii) COMP-01 FathomDB nDCG@10 ≥
0.3265 − 0.002 and MRR@10 ≥ 0.5312 − 0.005; (iii) COMP-03 sweep
`fathomdb-fts` per-type session Recall@5 not lower than the baseline beyond
one question per type; (iv) tc5-10k-latency `body-text-only` and projected
p50 not above the baseline beyond +25%; (v) COMP-02a 10k and 17k FathomDB
query-ready, p50 and p95 within the manifest tolerances; (vi) COMP-03 sweep
`fathomdb-hybrid`, at pool = limit, per-type session Recall@5 and Recall@10
not lower than the baseline beyond one question per type.* If clause (i),
(ii), (iii), (iv) or (v) fails, the H1 commit is reverted. If clause (vi)
fails, H1 is kept, and the hybrid regression is reported for follow-up
(owner ruling Q-H6, 2026-10-16). The H2 gate then takes the H1-commit
hybrid numbers as its reference.

**REQ40-H4 — vector-pool selection rule (offline, before the engine
change).** The rule lives in
`experiments/configs/comp-03/vector-pool-selection.v1.json`. Its verdict
is `vector-pool-verdict.v1.json`.

- **Candidates:** `P0` ∈ {10, 20, 50, 100}.
- **Measurement.** The orchestrator runs it on cuda:0, with a CUDA wheel
  built at `5b09afd44` (ruling Q-H4); the exact command is in
  [test-plan.md](test-plan.md) § 3g.1.
  - `comp03_unit_h.pool` calls the sweep's own hybrid retrieval
    (`comp03_answer.retrieve` with the `fathomdb-hybrid` product arm, the
    same store build, device check and vector coverage check), reused,
    not copied, with `top_k = P` for P ∈ {10, 20, 50, 100}. That call is
    `search(text, limit=P)` with rerank depth 0 and no graph arm
    (`comp_fts.py`), as in the sweep (review F11).
  - By § 1.6, pool `P0` at k = 5 and 10 ranks as the first k of the
    limit-`P0` list.
  - The verdict records the IDs for every P. The self-check the review
    proposed, `ids[P][:k] == ids[100][:k]`, does not hold on today's
    engine: pool = limit is exactly what breaks it (16 of 500 questions
    agree). The in-engine check is REQ40-H6 (iv) instead.
- **Reference.** `P0` = 10: today's limit-10 ranking, which the sweep and
  the answer runs use.
- **Constraints** on the selection set (414 questions):
  - for every type, the session Recall-any@5 and Recall-any@10 counts are
    at least the reference minus 1;
  - overall, the Recall-any@5, Recall-any@10 and Recall-all@10 counts are
    at least the reference minus 1.
- **Choice.** The largest `P0` that meets every constraint. The reference
  meets them by construction.
- **Verdict file.** It records the full table, with k = 20 and the
  answer-set questions reported but not used.
- **Run record** (verdict at harness `f9ae5c27b`: pool 10). The H2 run used
  two workers. two workers shared the attempt root `~/.cache/fathomdb-comp/h2-pool-selection`: the plan's command on cuda:0 (forward, wrote the verdict) and an orchestrator helper on GPU 1 (scratch `h2_pool_reverse.py`, the same `comp03_answer.retrieve` call and per-question files) that wrote questions 358–500 (143 questions). The verdict's `inputs.runner` and `inputs.device` reflect only the last writer (cuda:0). Cross-check: questions `e6041065`, `gpt4_1d4ab0c9`, `gpt4_7bc6cf22` and `gpt4_fe651585_abs` at pools 10 and 100, recomputed on GPU 0, gave 8/8 identical id lists. The verdict file is not hand-edited.

**REQ40-H5 — vector-pool engine change.**

- **The constant.** `VECTOR_FUSION_POOL` (`search_types.rs`) is the
  verdict's `P0`. It is `pub(crate)`, with a `test-hooks` accessor
  `vector_fusion_pool_for_test()`, because the contract is the prefix
  guarantee, not the number (review F10). No public Rust item is added.
- **The one behavioural change.** The vector branch of `search` hydrates
  up to `max(final_limit, VECTOR_FUSION_POOL)` surviving candidates;
  today it hydrates `final_limit`.
- **Unchanged:**
  - the phase-1 and phase-2 depth;
  - the text branch;
  - RRF: `Σ w/(30 + rank)`, text 3, vector 1;
  - truncation to `final_limit` after fusion;
  - the `vector_stage_only` seam, which still truncates the vector
    list to `final_limit`;
  - the test-only `search_limit_override`.
- **Contract.** For one database state and view, with `rerank_depth` 0 and
  no graph arm, `search(limit=k)` returns the first k results of
  `search(limit=k')` for every k < k' ≤ `VECTOR_FUSION_POOL`. Scores are
  bit-identical. Above the pool the guarantee is not claimed. The docs
  state only what AC40-79 tests (ruling Q-H2), although every
  post-fusion stage reads the untruncated list.
- **Explain.** The trace's `vector_hits` reports the hydrated count.
- **Docs, in the H2 commit.** `dev/interfaces/{python,typescript,rust,rust-sdk}.md`,
  `docs/guides/hybrid-search-filtering.md` and the CHANGELOG state the
  prefix guarantee, which is new public behaviour.

**REQ40-H6 — H2 gate (orchestrator, at the H2 commit, after the code
review).** The reference is the final-head baseline (it was the H1-commit
measurement until H1 was superseded). Verbatim: *(i) COMP-03 sweep
`fathomdb-hybrid` per-type session Recall@5 and Recall@10 not lower than the
final-head baseline beyond one question per type; (ii) `fathomdb-hybrid`
COMP-03 warm p50 ≤ the final-head baseline × 1.25 and p95 ≤ the final-head
baseline × 1.35, and `comp01-hybrid` `fathomdb-hybrid` p50 ≤ the final-head
baseline × 1.25; (iii)
the Unit E cross-SDK suite equal in every case, SDK and repetition; (iv)
for every sweep question, the hybrid limit-10 IDs and score bits equal the
first ten of a limit-`P0` call on the same store.*

- If (i) or (ii) fails, `P0` steps down to the next smaller candidate
  that met REQ40-H4, and the gate is measured again.
- At `P0` = 10, limit 10 ranks exactly as at the final head, so the step
  down ends there. A failure that remains there is not H2's and is
  reported.

**REQ40-H7 — cohort prefilter switch (H3).** In `read_projected_text_in_tx`,
when the filter carries at least one attribute pair:

- **Probe.** For each pair, run a bounded count in the reader
  transaction:
  `SELECT count(*) FROM (SELECT 1 FROM canonical_attributes WHERE
  attr_name = ? AND attr_value = ? LIMIT T + 1)`, with
  `T = COHORT_PREFILTER_MAX_ROWS = 256`. The cost is at most `T + 1`
  index entries per pair.
- **Switch.** If the smallest count is ≤ `T`, the search uses the
  cohort-prefiltered rank stream. That is the Defect C stream statement
  plus `AND p.write_cursor IN (SELECT write_cursor FROM
  canonical_attributes WHERE attr_name = ?a AND attr_value = ?v)` for
  that pair (the first in filter order on ties).
- **Otherwise,** and for a filter without attribute pairs, the Defect C
  path is unchanged.
- **Why `T` = 256.** The probe in § 1.6 puts the crossover between 92 and
  968 rows. 256 sits inside that range, on the conservative side for the
  engine's heavier node predicate. `T` is fixed here and is not retuned
  after measuring (ruling Q-H3).
- **The cost model is approximate** (review F12). The real trade-off is
  cohort size against matched-row count: for a query with few matches the
  Defect C stream is already cheap, and the cohort materialisation is
  overhead. Before the gate, the implementer records one report-only
  in-engine timing with the prefilter forced on at c10 (968 rows) on the
  10k store, so the engine's cost at the next size up is known
  ([test-plan.md](test-plan.md) § 3g.4). No retuning follows from it
  without a ruling.

**REQ40-H8 — identity and witnesses.**

- **Same results.** The existing attribute `EXISTS` already implies the
  prefilter, so the row set and order are unchanged: bm25 ascending, then
  `write_cursor` ascending, with Defect C's tie completion. Results,
  scores included, equal the rank stream's and the forced full sort's at
  every limit.
- **Lifecycle filtering is untouched.** Stale attribute rows can only
  enlarge the probe's count, which affects the plan choice, never the
  results.
- **Witnesses (`test-hooks`):**
  - Routes `projected_cohort_stream_strict_boundary` and
    `projected_cohort_stream_tie_completed`.
  - `FATHOMDB_FTS_COHORT_PREFILTER_FOR_TEST`: `on` ignores `T`, `off`
    never prefilters.
  - `FATHOMDB_FTS_FAIL_COHORT_FOR_TEST` breaks the cohort statement. The
    route list is then exactly
    `["projected_cohort_stream_fallback", "projected_rank_stream_strict_boundary"]`
    (or `…_tie_completed` for the second entry), so the fallback is
    observable (review F7).
  - With `FATHOMDB_FTS_COHORT_PREFILTER_FOR_TEST=off`, or a cohort above
    `T`, no `projected_cohort_*` route is recorded.
  - The cohort statement's plan has `LIST SUBQUERY` with
    `canonical_attributes_name_value_idx`, and no
    `USE TEMP B-TREE FOR ORDER BY`. The witness is never dropped to reach
    GREEN; design § 2g.4 gives the fallback if the bundled SQLite plans
    the statement differently (review F9).
- **The row-key potential** (attribute `EXISTS` keyed on
  `p.write_cursor`) stays a potential: H3 does not need it.

**REQ40-H9 — H3 gate (orchestrator, at the H3 commit, after the code
review).** The reference is the H2-commit measurement, so H2's change is
not charged to H3 (review F3). Verbatim:
*tc5-10k-latency `projected-c01` p50 at least 25% below the H2-commit
value; `projected-unfiltered`, `projected-c10` and `projected-c33` p50
and p95 within the manifest tolerances of the H2-commit values; COMP-02b
10k FathomDB c01 p50 lower than at the H2 commit, c10 and c33 p50 within
+25% and p95 within +35% of the H2 commit, violations 0, short-result
counts equal to the H2 commit's.* If the gate fails, the H3 commit is
reverted. The tolerances are the manifest's, with COMP-02b a manifest
benchmark (owner ruling Q-H5, REQ40-H10).

**REQ40-H10 — re-runs and baseline.** After GREEN and the code review, the
orchestrator re-runs every benchmark at the Unit H engine commit, with
receipts:

- COMP-01 FTS and hybrid;
- COMP-02a at 10k and 17k;
- COMP-02b at 10k;
- tc5-10k-latency;
- the COMP-03 sweep;
- the COMP-03 mix answer run.

The baseline manifest is then updated as [test-plan.md](test-plan.md)
§ 3g.7 sets out. By owner ruling Q-H5 (2026-10-16, review F8), COMP-02a
(10k and 17k) and COMP-02b (10k) become manifest benchmarks, taking it from
six to nine ids (three ids; "eight" in the first draft was a miscount,
corrected by the coordinator on 2026-10-16). They are added in the harness step, before the H1 gate,
with expected values from the final-head receipts, so REQ40-H9 (and the
superseded REQ40-H3 (v)) read manifest tolerances.
`test_slice_40_baseline_manifest_lists_the_six_benchmarks` is renamed for
nine ids under that ruling, and its new assertions are written RED
first.

**Unit I requirements** (added by owner ruling 2026-10-16). One heavy read
must not slow every later read on the same engine (§ 1.7). Unit I lands
as one engine commit after the H3 commit, and its gate is measured
against the H3 commit, so a failed gate reverts only Unit I. Every
number here is fixed before measurement.

**REQ40-I1 — one page cache per connection.** Workspace builds of the
bundled SQLite compile without `SQLITE_ENABLE_MEMORY_MANAGEMENT`, so
`pcache1` gives each connection its own page-cache group (mode 1):

- the one `.flag("-DSQLITE_ENABLE_MEMORY_MANAGEMENT")` line is removed
  from `third_party/libsqlite3-sys-0.38.1/build.rs`;
- no other compile flag changes, and `sqlite3.c` is not touched (the
  amalgamation digest stays
  `7e4799907bdc92cff8858db2c52f5a03e136d5989ead6e513c0ef64057baeb9e`);
- the vendored-tree digest changes. The new value is recorded in
  `FATHOMDB-PATCH.md` (which also gains a section for this change),
  `scripts/pinned-override-rot.json` (digest and rationale) and the
  `SQLITE_VENDOR_TREE_SHA256` constant in
  `scripts/check-pinned-override-rot.py`, and
  `scripts/check-pinned-override-rot.sh` passes;
- `dev/design/pinned-override-rot-guard.md` names both changes of the
  vendored crate.

**REQ40-I2 — a heavy read cannot take another reader's cache.** On a
store built by the test (§ 3h of the test plan), with each reader's
working set below its cache budget:

- (i) after one reader runs a read that touches more pages than its
  budget, that reader's `SQLITE_DBSTATUS_CACHE_USED` is at most 1.10 ×
  2,048,000 bytes (SQLite's default `cache_size`, `-2000`, which the
  production open path keeps; the `_for_test` RSS fixture in
  `graph_api.rs` persists `PRAGMA default_cache_size=64` into its own
  store only). The arithmetic is in AC40-96;
- (ii) the next pass of every other reader over its own working set has
  zero `SQLITE_DBSTATUS_CACHE_MISS` delta, as its pass just before the
  heavy read did;
- (iii) the bundled SQLite reports
  `sqlite3_compileoption_used("ENABLE_MEMORY_MANAGEMENT") == 0`;
- (iv) every pass returns the same ids before and after the heavy read.

(i), (ii) and (iii) fail on today's build; § 1.7 gives the probe that
shows it. The assertions use counters, not timing.

**REQ40-I3 — no public surface; the memory ceiling is unchanged.**

- No `EngineConfig` knob, no new API, no `PRAGMA cache_size` change.
  Each connection keeps the 2,000 KiB default.
- The ceiling is unchanged. Today it is the group's sum of limits:
  twelve 2,000 KiB budgets per engine by default (§ 1.7), about 24 MB
  with SQLite's per-page header, which one reader can hold alone.
  After Unit I each connection is held to its own 2,000 KiB, and the
  sum is the same.
- Resident page memory can rise toward that ceiling. In mode 2 the
  active reader recycles idle connections' pages, so the resident
  total tends toward the active working set. In mode 1 every
  connection keeps up to its own 2,000 KiB, so a long-running engine
  whose readers have all been used can hold close to the full 24 MB.
  On Tegra and Jetson targets, which build the same `build.rs`, about
  24 MB per open engine is small against the device memory, but it is
  a real increase over a mostly idle mode-2 engine.
- Results do not change (REQ40-I2 (iv); gate clause (iv)).

**REQ40-I4 — published Rust crates, stated.** Root patches do not reach
downstream Rust consumers (§ 1.7). Ruling Q-I1 (coordinator, on the
reviewer's recommendation, 2026-10-16) chose option (a), document only
(test-plan § 12):

- `dev/interfaces/rust.md` and `dev/interfaces/rust-sdk.md`, under
  "SQLite runtime configuration", say:
  - artifacts built from this repository use one page cache per
    connection;
  - a Rust build that resolves the registry `libsqlite3-sys` keeps
    SQLite's shared page cache, where one large read can slow later
    reads until the engine is reopened;
  - `LIBSQLITE3_FLAGS=-USQLITE_ENABLE_MEMORY_MANAGEMENT` at build time
    gives the same behaviour as the workspace build;
  - the flag is removed upstream (rusqlite commit `f31f7324`), so Rust
    consumers get the fix from the first `libsqlite3-sys` release that
    contains it, and the override is needed only until then.
- No runtime check or mitigation in the engine. A consumer-facing
  surface that reports the effective mode (for example from
  `sqlite3_compileoption_used`, as `connection_runtime.rs` already does
  for the Windows marker) is a ledger follow-up, not part of Unit I.
- `CHANGELOG.md` gains one `### Fixed` entry under 0.8.28.

**REQ40-I5 — report-only timing in a release build** (as F12 for H3).
An `#[ignore]`d test, `reader_page_cache_timing_report`, builds a store
from `FATHOMDB_TC5_INPUTS` (the `tc5-10000` input pack) and runs:

1. two body passes over the 100 queries;
2. one c01 `search_projected_text` query, on the Defect C route and,
   separately, on the cohort route, each on a fresh open;
3. three more body passes.

It prints each pass's p50 and p95, and each reader's `cache_used_bytes`
and `cache_miss` delta. The implementer runs it with
`--release` at the H3 commit and at the Unit I commit, and records the
output in the test plan (§ 3h.3). It gates nothing.

**REQ40-I6 — Unit I gate** (orchestrator, at the Unit I commit, after
the code review). The reference is the H3 commit, re-measured with the
v2 harness (fresh open per cell and per cohort). The Unit H gates' v1
numbers cannot serve: body, c10 and c33 there were measured after c01
on one open. The baseline engine `5b09afd44` is measured too,
for the record, but not gated. Verbatim:

- *(i) `reader_page_cache_timing_report` at the Unit I commit: each of
  the three body passes after the c01 query has p50 at most 1.15 × the
  second body pass before it, on both routes;*
- *(ii) tc5-10k-latency (fresh open per cell): every cell's p50 within
  +25% and p95 within +35% of the H3 commit re-measured with the v2
  harness, or lower; short-result
  counts equal to the H3 commit's;*
- *(iii) COMP-02b 10k (fresh open per cohort): FathomDB c01, c10 and
  c33 p50 within +25% and p95 within +35% of the H3 commit re-measured
  with the v2 harness, or lower;
  violations 0; short-result counts equal to the H3 commit's;*
- *(iv) COMP-01 FTS nDCG@10 and the COMP-03 sweep's FTS-arm metrics
  equal to the H3 commit's to every printed digit; COMP-01 hybrid and
  the sweep's hybrid arm within the manifest tolerances of the H3
  commit; COMP-02a 10k and 17k FathomDB query-ready, p50 and p95 within
  the manifest tolerances of the H3 commit, or lower.*

If (i), (ii) or (iii) fails, the Unit I commit is reverted. A failure in
(iv) is analysed first: a changed result would be a defect, because a
page cache cannot change results.

**REQ40-I7 — measurement method and baseline.**

- The harness fix (`comp03-slice40`, test-plan § 3h.4) is the method for
  every Unit I measurement:
  - the tc5-10k-latency summary is schema
    `comp-02b.latency.summary.v2`, with config version 2 and
    `measurement = fresh_open_per_cell`;
  - COMP-02b runs from `experiments/configs/comp-02/filtered-10000.v2.json`
    and records `fresh_open_per_cohort` per cell.
- Harness notes (review F7, F8):
  - the classifier writes `comp02b-latency-v2-measurement-plan.v1.json`
    on the first classification of a v2 receipt; that plan file is
    committed with the first v2 receipt;
  - `run_filtered_cell` is arm-generic, so the SurrealDB arm also opens
    per cohort now. v1 and v2 numbers are not comparable for either arm;
  - each cell's timed pass still follows its own warm pass on the same
    open, so c01 measures its own post-heavy state. That is inherent
    and intended: the method isolates cells from each other, not a
    query from its own cell;
  - `peak_rss_mib` is the maximum of process RSS over the three opens.
    RSS rarely falls after a close, so in practice it is the last
    open's figure.
- The baseline manifest's tc5-10k-latency and comp02b-10000 values were
  measured with the single-open method. After a passed gate, those two
  entries take:
  - the Unit I values as expected values;
  - the v2 commands (`filtered-10000.v2.json`);
  - a note naming the method change.

  Their tolerances do not change. The other entries change only
  through REQ40-H10's re-run.

**Unit J requirements** (added by owner ruling 2026-10-16). Remove the
test-order dependency on the SQLite runtime latch (§ 1.8), so Unit I's
test runs are not confounded by it. Unit J lands before Unit I's RED and
GREEN. The owner approved the edits to existing tests: guards only, with
no assertion or fixture change.

**REQ40-J1 — guard every raw-first test.** Each test that can open a raw
`rusqlite` connection before any `Engine::open` in its process, in a
binary that also opens engines, calls
`configure_runtime(RuntimeSqliteMode::Performance).expect("configure test runtime")`
first, following `pr_g0_identity.rs`. That means a file-level
`configure_test_runtime()` helper with the latch comment, called at the
top of the test, or of the shared helper where the raw open is. The
sites are the five that the sweep (test-plan § 3i.2) confirms:

- `pr_g8_dangling_edges.rs`, `s20_endpoint_probe_hits_partial_index_no_scan`;
- `tc57_governed_write_race.rs`, `wal_pair` (the three `tc57_mechanism_*`
  tests);
- `pr_g3_read_collection.rs`, `read_collection_plan_is_index_driven_no_scan_no_temp_btree`;
- `truncate_wal.rs`, `noncurrent_database_is_refused_without_mutation`;
- `truncate_wal.rs`, `malformed_wal_with_noncurrent_standalone_main_refuses_without_mutation`.

No assertion, fixture or test name changes.

**REQ40-J2 — the `TooLate` message names the remedy.** The `Display`
text of `RuntimeConfigurationError::TooLate` becomes:

*"SQLite runtime configuration is too late: SQLite was initialized in
this process before FathomDB configured it; call configure_runtime or
open an Engine before any other SQLite connection (this process must
restart)"*.

The variant, the latch and the other messages do not change.

**REQ40-J3 — docs.** `docs/reference/config.md` (Process-start SQLite
mode) and `dev/interfaces/rust.md` (SQLite runtime configuration) gain
one sentence each. It says that in a Rust process that also uses
`rusqlite`, the first SQLite connection of any kind initializes the
runtime, so `configure_runtime` or the first `Engine::open` must come
before it.

**REQ40-J4 — no engine behaviour change.**
`ADR-0.8.25-sqlite-runtime-configuration` stays as it is: an implicit
open after an outside initialization still fails closed with `TooLate`.

**REQ40-J5 — measured.** Each of the five guarded binaries is looped in
parallel before and after the guards (test-plan § 3i.3). After the
guards, 0 of 30 runs fail with `TooLate` for each binary under default
features and under `test-hooks`.

## 3. Non-goals

- No BM25 `k1`/`b` tuning and no change to `RRF_K` or the arm weights.
- No change to the stopword list for Defects A to C; Unit D3 extends it
  (REQ40-D5).
- No new public API, no new knob, no schema change, no migration.
- No near-duplicate detection; only byte-identical bodies are affected, as
  they already are by fusion's dedupe on body.
- No change to the direct text-only candidate window (§ 4).
- No fielded title boost through declared `searchable` projections.
  Potential, deferred by owner ruling 2026-10-16 (`status.md`).
- Unit D does not touch the attribute-filter row-key change or the
  filter-first plan; both stay listed as potentials in `status.md`.
- No rerank and no graph arm in the hybrid arm.
- No performance assertion in tests for Defect C; the orchestrator
  measures latency on the 10k corpus after W4.
- No change to the frozen IR-C experiment harnesses
  (`tests/support/ir_retrieval.rs` and its includers), which document
  themselves as a verbatim historical copy.
- The stale `Σ 1/(60 + rank)` formula in the public docs was a non-goal for
  Defects A to D; Unit E now fixes it (REQ40-E4).
- Units E to G do not touch the five unclassified pre-existing COMP
  receipts, landing, the attribute-filter row-key and filter-first
  potentials, or fielded title boost.
- No product change for Unit G this round (REQ40-G7).
- Unit H does not change `RRF_K`, the branch weights or the fusion
  formula, document indexing, the hybrid path's attribute filter or
  `search_text_only`'s candidate window. It does not build tiered
  evidence rendering, a session-grouped retrieval API, fielded weighting
  or query rewriting (analysis § 5); those stay owner decisions.
- Unit H adds the filter-first potential as the cohort prefilter (H3); the
  attribute-filter row-key potential stays a potential.
- Unit I does not add a page-cache knob, change `cache_size`, install a
  custom page cache (`SQLITE_CONFIG_PCACHE2`) or change any other SQLite
  compile flag. It adds no runtime mitigation for published Rust crates
  (ruling Q-I1: option (a), document only).

## 4. Candidate bound of `search_text_only`: out of scope

The direct text-only path fetches a fixed node window of
`MAX_SEARCH_RESULT_LIMIT` (100) FTS rows for every accepted `limit`
(`SearchReaderWork::text_only`, `direct_text_candidate_limit`), then fuses
and truncates to `limit`. `candidate_limit = max(search_limit_override,
limit)` in `search_api.rs` feeds the vector stage, not this window. Fusion
already collapsed duplicate bodies before this slice, so a caller asking
for close to 100 results can get fewer than `limit` when the window holds
duplicate rows; for the default `limit = 10` that needs more than 90
duplicate rows in the window. The fix is not a small over-fetch: the window
would have to count distinct bodies inside `collect_complete_rank_boundary`
while keeping the Slice 20 boundary-tie completion and the Slice 23 prefix
stability. Recorded as a follow-up; AC40-13 guards the default limit.

## 5. Acceptance criteria

Each AC is checked by the named test. Engine tests are in
`src/rust/crates/fathomdb-engine/tests/slice40_fts_ranking.rs` unless
another file is named; the Defect C tests (AC40-27 to AC40-31) are in
`tests/slice40_projected_fts_stream.rs`, which needs `--features
test-hooks`; query tests are in the `tests` module of
`src/rust/crates/fathomdb-query/src/lib.rs`. Defect C's criteria start at
AC40-27 because AC40-23 to AC40-26 were already assigned. Units E to G's
harness tests (AC40-49 to AC40-69) are in `tests/experiments/test_sdk_parity_slice40.py`,
`test_baseline_check.py` and `test_comp03_evidence.py` on the comp branch.
Unit D's
harness tests (AC40-33 to AC40-45) are in `tests/experiments/test_comp03_retrieval.py`
and `tests/experiments/test_comp03_answer.py` on the `slice/0.8.28-40-comp03`
branch; its D3 tests are in the `fathomdb-query` test module. Unit H's
harness tests (AC40-70 to AC40-73, AC40-77, AC40-78) are in
`tests/experiments/test_comp03_unit_h.py` on the comp branch; its engine
tests are in the `fathomdb-query` test module (AC40-74),
`tests/slice40_vector_pool.rs` (AC40-79 to AC40-82) and
`tests/slice40_cohort_prefilter.rs` (AC40-84 to AC40-90), both with
`--features test-hooks`.

| AC | Check | Test | REQ |
| --- | --- | --- | --- |
| AC40-01 | A duplicate pair at text ranks 13/14 does not outrank the distinct rank-1 body and sits 13th | `single_arm_duplicate_body_does_not_outrank_higher_ranked_distinct_body` | 01 |
| AC40-02 | A duplicated body scores exactly `w_text/(K + first rank)`; later bodies keep positional ranks | `single_arm_duplicate_body_scores_once_at_first_occurrence_rank` | 01 |
| AC40-03 | The representative of a duplicated body is its first occurrence | `duplicate_body_representative_is_first_occurrence` | 03 |
| AC40-04 | The rule holds on the vector and graph arms | `vector_arm_duplicate_body_scores_once`, `graph_arm_duplicate_body_scores_once` | 01 |
| AC40-05 | The dedupe is per arm: one vector term plus one text term | `duplicate_dedupe_is_per_arm_not_global` | 01, 02 |
| AC40-06 | Cross-arm agreement accumulates, with and without in-arm duplicates | `cross_arm_agreement_still_accumulates_with_in_arm_duplicates`, `three_arm_agreement_sums_one_term_per_arm`; existing `pr_g9_rrf_fusion.rs` tests | 02 |
| AC40-07 | Property: text arm only, fused order equals first-occurrence order, scores strictly decreasing | `prop_text_only_fusion_is_monotone_in_first_occurrence_rank` | 01 |
| AC40-08 | Property: any three arms equal the once-per-arm reference (order, score bits, identity) | `prop_fusion_matches_once_per_arm_reference` | 01–03 |
| AC40-09 | Property: inputs with no in-arm duplicate are unchanged | `prop_distinct_within_arm_inputs_are_unchanged` | 03 |
| AC40-10 | Property: `fuse_rrf(v, t) == fuse_three_arms(v, t, [])` | `prop_two_arm_fusion_equals_three_arm_with_empty_graph` | 03 |
| AC40-11 | End to end: duplicate pair does not win rank 1; top 10 are the first ten distinct rows | `text_only_duplicate_bodies_do_not_win_rank_one` | 01 |
| AC40-12 | End to end: the pair surfaces once, at its first FTS rank | `text_only_duplicate_body_surfaces_once_at_first_occurrence` | 01, 03 |
| AC40-13 | End to end: the default limit is filled with distinct bodies | `text_only_duplicate_bodies_still_fill_the_limit` | 03 |
| AC40-14 | End to end: a 1-character token ranks the row carrying it first | `text_only_single_char_token_ranks_matching_row_first` | 04 |
| AC40-15 | End to end: "d3" and "1" each rank their row first | `text_only_two_char_and_digit_tokens_rank_matching_rows_first` | 04 |
| AC40-16 | End to end: short tokens with FTS5 control syntax never error | `text_only_short_tokens_with_fts5_control_syntax_never_error` | 06 |
| AC40-17 | Property: content terms are every non-stopword token of any length, in first-seen order; else fallback | `prop_content_terms_are_every_non_stopword_token` | 04, 05, 07 |
| AC40-18 | Property: every term is one FTS5 literal; content terms are alphanumeric | `prop_every_term_is_one_fts5_string_literal`, `prop_content_path_terms_are_alphanumeric` | 06 |
| AC40-19 | Unit: short tokens kept ("c", "1", "d3", "ii"); short-only query takes the content path | `one_and_two_char_content_tokens_are_kept`, `short_token_only_query_takes_the_content_path` | 04, 07 |
| AC40-20 | Unit: stopwords, including one-character ones, are dropped; empty fragments never emitted | `stopwords_are_dropped`, `short_stopwords_are_still_dropped`, `empty_split_fragments_are_never_emitted` | 04, 05 |
| AC40-21 | Unit: fallback unchanged for all-stopword queries | `all_stopword_query_falls_back_to_raw_or`, `all_short_stopword_query_still_falls_back_to_raw_or`, `escapes_double_quotes_in_fallback_tokens` | 07 |
| AC40-22 | AC-038 parity holds with short tokens: the corpus has two rows with 1–2 character tokens, the query alphabet has "d3", "c", "ii", "1", and the reference mirror keeps short tokens | `fts5_injection_safety.rs`: `fts5_control_syntax_is_safe_and_matches_reference` | 04, 06 |
| AC40-23 | No doc still states the old behavior | Review of the files in [design.md](design.md) § 5, plus `rg -n "shorter than three\|≥3 chars\|sub-3-char" src/rust/crates/fathomdb-query src/rust/crates/fathomdb-engine/src` returns nothing | 08 |
| AC40-25 | End to end: a possessive query ("Cow's milk") ranks the topical row first and does not error | `text_only_possessive_query_ranks_topical_row_first` | 04 |
| AC40-26 | Unit: possessive and contraction fragments ("s", "t") are content tokens, pinned explicitly | `possessive_and_contraction_fragments_are_content_tokens` | 04, 05 |
| AC40-27 | Projected search order equals the documented full sort for limits 1, 2, 3, 5, 8, 10, 12, 100 on a corpus with three bm25 tie groups | `projected_search_matches_documented_order_across_tie_boundaries` | C2, C3 |
| AC40-28 | An eligible projected search records route `projected_rank_stream_strict_boundary` or `projected_rank_stream_tie_completed`, and its query plan has no `USE TEMP B-TREE FOR ORDER BY`; the forced full sort records `projected_full_sort_forced` and its plan does | `projected_search_uses_rank_stream_without_temp_order_by_btree` | C1 |
| AC40-29 | With the stream statement broken, projected search returns the documented order and records exactly one route, `projected_full_sort_fallback` | `projected_search_falls_back_to_full_sort_when_stream_fails` | C5 |
| AC40-30 | An attribute-filtered projected search returns only in-cohort rows in the documented order restricted to the cohort; the unfiltered list is unchanged | `projected_search_attribute_filter_keeps_cohort_and_order` | C4 |
| AC40-31 | Exactly `limit` rows at strict boundaries (2, 8) with 12 matches | `projected_search_returns_exactly_limit_rows_at_strict_boundaries` | C3 |
| AC40-32 | Body-path and existing projected-search contracts unchanged | existing `slice20_fts_rank_stream.rs`, `slice18_retrieval_result_limits.rs`, `slice45_nested_source_projections.rs`, `slice30_dependency_closure.rs`, `slice80_search_view_paths.rs`, `slice35_eligibility_pretruncation.rs` | C6 |
| AC40-33 | Recall-any, Recall-all and MRR on hand-built gold: two answer sessions, one found at 5, both at 10; two rounds of one session do not satisfy Recall-all; `None` without relevant rounds | `test_recall_all_needs_every_answer_session_while_recall_any_needs_one`, `test_two_rounds_of_one_session_do_not_satisfy_recall_all`, `test_metrics_are_none_without_relevant_rounds_and_zero_when_nothing_found` | D1 |
| AC40-34 | Seeded bootstrap CI; paired deltas are treatment − control overall and per type | `test_bootstrap_ci_is_seeded_and_degenerate_on_constant_differences`, `test_paired_deltas_are_treatment_minus_control_overall_and_per_type` | D1 |
| AC40-35 | Decision rule: adopt when (a), (b), (c) hold; reject on each failed clause, including the boundary cases lower bound = 0 and overall delta = 0 | `test_decision_rule_adopts_when_all_three_clauses_hold`, `test_decision_rule_rejects_when_any_clause_fails` (4 cases), `test_decision_rule_accepts_an_overall_delta_of_exactly_zero` | D3 |
| AC40-36 | Sweep config is typed, pins the v1 corpus and document unit, the arms, k = 5 and 10, the bootstrap seed and the rule verbatim; a wrong schema is refused | `test_sweep_config_is_typed_and_states_the_rule_verbatim` | D1, D3 |
| AC40-37 | Gold-leak discipline: the sweep's `convert_question` and `gold_for` are `comp03_answer`'s; gold changes leave hybrid retrieval byte-identical | `test_sweep_reuses_answer_harness_conversion_and_gold`, `test_gold_changes_leave_hybrid_retrieval_byte_identical` | D2 |
| AC40-38 | Every question (abstentions included) is retrieved, abstentions are not scored, rerun does no new work, a wrong dataset digest is refused | `test_sweep_covers_every_question_and_resume_does_no_new_work`, `test_sweep_refuses_a_dataset_whose_digest_differs` | D2 |
| AC40-39 | Device: a CUDA request resolved to CPU (or unresolved) is refused; the sweep records requested and effective device | `test_device_check_refuses_a_cuda_request_that_resolved_to_cpu`, `test_sweep_hybrid_arm_records_the_resolved_device_and_writes_a_verdict` | D2 |
| AC40-40 | Hybrid store: the fts arm's plain bodies, one vector row per document, at least one `vector` hit, crosses a vocabulary gap FTS cannot; coverage check refuses a store without vectors; build records drain (embedding) time apart | `test_hybrid_arm_store_has_one_vector_row_per_document_and_vector_hits`, `test_vector_coverage_check_refuses_a_store_without_vectors`, `test_sweep_hybrid_arm_records_the_resolved_device_and_writes_a_verdict` | D1, D2 |
| AC40-41 | Withdrawn by R1 ruling Q-D2: the `fathomdb-fts-projected` variant was dropped with the JSON-body hybrid shape | — | — |
| AC40-42 | `comp_fts`: the `fathomdb` arm's source and settings are byte-unchanged; `fathomdb-hybrid` and campaign `comp01-hybrid` are registered; the sweep's fts record carries v1's settings | `test_comp_fts_fts_arm_is_byte_unchanged_and_hybrid_arm_is_registered`, `test_sweep_covers_every_question_and_resume_does_no_new_work` | D4 |
| AC40-43 | v2 answer config keeps v1's questions, corpus, document unit, evidence, models and metrics, widens to three arms at k = 10, gates only the hybrid arm | `test_v2_config_keeps_v1_inputs_and_widens_to_three_arms_at_k10` | D4 |
| AC40-44 | Arm order: rotation equals v1's alternation for two arms; each of three arms takes each position once per three questions | `test_arm_order_rotation_reduces_to_the_v1_alternation_for_two_arms`, `test_arm_order_rotation_puts_each_of_three_arms_in_each_position_once` | D4 |
| AC40-45 | Hybrid gate: a `reject` sweep verdict runs no hybrid work; `adopt` runs it; v1 tests unchanged | `test_hybrid_arm_is_skipped_unless_the_sweep_verdict_is_adopt`, `test_hybrid_arm_runs_when_the_sweep_verdict_is_adopt`, the pre-existing `test_comp03_answer.py` tests | D4 |
| AC40-46 | D3: the twelve words are stopwords and `d`, `c`, `d3`, `ii`, `1`, `b12` are not; each word alone takes the fallback; possessive and contraction fragments and the short function words are dropped; short content tokens survive | `unit_d_words_are_in_the_stop_list_and_content_letters_are_not`, `each_unit_d_word_alone_takes_the_fallback`, `possessive_and_contraction_fragments_are_dropped`, `short_function_words_are_dropped`, `short_content_tokens_survive_the_unit_d_stop_list` | D5 |
| AC40-47 | D3: every other Slice 40 compiler and engine test still passes unchanged; `possessive_and_contraction_fragments_are_content_tokens` is deleted (R1 ruling Q-D1) and the AC-038 private stop-list copy gains the twelve words (Q-D3) | the `fathomdb-query` test module, `slice40_fts_ranking.rs`, `fts5_injection_safety.rs`, `slice40_projected_fts_stream.rs` | D5 |
| AC40-48 | D3 measurement gate (REQ40-D6) | orchestrator measurement after W4 | D6 |
| AC40-49 | Parity comparison is exact in `(id, score, branch)`: order, a one-ULP score change, branch, hit count and query set all mismatch | `test_parity_requires_exact_id_score_and_branch_per_hit` | E2 |
| AC40-50 | Text-only parity fixture: at least two duplicate bodies; every D3 word, the short content tokens `d`, `c`, `d3`, `1`, `ii` and an apostrophe across the queries; deterministic | `test_text_only_parity_fixture_carries_the_slice_40_query_shapes` | E1 |
| AC40-51 | Projected parity fixture: a bm25 tie group straddles the limit; the filter case returns a strict, non-empty subset | `test_projected_parity_fixture_puts_a_bm25_tie_group_across_the_limit` | E1 |
| AC40-52 | Dense parity fixture: one vector row per document; `search` returns a `vector` hit | `test_dense_parity_fixture_store_embeds_every_document` | E1 |
| AC40-53 | Registry measures `search_text_only`, `search_projected_text` and `search`; the latter two normalise to ordered hits | `test_registry_measures_the_three_slice_40_verbs_with_ordered_hits` | E1 |
| AC40-54 | Python driver emits `(id, score, branch)` hits on request, equal to the Python oracle on the same file, for all three cases; requests without a normalizer still return IDs only | `test_python_driver_hits_equal_the_python_oracle_on_the_same_database` (2 cases), `test_python_driver_dense_hits_equal_the_python_oracle`, `test_pre_existing_requests_without_a_normalizer_still_return_ids_only` | E1, E2 |
| AC40-55 | TypeScript and Rust drivers pass the same cases; the suite records every SDK equal to the oracle in every repetition | the TS and Rust driver suites (`test_sdk_typescript_driver.py`, `sdk_drivers/rust/tests`) extended at W4; the suite run by the orchestrator | E1–E3 |
| AC40-56 | Interface docs describe the three verbs' Slice 40 behaviour; the public RRF formula reads `RRF_K = 30`, 3:1; `agent-lint-docs.sh` passes | review plus `./scripts/agent-lint-docs.sh` and `./scripts/agent-lint-md.sh` | E4 |
| AC40-57 | Drift check: PASS inside tolerance, DRIFT outside (absolute, relative and per-question tolerances), MISSING when absent; non-zero exit on DRIFT or MISSING | `test_every_metric_inside_its_tolerance_passes`, `test_a_metric_outside_its_tolerance_drifts`, `test_a_missing_metric_is_reported_and_is_not_a_pass`, `test_question_tolerance_is_one_question_of_the_type_count`, `test_cli_exits_non_zero_on_drift_or_missing` | F3 |
| AC40-58 | Baseline manifest lists the six benchmarks with inputs, commit, build recipe, command, receipt and tolerances; answer accuracy marked not a gate; nine benchmarks, with COMP-02a (10k, 17k) and COMP-02b (10k), after the Unit H harness step (owner ruling Q-H5) | `test_slice_40_baseline_manifest_lists_the_six_benchmarks`, renamed `…_nine_benchmarks` under Q-H5 | F1, H10 |
| AC40-59 | Latency harness keeps the six diagnostic cells and the script's percentiles | `test_latency_harness_keeps_the_diagnostic_cells_and_percentiles` | F2 |
| AC40-60 | Receipts record engine commit, native SHA-256 and version; a malformed commit is refused | `test_engine_identity_records_the_commit_and_the_native_digest` | F4 |
| AC40-61 | Evidence caps: per-round then total, rank order; the baseline policy shows the same rounds as `render_evidence` | `test_caps_apply_per_round_then_total_in_rank_order`, `test_the_baseline_policy_matches_the_answer_harness_renderer` | G1 |
| AC40-62 | Session expansion order, its stop at the cap, and blank rounds at 0 characters | `test_expansion_appends_remaining_rounds_of_hit_sessions_by_best_hit_rank`, `test_expansion_stops_at_the_total_cap`, `test_expansion_enumerates_blank_rounds_at_zero_chars` | G1 |
| AC40-63 | Prompt metrics, including untruncated has-answer and the gold-character fraction; the twelve-policy grid | `test_prompt_metrics_count_gold_sessions_and_answer_rounds`, `test_untruncated_and_char_fraction_metrics_see_a_cut_answer_round`, `test_policy_grid_is_the_stated_twelve_policies` | G1 |
| AC40-64 | Rule: maximise has-answer in prompt; per-type untruncation constraint pooled (3) and for fathomdb-fts (1); fewer chars on ties; the lowest-chars candidate loses on untruncation; the tolerance boundary; baseline fallback | `test_rule_maximises_has_answer_and_rejects_policies_that_cut_answer_rounds`, `test_pooled_tolerance_is_three_counts_and_fts_tolerance_is_one`, `test_rule_also_holds_for_fathomdb_fts_alone_and_falls_back_when_the_pooled_winner_breaks_it`, `test_rule_breaks_ties_on_fewer_chars_and_keeps_the_baseline_when_nothing_beats_it` | G2 |
| AC40-65 | One implementation: with expansion on, `render_evidence` shows exactly the evaluator's rounds; the evidence record covers the expanded text and counts | `test_renderer_shows_exactly_the_evaluator_rounds_with_expansion`, `test_evidence_record_digests_the_expanded_text_and_counts_it` | G4 |
| AC40-66 | Chronological rendering (newest last, dates kept, same rounds, date ties by session then round position); the sentence only when enabled; both off byte-identical to v2 | `test_chronological_rendering_orders_by_session_date_newest_last_and_keeps_dates`, `test_chronological_rendering_breaks_date_ties_by_session_then_round_position`, `test_chronological_rendering_selects_the_same_rounds_as_rank_order_under_the_cap`, `test_recency_sentence_is_added_only_when_enabled_and_v2_rendering_is_unchanged` | G3 |
| AC40-67 | Adoption rule, with its boundaries (overall −1 with KU plus temporal flat adopts; KU +1 / temporal −1 adopts) and a per-type report | `test_recency_rule_adopts_unless_ku_plus_temporal_drops_or_overall_drops_by_more_than_one`, `test_recency_rule_boundaries` | G5 |
| AC40-68 | v3, v4a, v4b and v4c configs: single-factor steps, provisional verdicts; the verdict file names its inputs (470 scored, 414 selection, 60 answer IDs of which 56 scored) and the expected answer-set delta | `test_v3_v4a_v4b_v4c_configs_add_one_factor_each`, `test_evidence_verdict_file_names_its_inputs` | G2, G5 |
| AC40-69 | The best-mix config is built from the verdicts on the arm of record; v4c contingency; renderer caps and policy are mutually exclusive | `test_mix_config_follows_the_three_verdicts`, `test_mix_config_follows_the_arm_of_record`, `test_mix_config_needs_v4c_for_chronological_order_alone`, `test_render_evidence_refuses_a_policy_together_with_caps` | G2, G6 |
| AC40-70 | Done; its verdict is evidence for Slice 41. Stop-list selection config: typed; holds the 47 candidates (no `d`, no `ve`, no current stopword), `engine_stopwords` equal to the 63-word list at `5b09afd44` verbatim, the 414-question selection set and the rule verbatim, including the emptiness check; a wrong schema is refused | `test_stoplist_selection_config_states_the_candidates_and_rule` | H1 |
| AC40-71 | Done; its verdict is evidence for Slice 41. Simulation: removing words harness-side gives the content tokens of compiling with them as stopwords (property); a word that empties a query alone is excluded; two words that empty a query only jointly cause the lower-`ΔnDCG` one to be pruned | `test_stripping_words_equals_compiling_with_them_as_stopwords`, `test_a_word_that_empties_a_query_is_excluded` | H1 |
| AC40-72 | Done; its verdict is evidence for Slice 41. Rule: keep `Δall ≥ 1`; combined constraints (a)–(c), with (c) a point delta ≥ +1 and a bootstrap CI lower bound ≥ −1 question (owner ruling N1); pruning order by lowest `ΔnDCG`, then `Δall`, then name; boundaries (exactly −0.002, exactly −1 question, exactly +1, CI lower bound exactly −1 question); "not adopted" when nothing gains or (c) fails | `test_stoplist_rule_keeps_gaining_words_and_prunes_until_constraints_hold`, `test_stoplist_rule_boundaries`, `test_stoplist_rule_is_not_adopted_when_nothing_gains` | H1 |
| AC40-73 | Done; its verdict is evidence for Slice 41. Stop-list verdict file names its inputs (414 selection, 56 scored answer-set, 323 NFCorpus queries, engine commit) and the per-word table | `test_stoplist_verdict_names_its_inputs` | H1 |
| AC40-74 | **Superseded** (owner ruling 2026-10-16: no default `STOPWORDS` change; Slice 41). The verdict's words are stopwords and `STOPWORDS.len()` is 63 plus their count; `d`, `c`, `d3`, `ii`, `1`, `b12` are not; the Unit D twelve stay; each new word alone, upper-cased, takes the fallback (the raw token keeps its case); the § 1.6 question shapes compile to their content terms | `unit_h_words_are_in_the_stop_list_and_content_tokens_are_not`, `each_unit_h_word_alone_takes_the_fallback`, `unit_h_question_shapes_compile_to_content_terms` | H2 |
| AC40-75 | **Superseded** (owner ruling 2026-10-16: no default `STOPWORDS` change; Slice 41). The AC-038 private copy gains the same words; every other Slice 40 compiler and engine test passes unchanged | `fts5_injection_safety.rs`; the `fathomdb-query` test module, `slice40_fts_ranking.rs` | H2 |
| AC40-76 | **Superseded** (owner ruling 2026-10-16: no default `STOPWORDS` change; Slice 41). H1 gate (REQ40-H3), at the H1 commit | orchestrator measurement after the code review | H3 |
| AC40-77 | Pool selection config: typed; candidates {10, 20, 50, 100}, reference 10, the rule verbatim; the pool command calls the sweep's hybrid retrieval function, not a copy | `test_vector_pool_selection_config_is_typed_and_states_the_rule` | H4 |
| AC40-78 | Pool metrics use the first k of the limit-`P` list; the rule picks the largest pool meeting every constraint; the reference always passes; boundaries at exactly −1 question | `test_pool_metrics_use_prefixes_of_the_limit_p_ranking`, `test_pool_rule_picks_the_largest_pool_that_keeps_recall`, `test_pool_rule_boundaries` | H4 |
| AC40-79 | Hybrid results are prefix-stable up to the pool: for limits k < k' ≤ `VECTOR_FUSION_POOL`, ids and score bits of limit k equal the first k of limit k', on a corpus where today's engine breaks the prefix | `hybrid_results_are_prefix_stable_up_to_the_vector_pool` | H5 |
| AC40-80 | The vector branch hydrates `min(max(limit, pool), surviving)` rows: explain `vector_hits` at limits below, at and above the pool; the pool is read through `vector_fusion_pool_for_test()` | `vector_branch_hydrates_the_pool_below_it_and_the_limit_above_it` | H5 |
| AC40-81 | Fused scores still equal `Σ w/(30 + rank)` with text 3 and vector 1, for a body found by both branches with the vector rank beyond `limit` | `fused_score_formula_is_unchanged_with_the_pool` | H5 |
| AC40-82 | `vector_stage_only` returns `limit` vector hits; the filtered vector branch keeps its cohort; the existing hybrid tests pass unchanged or are reported for a ruling | `vector_stage_only_and_filtered_vector_search_keep_their_contracts`; existing `ga2_vector_stage_seam.rs`, `slice18_retrieval_result_limits.rs`, `pr_g9_rrf_fusion.rs` | H5 |
| AC40-83 | H2 gate (REQ40-H6), at the H2 commit, cross-SDK parity and the in-engine prefix check included | orchestrator measurement and the Unit E suite after the code review | H6 |
| AC40-84 | A cohort of at most `T` rows takes the cohort route; its plan has `LIST SUBQUERY` on `canonical_attributes_name_value_idx` and no temp B-tree for ORDER BY | `small_cohort_takes_the_cohort_prefilter_route` | H7, H8 |
| AC40-85 | Cohort route results equal the Defect C stream's and the forced full sort's (ids, score bits, order) for limits 1, 2, 3, 5, 8, 10, 12, 100 across three bm25 tie groups | `cohort_prefilter_results_equal_rank_stream_and_full_sort` | H8 |
| AC40-86 | Boundary: `T` rows prefilter, `T + 1` rows keep the Defect C stream | `cohort_of_t_rows_prefilters_and_t_plus_one_keeps_the_rank_stream` | H7 |
| AC40-87 | A failing cohort statement falls back to the Defect C stream with the same results; the route list is exactly `projected_cohort_stream_fallback` then the Defect C route; forced off records no cohort route | `cohort_prefilter_failure_falls_back_to_the_rank_stream` | H8 |
| AC40-88 | Two attribute pairs: the smaller cohort prefilters; results equal the forced-off results | `multi_attribute_filter_prefilters_on_the_smallest_cohort` | H7 |
| AC40-89 | Lifecycle and the other filter fields still apply on the cohort route (superseded node, `kind`) | `cohort_prefilter_keeps_lifecycle_and_other_filter_fields` | H8 |
| AC40-90 | Property: random corpora, cohorts and limits, the cohort route equals the forced full sort | `prop_cohort_prefilter_matches_the_full_sort` | H8 |
| AC40-91 | Defect C's tests (AC40-27 to AC40-32) pass unchanged | `slice40_projected_fts_stream.rs` and the AC40-32 list | H8 |
| AC40-92 | H3 gate (REQ40-H9), at the H3 commit against the H2 commit | orchestrator measurement after the code review | H9 |
| AC40-93 | Interface docs, the hybrid guide and the CHANGELOG describe H2 and H3 (H1 superseded: no stop-list doc change on Slice 40); `agent-lint-docs.sh` passes | review plus `./scripts/agent-lint-docs.sh` and `./scripts/agent-lint-md.sh` | H2, H5 |
| AC40-94 | Re-runs with receipts at the Unit H commit; baseline manifest updated as test-plan § 3g.7 | orchestrator | H10 |
| AC40-95 | The bundled SQLite is built without `SQLITE_ENABLE_MEMORY_MANAGEMENT`: `sqlite3_compileoption_used("ENABLE_MEMORY_MANAGEMENT") == 0` | `bundled_sqlite_keeps_a_page_cache_per_connection` (`tests/slice40_reader_page_cache.rs`) | I1, I2 |
| AC40-96 | After one reader runs a read larger than its cache budget, that reader's `CACHE_USED` is at most 1.10 × 2,048,000 bytes. Arithmetic, stated in the test comment: `cache_size = -2000` gives `nMax = 2,048,000 / (page + extra)` pages, and `CACHE_USED` counts each page at its full allocation (page, extra and `PgHdr1`), so a full cache reports about 2.6% over 2,048,000 bytes (the probe's 2,053 KiB); `mxPinned` lets about ten more pinned pages through. 1.10× covers both; today's build gives 4,378 KiB | `a_heavy_read_stays_inside_its_own_page_cache` | I2 |
| AC40-97 | After that read, every other reader's next pass over its own working set has zero `CACHE_MISS` delta (as its pass before the read did), and every pass returns the same ids | `other_readers_keep_their_page_caches_after_a_heavy_read` | I2 |
| AC40-98 | The vendored-tree digest is the same in `FATHOMDB-PATCH.md`, `pinned-override-rot.json` and the checker constant; the amalgamation digest is unchanged; the checker passes | `./scripts/check-pinned-override-rot.sh` and its fixture tests | I1 |
| AC40-99 | `rust.md`, `rust-sdk.md`, `pinned-override-rot-guard.md`, `FATHOMDB-PATCH.md` and the CHANGELOG describe the change and the published-crate reach (REQ40-I4, Q-I1 ruled (a)); no knob, no `cache_size` change | review plus `./scripts/agent-lint-docs.sh` and `./scripts/agent-lint-md.sh` | I3, I4 |
| AC40-100 | The report-only timing ran in a release build at the H3 commit and at the Unit I commit; its output is in the test plan | `reader_page_cache_timing_report` (`#[ignore]`) | I5 |
| AC40-101 | Latency harness: each cell is timed on its own engine open, closed before the next; summary v2 records the method, the open count and the cell order; receipt config version 2; a v2 summary is classified under plan `comp02b-latency-v2` and a v1 summary still under `comp02b-latency-v1`; the v2 plan file is committed with the first v2 receipt | `test_latency_harness_times_each_cell_on_its_own_open`, `test_latency_receipt_config_names_the_measurement_method`, `test_classification_reads_a_v2_latency_summary_under_its_own_plan` (`comp03-slice40`) | I7 |
| AC40-102 | COMP-02b: each cohort pass runs on its own open, closed before the next, and records its open index and the cohort order; the seeded query orders equal the single-open method's; `filtered-10000.v2.json` keeps v1's inputs and arms | `test_filtered_cell_times_each_cohort_on_its_own_open`, `test_filtered_cohort_orders_match_the_single_open_method`, `test_comp02b_v2_config_names_the_fresh_open_per_cohort_method` (`comp03-slice40`) | I7 |
| AC40-103 | Unit I gate (REQ40-I6), at the Unit I commit against the H3 commit | orchestrator measurement after the code review | I6 |
| AC40-104 | After a passed gate, the tc5-10k-latency and comp02b-10000 manifest entries take the Unit I values, the v2 commands and a method note; tolerances unchanged | orchestrator; `test_baseline_check.py` | I7 |
| AC40-105 | Each of the five raw-first test sites calls `configure_runtime(RuntimeSqliteMode::Performance)` before its first raw open; no assertion, fixture or name changes | review of the diff; the sweep table (test-plan § 3i.2) | J1 |
| AC40-106 | Parallel loops: after the guards, 0 of 30 runs fail for `pr_g8_dangling_edges`, `tc57_governed_write_race`, `pr_g3_read_collection` and `truncate_wal`, under default features (`operator` for `truncate_wal`; `tc57` needs `test-hooks`) and `test-hooks`; the before-counts are recorded (owner, 2026-10-16: 30 iterations) | the loop in test-plan § 3i.3 | J1, J5 |
| AC40-107 | The `TooLate` message is the REQ40-J2 text; the variant and the latch are unchanged | `cargo test -p fathomdb-engine` (the runtime-configuration tests pass unchanged); review | J2, J4 |
| AC40-108 | `config.md` and `rust.md` carry the REQ40-J3 sentence; `agent-lint-docs.sh` and `agent-lint-md.sh` pass | review plus the two linters | J3 |
| AC40-24 | The full gate passes | `cargo test -p fathomdb-engine`, `cargo test -p fathomdb-query`, `./scripts/agent-lint.sh`, `./scripts/agent-lint-md.sh`, then `./scripts/agent-verify.sh` on the final W5 tree | all |
