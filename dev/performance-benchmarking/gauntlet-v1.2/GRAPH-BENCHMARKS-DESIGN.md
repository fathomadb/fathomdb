# Graph benchmark implementation design

**Status:** approved after independent review (`FIX-1` through `FIX-3`)

**Requirements:** [Graph benchmark requirements](GRAPH-BENCHMARKS-REQUIREMENTS.md)

## Architecture

The implementation adds three experiment modules and keeps the top-level
gauntlet as an orchestrator:

```text
checked-in strict config
        |
        v
experiments.graph_*_01  -----> external raw observations/database
        |                                  |
        +---- safe metrics + record <-------+
        |
        v
thin scripts/perf-experiments adapter
        |
        v
gauntlet cell registry (optional graph selector)
```

`experiments/graph_benchmarks.py` owns shared strict-field validation,
canonical hashing, percentile calculation, denominator reconciliation, and
safe artifact identities. It contains no FathomDB calls. Each cell module owns
its domain oracle, live work, metrics, and receipt. Adapters only translate
closed arguments into the module CLI.

Every live runner creates its database through
`prepare_test_database(..., embedder="none", check_reranker=False)`. Retrieval
reuses precomputed GRAPH-01 passage embeddings/rankings; no CPU embedding is
introduced. Raw observations, databases, and corpus-derived records remain
under a new external root. Checked-in receipts contain safe aggregates and
hashes only.

## Common contracts

Each configuration has exact top-level keys:

- `schema_version`, `program_track`, `cell_id`, and `claim_boundary`;
- `measurement` with repetitions, warm-up, estimator, and denominators;
- cell-specific immutable fixture/workload/treatment identity; and
- `outputs` declaring separate quality/fidelity and performance sections.

Unknown or missing fields fail before an output directory is created. The
resolved canonical JSON digest is the workload/config identity. Result
validators require:

```text
attempted = completed + errors + typed_refusals
```

Raw observation manifests include SHA-256 and byte count. Safe receipts bind
the source Git identity, package/runtime identity, configuration digest,
fixture or corpus digest, and raw manifest digest.

The stable receipt identity used for standalone/gauntlet parity contains only
the record schema, cell/experiment, source commit, runtime hashes,
configuration digest, workload/corpus digest, and replay-stable input artifact
digests. It excludes run ID, timestamp, output paths, raw-manifest digest,
latency/resource metrics, and any other observation expected to differ across
executions.

## GRAPH-EVIDENCE-01

The cell reuses the public graph API and the installed evidence fixture shape.
Its checked-in fixture declares nodes, edges, provenance, canonical source,
matrix cells, expected semantic identities, and expected refusal classes.
The oracle validator identities are
`scripts/release/slice50-evidence-matrix.py` at SHA-256
`9ed23cea7246d01bf1ed32f58e2081d8cdcd11ce487ce2954b9c65190aba8c72`
and `scripts/release/smoke/frozen-evidence-python.py` at SHA-256
`2d544b4a1331b9f294bcdd2e6c715645482d99abf29880fd25010b26c5985258`.
Opaque `fdbgev1` values are observations, never oracles.

The evidence configuration freezes fixture identity
`graph-evidence-synthetic-v1`, all 12 seed/direction/depth matrix cells
(`explicit|query` × `outgoing|incoming|both` × `1|2`), and ordinary,
atomically actuated, and parallel-winner edges. It uses 5 untimed warm-ups, 20
measured success-path operations per matrix cell, 3 repetitions, and one
attempt per refusal class per repetition. The canonical source body is fixture
owned and its hash is written into the resolved fixture before the database is
opened.

For each repetition the runner:

1. creates and drains a fresh provenance-complete graph;
2. freezes a read context;
3. runs the registered seed/direction/depth matrix with evidence disabled and
   enabled;
4. resolves every target and terminal-edge reference;
5. compares resolved revisions, endpoints, kinds, source hashes, and bytes to
   the fixture manifest;
6. executes tamper, foreign-context/database, replay, supersession, and
   erasure cases; and
7. writes per-operation observations externally before computing safe totals
   and latency percentiles.

The semantic-identity digest excludes opaque reference bytes. Fidelity
failures never enter success-path latency distributions. Empty expansion and
evidence-disabled response equivalence are explicit acceptance probes.

## GRAPH-EXPAND-01

`bfs_oracle` operates on an immutable adjacency fixture and never imports
FathomDB. It performs breadth-first traversal in seed order, sorts eligible
edges and targets by stable logical identity, records shortest hop and
terminal direction, and deduplicates targets. It independently charges one
work unit for every raw directional incident-edge row loaded for every
seed/frontier state before edge-kind, liveness, node-eligibility, or
target-kind filtering. A physical edge examined from two seeds/frontier states
is charged twice. `W-1`, `W`, and `W+1` probes use this oracle-computed value,
never the SUT result. Property tests compare the oracle with small exhaustive
graphs containing cycles, parallel edges, filters, and repeated seeds. The
existing conformance fixture identity is
`ec43aec001ebfa2b6e3696559e31bfcb19b3c88f109e021ff3aa281a28563baf`.

The oracle implements the public ordering contract from
`dev/plans/0.8.25/features/slice-60/design.md` lines 496–553 exactly:

- frontier states sort by `(seed_ordinal, current_logical_id)`;
- incident edges sort by `(direction_rank, edge_kind, next_logical_id,
  edge_logical_id, write_cursor)`, where outgoing precedes incoming;
- competing target origins sort by `(hop_count, seed_ordinal,
  predecessor_logical_id, direction_rank, edge_kind, target_logical_id)`;
- explicit seed IDs are never returned as targets;
- targets deduplicate by logical ID using their least origin tuple; and
- `result_limit` is applied only after the complete bounded walk and target
  ordering.

Property tests include competing paths, parallel edges, repeated seeds,
self-loops, cycles, and a result-limit prefix.

Query-seed correctness is independent of native traversal. The generated
manifest owns these exact seed fixtures, whose tokens occur nowhere else:

| Seed | Query text | Expected ordered logical IDs |
| --- | --- | --- |
| Q1 | `graphseed-one` | `node-000001` |
| Q5 | `graphseed-five` | `node-000010` through `node-000014` |
| Q10 | `graphseed-ten` | `node-000020` through `node-000029` |

The generator writes those tokens into the named node bodies and records the
expanded ordered lists in the workload manifest. The runner first compares
native `result.seeds` to the pinned IDs; only then does the BFS oracle start
from those IDs. A self-consistent target set reached from different native
seeds is a fidelity failure.

The generator uses Python's local `random.Random` with algorithm identity
`graph-expand-powerlaw-v1` and seed `20260921`. It creates typed nodes plus
directed power-law-like edges, rejecting self-loops and duplicate logical edge
IDs until the exact edge count is reached, then writes a canonical adjacency
manifest and digest. The checked-in configuration freezes the initial local
cells:

- `smoke`: 1,000 nodes and 5,000 edges;
- `small`: 10,000 nodes and 50,000 edges;
- `medium`: 50,000 nodes and 1,000,000 edges; and
- `large`: 100,000 nodes and 5,000,000 edges.

Only `smoke` is part of zero-cost repository verification. Larger cells are
explicitly selected. Preflight may block a cell but may not resize it.

The registered v1 matrix is the following exact set. `E1`, `E4`, and `E16`
mean explicit seed counts; `Q1`, `Q5`, and `Q10` mean query-seed ranked limits.
`N`, `EK`, `TK`, and `EL` mean no filter, edge-kind, target-kind, and
eligibility filters. `C` and `F` mean current and frozen contexts. Boolean
columns use `0`/`1`.

| ID | Scale | Seed | Direction | Depth | Degree | Filter | Context | Explain | Evidence | Concurrency | Bound |
| --- | --- | --- | --- | ---: | --- | --- | --- | ---: | ---: | ---: | --- |
| base | smoke | E1 | outgoing | 1 | median | N | C | 0 | 0 | 1 | exact |
| seed-e4 | smoke | E4 | outgoing | 1 | median | N | C | 0 | 0 | 1 | exact |
| seed-e16 | smoke | E16 | outgoing | 1 | median | N | C | 0 | 0 | 1 | exact |
| seed-q1 | smoke | Q1 | outgoing | 1 | median | N | C | 0 | 0 | 1 | exact |
| seed-q5 | smoke | Q5 | outgoing | 1 | median | N | C | 0 | 0 | 1 | exact |
| seed-q10 | smoke | Q10 | outgoing | 1 | median | N | C | 0 | 0 | 1 | exact |
| direction-in | smoke | E1 | incoming | 1 | median | N | C | 0 | 0 | 1 | exact |
| direction-both | smoke | E1 | both | 1 | median | N | C | 0 | 0 | 1 | exact |
| depth-0 | smoke | E1 | outgoing | 0 | median | N | C | 0 | 0 | 1 | minimum-valid |
| depth-2 | smoke | E1 | outgoing | 2 | median | N | C | 0 | 0 | 1 | exact |
| degree-low | smoke | E1 | outgoing | 1 | low | N | C | 0 | 0 | 1 | exact |
| degree-high | smoke | E1 | outgoing | 1 | high | N | C | 0 | 0 | 1 | exact |
| filter-edge | smoke | E1 | outgoing | 1 | median | EK | C | 0 | 0 | 1 | exact |
| filter-target | smoke | E1 | outgoing | 1 | median | TK | C | 0 | 0 | 1 | exact |
| filter-eligibility | smoke | E1 | outgoing | 1 | median | EL | C | 0 | 0 | 1 | exact |
| context-frozen | smoke | E1 | outgoing | 1 | median | N | F | 0 | 0 | 1 | exact |
| explanation | smoke | E1 | outgoing | 1 | median | N | C | 1 | 0 | 1 | exact |
| evidence | smoke | E1 | outgoing | 1 | median | N | F | 0 | 1 | 1 | exact |
| concurrency-2 | smoke | E1 | outgoing | 1 | median | N | C | 0 | 0 | 2 | exact |
| concurrency-4 | smoke | E1 | outgoing | 1 | median | N | C | 0 | 0 | 4 | exact |
| concurrency-8 | smoke | E1 | outgoing | 1 | median | N | C | 0 | 0 | 8 | exact |
| bound-below | smoke | E1 | outgoing | 1 | median | N | C | 0 | 0 | 1 | W-1 |
| bound-exact | smoke | E1 | outgoing | 1 | median | N | C | 0 | 0 | 1 | W |
| bound-above | smoke | E1 | outgoing | 1 | median | N | C | 0 | 0 | 1 | W+1 |
| scale-small | small | E1 | outgoing | 1 | median | N | C | 0 | 0 | 1 | exact |
| scale-medium | medium | E1 | outgoing | 1 | median | N | C | 0 | 0 | 1 | exact |
| scale-large | large | E1 | outgoing | 1 | median | N | C | 0 | 0 | 1 | exact |

The bound fixture has oracle `W=3`, so its requests are valid budgets 2, 3,
and 4. Depth zero is a separate cell using the minimum valid request budget 1
and must report work 0. An oracle W of 1 has no below probe; W of 10,000 has no
above probe. Request values 0 and 10,001 are request-validation tests, not
traversal-bound refusals.

Every cell uses 5 untimed warm-up operations, 100 measured operations for
`smoke`, 1,000 for larger scales, and 3 repetitions. Closed-loop concurrency
is canonical; offered-rate cells are separate and disabled in v1.

Correctness runs before measurement. The runner verifies native target order,
hops, direction, deduplication, completeness, and work units against the
oracle for each timed operation. It separately probes exact work, one below,
and one above. Timed output uses the linear-interpolation percentile
implementation already used by `experiments.graph_01`: sort values, compute
`position = p * (n - 1)`, and linearly interpolate between the floor and
ceiling indices. It is not the nearest-rank estimator. The cell reports
returned targets and native work units; independently computed incident-edge
counts are labelled `oracle_edge_rows_estimate`, not native visit telemetry.
Latency and throughput remain separate rather than a blended score.

## GRAPH-RETRIEVAL-01

The MuSiQue adapter reuses GRAPH-01 loading, relation admission, projection,
paired bootstrap, and support labels. It does not repeat extraction. Because
the retained GRAPH-01 observations contain only top-10 control IDs, a separate
qualification command materializes the required canonical top-20 seed
manifest once. All scored arms then read exactly that manifest.

The historical input identities remain: MuSiQue development JSONL
`3cff37fd7221506a343a125cf7ca20aab7cd09877e376122da9627e1b935b26f`,
question-blind extractions
`7ca969520e7c79dab83a3cce0800f8e87369b3aed7b7264577f6959f397373d4`,
300-question cohort
`3f4061119a278045c6c905f5fc5db9705ef733a2b051541042a49012f14f2019`,
and reused GRAPH-01 configuration
`5fc5ca210fc2cbaa230d9102030b0206fd4ae66e436135289e1d01663b1989bf`.
The retrieval configuration fixes top-20 evaluation depth, top-10 context
depth, 2,000 paired bootstrap draws, seed `20260921`, 3 repetitions, and the
same linear-interpolation percentile estimator used by the other cells.

Top-20 materialization pins `BAAI/bge-small-en-v1.5` revision
`5c38ec7c405ec4b44b94cc5a9bb96e735b38267a`, CLS pooling, L2 normalization,
RRF `k=60`, and candidate depth 20. Model files are bound by the existing
digests: config
`094f8e891b932f2000c92cfc663bac4c62069f5d8af5b5278c4306aef3084750`,
tokenizer
`d241a60d5e8f04cc1b2b3e9ef7a4921b27bf526d9f6050ab90f9267a1f9e5c66`,
and weights
`3c9f31665447c8911517620762200d2245a2518d6e7208acc78cd9db317e21ad`.
Dense encoding must execute on a pinned RTX 3090 CUDA UUID; CPU or `auto`
fallback is a blocker. The accelerator implementation must reproduce the
retained top-10 control IDs for all 300 questions exactly before ranks 11–20
are qualified. The external seed manifest records its own digest, GPU UUID,
model identities, materializer source hash, question count, and parity count.
The live retrieval runner requires that manifest as an input and cannot invoke
the materializer.

Every arm preserves an ordered top-20 evaluation ranking. A separate top-10
context ranking is derived from its first ten entries. Native depth-one and
depth-two arms send entity anchors from the frozen projection to
`graph.expand`; reached artifacts become passage candidates only after exact
terminal-edge/source evidence resolves to a known paragraph. The candidate
policy is:

- protect control ranks one through eight;
- promote at most two new passages;
- restrict candidates to the frozen top-20 seed universe plus
  evidence-resolved graph candidates;
- deduplicate by paragraph ID;
- order promotions by shortest hop, seed rank, source paragraph ID; and
- return an ordered top-20 evaluation ranking where 20 are available and
  derive exactly ten context passages from it where ten are available.

The treatment records native request/result digests so an in-memory adjacency
result cannot masquerade as the measured arm. Scoring is offline from the
materialized rankings. Overall and hop-stratified metrics use the identical
question set; bootstrap uses 2,000 paired question draws and seed `20260921`.
The historical cohort is directional reuse and cannot be labelled official
MuSiQue.

STaRK is a separate adapter state. Until its full manifest qualifies, validate
and dry-run return a typed blocker and do not emit a score. Qualification
requires the complete unchanged knowledge base and split, complete mapping,
an exact official baseline configuration, and evaluator parity on identical
predictions within `1e-12` absolute tolerance.

## Gauntlet integration

The existing default ten cells are unchanged. Canonical order appends the
existing optional `ac013-scale-matrix`, then `graph-evidence01`,
`graph-expand01`, and `graph-retrieval01`. The closed configuration adds
nullable config keys `graph_evidence`, `graph_expand`, and `graph_retrieval`;
nullable asset groups `graph_output`, `musique`, and `stark`; and timeouts for
enabled graph cells. `graph_output` contains `external_output_root`.
`musique` contains `dataset`, `extractions`, `cohort`, and the qualified
`seed_manifest`. `stark` contains `checkout`, `dataset_root`, `manifest`, and
`evaluator`.

`--suite graph` is mutually exclusive with `--cells` and resolves exactly the
three graph cells in canonical order. It may select only graph cells already
enabled by the loaded configuration; it never expands configuration scope.
With neither option, the current ten-cell default is unchanged. Each plan
invokes the corresponding thin adapter with absolute paths for config,
source/runtime, artifact root, receipt root, and result path. The orchestrator
records each cell independently and continues according to its existing
failure-isolation policy.

The exact repository-local commands frozen for Slice 90 are:

```bash
PYTHONPATH=src/python .venv/bin/python -m pytest \
  tests/experiments/test_graph_evidence_01.py \
  tests/experiments/test_graph_expand_01.py \
  tests/experiments/test_graph_retrieval_01.py \
  tests/experiments/test_perf_gauntlet_graph_cells.py -q

PYTHONPATH=src/python .venv/bin/python -m experiments.graph_evidence_01 \
  smoke experiments/configs/graph-evidence-01/exact-evidence.v1.json \
  /tmp/fathomdb-graph-standalone-smoke/evidence

PYTHONPATH=src/python .venv/bin/python -m experiments.graph_expand_01 \
  smoke experiments/configs/graph-expand-01/bounded-traversal.v1.json \
  /tmp/fathomdb-graph-standalone-smoke/expand

PYTHONPATH=src/python .venv/bin/python -m experiments.graph_retrieval_01 \
  smoke experiments/configs/graph-retrieval-01/musique-native-expand.v1.json \
  /tmp/fathomdb-graph-standalone-smoke/retrieval

PYTHONPATH=src/python .venv/bin/python \
  scripts/perf-experiments/run-graph-suite-smoke.py \
  --release 0.8.26 \
  --source-root /home/coreyt/projects/fathomdb \
  --output-root /tmp/fathomdb-graph-gauntlet-smoke

PYTHONPATH=src/python .venv/bin/python \
  scripts/perf-experiments/compare-graph-receipts.py \
  --standalone-root /tmp/fathomdb-graph-standalone-smoke \
  --gauntlet-root /tmp/fathomdb-graph-gauntlet-smoke

./scripts/agent-verify.sh
```

`run-graph-suite-smoke.py` writes a runtime-specific strict gauntlet
configuration beneath its new output root and invokes
`run_gauntlet.py --suite graph`; it does not contain workload logic.
`compare-graph-receipts.py` compares only the stable identity projection
defined above, never full record bytes.

## Failure and blocker states

- `invalid_contract`: configuration, manifest, receipt, or digest drift;
- `blocked_prerequisite`: required external corpus or official evaluator is
  absent or unqualified;
- `blocked_resource`: an unchanged qualified scale cannot fit the declared
  disk/RAM envelope;
- `fidelity_failed`: native results disagree with the independent oracle;
- `complete`: the requested cell completed and reconciled denominators.

Failures are recorded without manufacturing zero scores. Paid models, corpus
acquisition, GPU embedding, and external publication remain separately
authorized operations.
