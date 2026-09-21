# Graph benchmark requirements and acceptance criteria

**Status:** frozen for implementation after design approval

**Scope:** `GRAPH-EVIDENCE-01`, `GRAPH-EXPAND-01`, and
`GRAPH-RETRIEVAL-01` as optional Performance Gauntlet v1.2 cells.

These requirements define runnable, directional benchmarks. They do not claim
official LDBC, LinkBench, Graphalytics, MuSiQue, or STaRK compliance.

## Shared requirements

| ID | Requirement | Executable acceptance |
| --- | --- | --- |
| GB-01 | Every cell has a strict, versioned JSON configuration with `program_track`, immutable workload identity, repetitions, warm-up, denominators, and estimator identity. | `test_graph_*_01.py` rejects missing and unknown fields and accepts each checked-in configuration. |
| GB-02 | Every live cell uses `prepare_test_database` and a new external artifact root. | Runner tests inject the preparation seam and reject an existing or in-repository artifact root. |
| GB-03 | Raw observations remain external; safe aggregates use `experiments.record.v1` and one append-only index row. | Runner tests inspect the receipt and prove repeated registration is idempotent. |
| GB-04 | Fidelity/retrieval metrics and latency/resource metrics remain separate objects and summary sections. | Schema tests reject blended headline fields and assert both sections exist. |
| GB-05 | Attempted, completed, error, and typed-refusal counts are explicit. | Result-schema tests require the four denominators and check they reconcile. |
| GB-06 | Comparable runs bind source, configuration, corpus/generator, runtime, and raw-observation digests. | Receipt-forgery tests alter each digest and require validation failure. |
| GB-07 | Standalone and gauntlet execution resolve the same stable receipt identity: schema, cell, source/runtime, configuration, workload/corpus, and replay-stable input artifact digests. Run ID, timestamp, path, raw-manifest, latency, and resource observations are excluded. | `test_perf_gauntlet_graph_cells.py` compares the planned invocation and stable receipt-identity projection. |

## GRAPH-EVIDENCE-01

| ID | Requirement | Executable acceptance |
| --- | --- | --- |
| GE-01 | The fixture, not FathomDB output, owns expected target revision, winning terminal-edge revision/endpoints/kind, source bytes/hash, and refusal class. | A scorer test supplies a self-consistent but fixture-wrong SUT result and requires failure. |
| GE-02 | The matrix covers explicit/query seeds, outgoing/incoming/both, depths one/two, ordinary/actuated edges, and a deterministic parallel-edge winner. | Configuration validation requires every registered matrix cell and fixture class. |
| GE-03 | Every positive target and terminal-edge reference resolves to its exact fixture-owned semantic identity and canonical source. | Live acceptance tests require 100% target, terminal-edge, source-hash, and source-byte exactness. |
| GE-04 | Opaque reference bytes are not compared for equality; resolved semantic identity is deterministic across repetitions. | Repeated observations with distinct valid tokens but identical resolution pass; semantic drift fails. |
| GE-05 | Foreign database/context, cross-request replay, bit tamper, supersession, and erasure fail nondisclosingly. | Negative tests require the registered refusal class and zero stale/post-erasure/unauthorized acceptance. |
| GE-06 | `include_evidence=false` preserves the ordinary response and empty results perform no evidence hydration. | Live tests compare ordinary response shapes and assert zero resolver calls for empty results. |
| GE-07 | Timing reports evidence-off, sidecar-only, and sidecar-plus-resolution separately. | Metrics tests require p50/p95/p99 plus response/reference byte counts for each success-path arm. |

## GRAPH-EXPAND-01

| ID | Requirement | Executable acceptance |
| --- | --- | --- |
| GX-01 | A source-independent BFS oracle computes bounded reachability, shortest hop, deterministic order, terminal direction, deduplication, and work units. One work unit is each raw directional incident-edge row loaded for each seed/frontier state before edge-kind, liveness, node-eligibility, or target-kind filtering; the same physical row is charged again if examined from another seed/frontier state. Query-seed cells pin the query text and independently expected ordered seed IDs before traversal. | Property tests cover cycles, parallel edges, repeated seeds, direction, filters, depth zero through two, exact work accounting, and rejection of a self-consistent traversal from wrong query seeds. |
| GX-02 | The deterministic generator binds seed, version, node/edge counts, topology, adjacency digest, and registered cells. | Same inputs produce byte-identical manifests; seed or generator drift changes the digest. |
| GX-03 | Canonical release comparisons use one frozen manifest. They never silently shrink a scale; unavailable scales report `blocked_resource`. | Manifest tests reject changed scales/cells and exercise the typed blocker. |
| GX-04 | The canonical matrix uses depths zero, one, and two. Depth three is a separately identified, non-comparable legacy diagnostic. | Configuration tests reject depth three in canonical cells. |
| GX-05 | Native results exactly match the oracle and succeed at the required work bound. For a registered fixture with oracle W in 2–9,999, W−1 refuses atomically while W and W+1 succeed; depth zero and API endpoints use valid request bounds. | Live correctness tests compare IDs/order/hops, the typed bound refusal, depth-zero work 0 with request budget 1, and request validation at 0/10,001. |
| GX-06 | Performance results pin open/closed-loop model, offered rate or concurrency, warm-up, operation count/duration, repetitions, and percentile estimator. | Strict configuration and metrics tests require the complete identity. |
| GX-07 | Timed cells retain correctness assertions and report latency, throughput, returned-target count, native work units, oracle edge-row estimates, bytes, errors/refusals, CPU, RSS, and storage. They do not claim unavailable native node/edge visit telemetry. | Performance-schema tests reject missing fields and mismatched denominators. |

## GRAPH-RETRIEVAL-01

| ID | Requirement | Executable acceptance |
| --- | --- | --- |
| GR-01 | The historical 300-question MuSiQue cohort and official-dev cohort have distinct identities and are never pooled. | Configuration and scorer tests reject mixed cohort identifiers. |
| GR-02 | Initial top-20 ranked passage seeds are materialized once per question in a hash-bound manifest consumed unchanged by every arm. The historical cohort materialization uses the pinned BGE revision with CLS pooling on a pinned RTX 3090, refuses CPU fallback, and must reproduce all retained top-10 rankings exactly. | Tests alter a question, rank, ID, device/model/materializer identity, top-10 parity result, or digest and require pre-scoring failure. |
| GR-03 | Arms are the frozen fused control, native depth one, and native depth two; graph treatments read native FathomDB traversal results. | Adapter tests inject native results and reject an in-memory-only treatment result. |
| GR-04 | Candidate promotion freezes protected prefix, promotion limit, evidence-to-passage mapping, deduplication, tie-break, and fixed passage budget. | Hand-computed tests prove useful addition, harmful displacement, no-op, and deterministic tie behavior. |
| GR-05 | Metrics include complete support, recall, precision, first-support MRR, nDCG where defined, candidate change, useful/harmful/no-op rates, and paired bootstrap deltas overall and by hop count. | Scorer tests use fixed qrels with hand-computed expectations and a pinned bootstrap seed/draw count. |
| GR-06 | Answer generation is optional and cannot block retrieval completion or change the retrieval verdict. | Configuration and runner tests complete with answer generation disabled and no credentials. |
| GR-07 | STaRK scoring is blocked unless repository, dataset/split, evaluator, baseline, counts/hashes, complete mappings, and numeric parity tolerance are pinned. | Qualification tests reject each absent/drifted identity and sampled data. |

## Gauntlet and verification requirements

| ID | Requirement | Executable acceptance |
| --- | --- | --- |
| GI-01 | The graph cells are optional and do not change the existing ten-cell default. | Gauntlet tests assert the default tuple is unchanged and the graph selector adds exactly three cells. |
| GI-02 | A graph-cell failure is isolated and cannot erase prior cell receipts. | Orchestration test forces one failure and retains earlier status/receipt entries. |
| GI-03 | Dry-run prints exact, closed invocations with configuration and output identities. | Plan-shape tests assert no implicit shell expansion and exact ordered arguments. |
| GV-01 | Focused tests, drift/forgery negatives, standalone/gauntlet parity, a no-paid-model smoke, and `./scripts/agent-verify.sh` are final gates. | Slice 90 executes the commands frozen in the design; any failure remains a blocker with its original diagnostic. |

## Qualification state

- The existing 0.8.26 installed evidence profile and local synthetic graph
  fixtures qualify the zero-network evidence and expansion correctness tiers.
- MuSiQue historical inputs retain the hashes in the GRAPH-01 contract. The
  official-dev identity remains a separate qualification task.
- Full Graphalytics, LinkBench/LDBC, and STaRK inputs are not vendored. Their
  external-comparison cells must return `blocked_prerequisite` or
  `blocked_resource` until their manifests are fully pinned; this does not
  block the synthetic correctness, local generated-scale, or historical
  MuSiQue adapters.
