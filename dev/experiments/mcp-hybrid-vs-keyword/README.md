# MCP search: hybrid versus text on LoCoMo (MR-021)

This is a retrieval-only evaluation run through the `fathomdb-mcp` server. It
cost $0: no LLM judge, no priced API, and no network beyond the package
registries. It answers the question MR-021 asks
(`dev/design/mcp-adapter/requirements.md`): does the hybrid search path reach
or exceed plain BM25 (`mode: "text"`) on precision@3? The config default is
`search_mode: "text"` until an evaluation shows that it does.

**Verdict: yes.** On LoCoMo, hybrid precision@3 is 0.2066 and text precision@3
is 0.1860, a paired difference of +0.0206 (95% CI [+0.0141, +0.0269]). Hybrid
also beats text on gold coverage@5 and @10, hit@5 and @10, MRR@10 and
nDCG@10, with every confidence interval above zero. Hit@1 is +0.0163, and its
interval [-0.0007, +0.0332] just includes zero. Scored with the SDK LoCoMo
runner's definitions, the MCP text path matches the SDK FTS baseline (see
[Comparison with the SDK LoCoMo baseline](#comparison-with-the-sdk-locomo-baseline)). **Recommendation:** under MR-021.2, the evidence allows
`search_mode: "hybrid"` as the default for stores opened with the default
embedder. This experiment does not change the config default. The adopting
ADR should cite these numbers and the caveats below.

## Dataset and mapping

- **Source:** LoCoMo, `data/corpus-data/raw/locomo10.json` in the main
  checkout. It is gitignored and was read in place, and none of it is committed
  here. The results identify each question by `<conversation>:<qa index>` and
  contain no question or turn text.
- **Documents:** one `note` node per dialog turn, 5,882 turns across 10
  conversations (419, 369, 663, 629, 680, 675, 689, 681, 509 and 568 turns).
  - `logical_id` is `<conversation index>:<dia_id>`, for example `0:D1:3`. The
    MCP search tool returns it as the ref `l:0:D1:3`.
  - The body is `[<session date>] <speaker>: <text>`. The session date comes
    from the conversation's `session_<n>_date_time`.
  - When a turn shares an image, its `blip_caption` is appended after a space as
    `[shared image: <caption>]`. Some answers live only in the caption, and an
    agent's memory would hold that caption, so it is included. This is an
    addition to the task's body shape.
  - `source_id` is `locomo-conv<i>-session<n>`.
- **Queries:** every QA item with at least one evidence id, excluding
  category 5.

  | | Count |
  | --- | --- |
  | QA items | 1,986 |
  | Excluded: category 5 (adversarial) | 446 |
  | Excluded: empty evidence (category 3) | 4 |
  | Excluded: no evidence id maps to a turn | 0 |
  | **Scored** | **1,536** |

  Category 5 in this file *does* carry evidence ids, so it was not dropped
  automatically. It was excluded because its questions are built on a false
  premise and have no correct answer to retrieve. The scored categories are
  multi-hop (1) with 282 queries, temporal (2) with 321, open-domain (3) with
  92, and single-hop (4) with 841. These names follow the mapping most
  published LoCoMo evaluations use; the numbers are what the data file
  carries.
- **Gold:** a query's relevant documents are its evidence turns. Raw evidence
  entries are normalized (`examples/locomo_eval/dataset.rs`, unit-tested):
  - entries holding several ids (`"D8:6; D9:17"`, `"D9:1 D4:4 D4:6"`) are
    split;
  - `D:11:26` is repaired to `D11:26`;
  - the zero-padded `D30:05` is read as `D30:5`.

  Three ids still do not resolve to a turn in their conversation and were
  dropped: `3:D10:19`, `3:D` and `6:D4:36`. Each of those queries kept other
  gold, so no query lost all of its gold. Gold sizes run from 1 to 19 turns;
  1,123 of the 1,536 queries have exactly one gold turn. **The precision@3
  ceiling is 0.4629**, the mean of min(|gold|, 3) / 3. Absolute precision@3
  values are low for that reason.
- **Fallback:** not needed. The LoCoMo evidence maps cleanly, so BEIR
  NFCorpus was not used.

## Store, projection and scoping

- **One store per conversation:** `conv<i>.sqlite`. A question is about one
  conversation, so each search goes to that conversation's store through the
  `store` argument. The server config lists one `roots` entry, the stores
  directory, which makes all ten stores addressable by name.
  - Scoping by store is the only clean option the MCP `search` tool offers.
    Its `kind` and `attributes` filters are hybrid-only, and `mode: "text"`
    rejects them, so filter-based scoping would not compare like with like.
  - The server keeps one open store at a time (`max_open_stores: 1`), so the
    evaluator runs the queries grouped by conversation.
- **Loading:** through `fathomdb-sdk`, because the MCP server is read-only.
  `examples/locomo_load.rs` performs these steps:
  1. Opens with `OpenOptions { use_default_embedder: true }`.
  2. Declares one projection, `{name: "turn", roles: [searchable], fts: {},
     vector: {}}`. This enrols the `note` kind for the dense arm.
     `vector_unsupported_kinds` must be empty.
  3. Writes the turns in batches of 256.
  4. Calls `drain(600 s)`.
- **Embedding completeness** was verified per store before searching, and the
  loader exits 1 on any shortfall:
  - `read::embedding_readiness` reported `state = Ready`,
    `usable_embedder = true` and `pending_count = 0`;
  - `read::projections` reported `dense_readiness = Ready`;
  - a read-only SQLite count after close found a `_fathomdb_vector_rows` row
    for every node: 5,882 of 5,882, and 100% in every store
    (`results/load.jsonl`).

  At evaluation time, the server's own `orient` reported `state: ready`,
  `usable_embedder: true` and `pending_count: 0` for all ten stores
  (`summary.json` → `orient_embedding_readiness`).

## Embedder, device and GPU evidence

- **Embedder:** the pinned default, `bge-small-en-v1.5` (384-dim, Candle).
  The corpus and the queries were embedded by the same CUDA build.
- **Build:** `fathomdb-mcp` gained an `embed-cuda` feature,
  `["default-embedder", "fathomdb-sdk/embed-cuda"]`. It forwards only to
  `fathomdb-sdk`, as `tests/manifest.rs` requires. Both the loader and the
  server binary were built with it.
- **Device selection:**
  - `CUDA_VISIBLE_DEVICES=0` hides the second 3090 and the Quadro K620 display
    GPU.
  - `FATHOMDB_EMBED_DEVICE=cuda:0` forces CUDA. A forced `cuda:N` never falls
    back to CPU: if CUDA cannot be used, the open fails
    (`fathomdb-embedder/src/device_policy.rs`). A silent CPU run therefore
    cannot happen.
- **Evidence** (`results/gpu.json`, `nvidia-smi` sampled every 250 ms):

  | Phase | RTX 3090 #0 | Notes |
  | --- | --- | --- |
  | Corpus embedding (`locomo_load`, 5,882 turns, 39.8 s) | up to 48% utilization, 35% mean while busy; memory 3 → 480 MiB | about 150 turns/s |
  | Query embedding inside `fathomdb-mcp` (first run) | up to 68% utilization; memory 3 → 544 MiB | |
  | `nvidia-smi --query-compute-apps` (second run) | process `fathomdb-mcp` on 3090 #0, 298–436 MiB | |

  The second 3090 stayed at 0% utilization and 2 MiB throughout. The K620 ran
  only its usual desktop load.

## Commands

All commands run from the `exp/mcp-eval` worktree. `$S` is a scratch
directory; the stores (30 MB) and raw run directories live there.

```bash
# CUDA build (toolkit-only env, nothing system-wide changed)
env PATH=/usr/local/cuda-12.6/bin:$PATH CUDA_PATH=/usr/local/cuda-12.6 \
  LIBRARY_PATH=/usr/local/cuda-12.6/targets/x86_64-linux/lib CUDA_COMPUTE_CAP=86 \
  CC=/usr/bin/gcc-13 CXX=/usr/bin/g++-13 CARGO_TARGET_DIR=$PWD/target \
  cargo build --release -p fathomdb-mcp --features embed-cuda \
  --bin fathomdb-mcp --example locomo_load
cp target/release/fathomdb-mcp $S/fathomdb-mcp-cuda
# The evaluator itself needs no CUDA (it only speaks JSON-RPC to the server)
CARGO_TARGET_DIR=$PWD/target cargo build --release -p fathomdb-mcp --example locomo_eval

# Load + embed on the GPU
env CUDA_VISIBLE_DEVICES=0 FATHOMDB_EMBED_DEVICE=cuda:0 \
  target/release/examples/locomo_load <main-checkout>/data/corpus-data/raw/locomo10.json $S/stores

# Evaluate through MCP stdio (the server inherits the device env)
env CUDA_VISIBLE_DEVICES=0 FATHOMDB_EMBED_DEVICE=cuda:0 \
  target/release/examples/locomo_eval <main-checkout>/data/corpus-data/raw/locomo10.json \
  $S/stores $S/fathomdb-mcp-cuda $S/eval-final
```

The evaluator writes this server config:

```json
{"roots": [{"path": "<stores>", "suffix": ".sqlite"}], "use_default_embedder": true,
 "default_max_tokens": 200000, "call_deadline_ms": 120000, "idle_timeout_secs": 0,
 "snippet_bytes": 40}
```

The recorded runs used the earlier key `snippet_chars: 40` (a 40-character
prefix). EP-17 renamed it to `snippet_bytes` with no alias, and snippets are
now selected around the match, so a rerun is not byte-comparable with them.

It sends one `search` call per mode per question, with the arguments
`{"store": "conv<i>", "query": <question>, "mode": "text" | "hybrid",
"limit": 10}`. Hybrid runs the server's default path: `Engine::search` with SDK
defaults (RRF fusion, `rerank_depth` 0, no graph arm).

**Determinism:** three full runs returned identical ranked refs for all 1,536
queries in both modes, and identical summaries apart from latency.

## Metrics

Metrics are computed per query from the refs the server returned, in rank
order, and averaged over queries. Definitions
(`examples/locomo_eval/metrics.rs`, unit-tested against hand-computed cases):

- **precision@3:** relevant refs in the top 3, divided by 3. The divisor is 3
  even when fewer results came back. Ceiling 0.4629 on this query set.
- **gold coverage@k** (fraction of gold recalled): relevant refs in the top k,
  divided by |gold|. A query with three gold turns that finds one scores 0.33.
  The evaluator and the result files call this `recall@k`. It is **not** the
  `R@k` the SDK LoCoMo runner reports; that is hit@k below.
- **hit@k:** 1 when at least one relevant ref is in the top k, else 0. This is
  what `experiments/locomo_metrics.py` reports as `R@k` / `r_at_k`.
- **MRR@10:** the mean of 1 / (rank of the first relevant ref in the top 10),
  or 0 when there is none. Same definition as the SDK runner.
- **nDCG@10:** binary relevance per returned ref, discounted by log2(rank + 1),
  divided by the ideal DCG of min(|gold|, 10) relevant refs. Same definition as
  the SDK runner.

Gold coverage, precision@3, MRR@10 and hit@1 come from the Rust evaluator.
hit@5, hit@10 and nDCG@10 were computed afterwards from `results/per_query.jsonl`
with a Python script using the SDK runner's definitions, with a paired
percentile bootstrap (10,000 resamples, seed 20261017). That is the same
procedure, but a different random generator from the evaluator's.

### Text versus hybrid (each cell is text / hybrid)

| Slice | n | precision@3 | coverage@5 | coverage@10 | MRR@10 | hit@1 |
| --- | --- | --- | --- | --- | --- | --- |
| **overall** | 1,536 | 0.1860 / **0.2066** | 0.5336 / **0.5858** | 0.6121 / **0.6552** | 0.4603 / **0.4937** | 0.3613 / **0.3776** |
| 1 multi-hop | 282 | 0.1489 / 0.2139 | 0.2243 / 0.3089 | 0.3154 / 0.3909 | 0.3001 / 0.3890 | 0.1950 / 0.2482 |
| 2 temporal | 321 | 0.2118 / 0.2253 | 0.6436 / 0.6794 | 0.7048 / 0.7344 | 0.5539 / 0.5695 | 0.4735 / 0.4673 |
| 3 open-domain | 92 | 0.0978 / 0.1087 | 0.2552 / 0.2911 | 0.3065 / 0.3262 | 0.2263 / 0.2586 | 0.1304 / 0.1630 |
| 4 single-hop | 841 | 0.1982 / 0.2077 | 0.6258 / 0.6752 | 0.7097 / 0.7495 | 0.5038 / 0.5257 | 0.3995 / 0.4102 |

### Paired hybrid minus text, with a 95% bootstrap CI

The CI is a paired percentile bootstrap over queries: 10,000 resamples, seed
20261017.

| Slice | precision@3 | coverage@5 | coverage@10 | MRR@10 | hit@1 |
| --- | --- | --- | --- | --- | --- |
| **overall** | **+0.0206 [+0.0141, +0.0269]** | +0.0522 [+0.0382, +0.0661] | +0.0430 [+0.0322, +0.0541] | +0.0335 [+0.0215, +0.0452] | +0.0163 [-0.0007, +0.0332] |
| 1 multi-hop | +0.0650 [+0.0449, +0.0851] | +0.0845 [+0.0600, +0.1106] | +0.0755 [+0.0525, +0.0994] | +0.0889 [+0.0610, +0.1173] | +0.0532 [+0.0142, +0.0922] |
| 2 temporal | +0.0135 [+0.0021, +0.0249] | +0.0358 [+0.0067, +0.0670] | +0.0296 [+0.0104, +0.0504] | +0.0156 [-0.0060, +0.0368] | -0.0062 [-0.0343, +0.0218] |
| 3 open-domain | +0.0109 [-0.0036, +0.0290] | +0.0360 [+0.0027, +0.0761] | +0.0197 [-0.0084, +0.0523] | +0.0323 [-0.0061, +0.0738] | +0.0326 [-0.0326, +0.0978] |
| 4 single-hop | +0.0095 [+0.0016, +0.0174] | +0.0493 [+0.0285, +0.0702] | +0.0398 [+0.0238, +0.0565] | +0.0218 [+0.0047, +0.0392] | +0.0107 [-0.0143, +0.0357] |

Per-query wins and losses on precision@3: hybrid is better on 147 queries,
text on 58, and 1,331 tie. For MRR@10 the split is 349, 180 and 1,007. The
gain is largest on multi-hop questions, where vocabulary mismatch between the
question and the turns is most common. Single-hop and temporal gains are
small but positive.

### hit@k and nDCG@10 (SDK runner definitions)

Each cell is text / hybrid, then hybrid minus text with its 95% CI.

| Slice | n | hit@5 | hit@10 | nDCG@10 |
| --- | --- | --- | --- | --- |
| **overall** | 1,536 | 0.5938 / **0.6530**; +0.0592 [+0.0430, +0.0755] | 0.6777 / **0.7285**; +0.0508 [+0.0378, +0.0638] | 0.4792 / **0.5131**; +0.0338 [+0.0245, +0.0428] |
| 1 multi-hop | 282 | 0.4397 / 0.5745; +0.1348 [+0.0887, +0.1844] | 0.5674 / 0.6844; +0.1170 [+0.0816, +0.1560] | 0.2432 / 0.3182; +0.0749 [+0.0572, +0.0932] |
| 2 temporal | 321 | 0.6760 / 0.7072; +0.0312 [+0.0000, +0.0623] | 0.7321 / 0.7632; +0.0312 [+0.0125, +0.0530] | 0.5815 / 0.6006; +0.0191 [+0.0023, +0.0362] |
| 3 open-domain | 92 | 0.3587 / 0.3913; +0.0326 [-0.0109, +0.0870] | 0.4130 / 0.4348; +0.0217 [-0.0326, +0.0761] | 0.2209 / 0.2426; +0.0217 [-0.0056, +0.0491] |
| 4 single-hop | 841 | 0.6397 / 0.6873; +0.0476 [+0.0262, +0.0690] | 0.7229 / 0.7622; +0.0392 [+0.0238, +0.0559] | 0.5476 / 0.5746; +0.0270 [+0.0132, +0.0409] |

## Comparison with the SDK LoCoMo baseline

The SDK-based LoCoMo runner's turn-level FTS baseline
(`experiments/runs/locomo-capability-a0-baseline-20260814T2311Z-d4a71071`)
scores the same 1,536 questions: categories 1–4, with the 4 empty-evidence
items excluded. It indexes one turn per document, uses FTS only, and keeps the
top 10. Under its definitions:

| Metric (SDK name) | SDK FTS baseline | MCP text | MCP hybrid |
| --- | --- | --- | --- |
| hit@1 (`R@1`) | 0.3438 | 0.3613 | 0.3776 |
| hit@5 (`R@5`) | 0.5931 | 0.5938 | 0.6530 |
| hit@10 (`R@10`) | 0.6673 | 0.6777 | 0.7285 |
| MRR | 0.4463 | 0.4603 | 0.4937 |
| nDCG@10 | 0.4653 | 0.4792 | 0.5131 |

The MCP text path matches the SDK FTS baseline: it is 0.1 to 1.8 points higher
on every metric, so nothing is lost by going through MCP. A plausible but
unverified reason for the small edge is that this loader adds the session date
and the image caption to each turn's body. The earlier draft of this README
read as worse than the SDK runs for one reason: it labelled gold coverage as
"recall", and the SDK runner's `R@k` is hit@k.

Not comparable: the 0.752 in
`experiments/runs/fathomdb-vs-mem0-locomo-comparison-20260824T2140Z-01e702be`
(Mem0 0.672). That number is end-to-end answer accuracy. gpt-4o-mini answers
from the top 10 hits and an LLM judge grades the answer. It is not a retrieval
metric, and a correct answer does not require a gold turn in the context.

### Health of the run

- **Text returned 0 results:** 0 of 1,536 queries. Four text searches
  returned fewer than 10 results (`completeness: complete`), and the other
  1,532 were `limit_reached`.
- **Hybrid degradation:** none. No query reported a degradation entry in
  either mode: no `soft_fallback`, `dense_disabled` or `embedder_unavailable`.
  All 1,536 hybrid results were `limit_reached`. The vector arm demonstrably
  ran: of the 15,360 hybrid hits, 6,859 came from the `vector` arm and 8,501
  from `text`.
- **No tool errors.** No result was cut by the token budget.
- **Latency through MCP stdio** (informational, GPU query embedding): text
  p50 1.2 ms and p95 2.6 ms; hybrid p50 7.0 ms and p95 13.5 ms.

Raw per-query results are in `results/per_query.jsonl` (1.3 MB). Each line
carries the refs in rank order, the arms, completeness, degradation and the
per-query metrics. `results/summary.json` holds the evaluator's tables; its
`recall@k` keys are gold coverage. The hit@k and nDCG@10 tables are not in it;
they were recomputed from `per_query.jsonl` as described under Metrics. The other
two result files are `results/load.jsonl` (completeness per store) and
`results/gpu.json`.

## Comparison with the prior EARP finding

The EARP campaign-3 tiered runs (2026-08-09 to 08-11) were run on a different,
real corpus with 4,472 scoreable queries. They found that hybrid (dense) search
earned nothing over FTS-only: exact-fact results were byte-identical, and
exploratory results were flat to worse. **This result does not agree.** On
LoCoMo, hybrid is a small but statistically clear improvement on every
rank-quality metric. Plausible reasons:

- LoCoMo questions are paraphrases of conversational turns ("What fields would
  Caroline be likely to pursue…"). The question wording often shares few terms
  with the evidence turn, which is where dense retrieval helps. The multi-hop
  slice shows the largest gain.
- LoCoMo documents are single short turns, so BM25 has little text to match
  against.
- EARP's gold and query mix were dominated by exact-fact lookups, where FTS is
  already at its ceiling.

The two results do not contradict each other. Hybrid's benefit depends on the
corpus, so the default should be justified per target workload. Agent-memory
stores of conversational turns are the case MR-021's "designated agent-memory
corpus" describes.

## Caveats

- **One dataset, ten conversations.** The queries are clustered by
  conversation, and the bootstrap resamples queries, not conversations, so
  the CI understates between-conversation variance. Against that, the
  precision@3 gain is positive in all ten conversations. It ranges from
  +0.0034 (conv3) to +0.0400 (conv6). Of the 205 queries where precision@3
  differs, hybrid wins 147 (72%).
- **The embedder is bge-small-en-v1.5 only.** A stronger embedder could widen
  the gap. No reranker and no graph arm were used.
- **LoCoMo turns are short, and most queries have a single gold turn.** That
  caps precision@3 at 0.463, so the absolute numbers are low by construction.
  The paired difference is the meaningful quantity.
- **The questions are LoCoMo's, not an agent's.** Real agent queries may be
  keyword-like, which favours text.
- **The session date is prefixed to every body.** This adds the same date
  tokens to every turn of a session, for both modes. The hybrid RRF fusion
  includes recency reweighting. All turns were written in one session, so
  recency carries no real signal here.
- **Category names** follow the common LoCoMo mapping. The numeric categories
  in `summary.json` are what the data file carries.
- **MR-021.1 asks for "the adapter's default search path and plain BM25".**
  The text arm here is the adapter's current default, `search_text_only`,
  which is plain FTS5 BM25. The hybrid arm is the candidate default.
