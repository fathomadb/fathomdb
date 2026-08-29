# Overall performance benchmarking and agent-memory experiments program

This is the portfolio overview for FathomDB's performance benchmarking and
experiments program. It prioritizes tracks, records dependencies, and names the
decision each track may inform. Each linked track file is its short plan.
Execution details belong in configuration and receipts, not here.

The intended product outcome is stated in
[the program goals](PROGRAM-GOALS.md): an agent-memory database with one
provenance-preserving canonical record and selectively materialized retrieval
projections. This program tests whether a projection or retrieval treatment
earns use for a named query shape; it does not assume that vectors, chunking,
graphs, or an external comparator are product defaults.

## Track names and execution status

Track identifiers use a readable `WORD-01` form. The word names the capability
or corpus; the number distinguishes its step. An identifier is an anchor, not
a substitute for the track's decision question.

- **Active** means the named plan may execute after its stated preconditions.
- **Planned** means the program has authorized planning, but a dated
  measurement contract is required before execution.
- **Parked** means neither implementation nor paid work may start until the
  listed diagnostic or external prerequisite is satisfied.
- **Complete historical** means retained evidence; it does not authorize a
  current product claim.

## System of record

Three layers have distinct responsibilities:

| Layer | Answers | Source of truth |
| --- | --- | --- |
| Program planning | What should run next, why, and after which prerequisites? | This document and its track plans |
| Coordination progress | Which lanes are active, reviewed, blocked, or next? | [Track Runner status](TRACK-RUNNER-STATUS.md), updated only by the coordinator |
| Execution evidence | What actually ran, with which configuration, and what did it measure? | [`../../experiments/`](../../experiments/README.md) — append-only `index.jsonl` and per-run receipts |
| Decision record | What does the evidence establish or rule out? | [`../experiments-ledger.md`](../experiments-ledger.md) and the relevant result note |

Raw corpus-derived output, databases, and logs remain in the access-controlled
external artifact root. `experiments/runs/` contains only safe, committed
receipts. A planned track never becomes complete merely because a receipt
exists; its stated exit condition and any required HITL ruling still apply.

## Portfolio rules

- Every new performance or experiment plan names its track ID here and links
  back to this program before execution begins.
- Every executed evaluation writes a typed `record.json` and an append-only
  `experiments/index.jsonl` row before it is closed. Plans link to run IDs;
  they do not copy result fields or reimplement the scoreboard.
- Timing is partitioned into ingestion/indexing, retrieval/query, and
  answerer/judge time. CPU, CUDA, reranker-enabled, cold, and steady-state
  treatments are distinct cells and must not be pooled.
- A single run is descriptive unless its track pre-registers repetitions,
  uncertainty, and an eligibility rule for a stronger claim.
- `blocked_prerequisite` is evidence, not failure. Credential, cost, license,
  corpus-integrity, or environment blockers remain visible until resolved or
  explicitly retired.
- A retrieval treatment earns a default only against its declared baseline and
  query shape. A dense-only gain, a single receipt, or a competitor result does
  not establish a product default.
- Derived memories are projections, not independent sources of truth. A
  vector child, summary, extracted fact, or graph edge must retain canonical
  source identity and participate in its supersession and erasure semantics.
- Report LOCOMO factoid, temporal, and multi-session results separately. An
  overall score may screen candidates, but it must not conceal a material
  class regression.

## Metric families

| Family | Measures | Examples |
| --- | --- | --- |
| Retrieval quality | Whether the needed evidence is retrieved, at the appropriate unit | R@10, R@5/R@20, MRR, r@1, nDCG@10, child-evidence recall, parent/document recall, duplicate-result rate |
| Answer and task quality | Whether retrieved evidence yields a useful and grounded answer | answer accuracy, temporal correctness, MuSiQue EM/F1, AutoE comprehensiveness/diversity/empowerment |
| Time and resource cost | Observed work cost under a named treatment | query p50/p95/p99, ingest acknowledgement and ready-to-search time, mutation-to-ready time, projection storage amplification, throughput, seed/open cost, stress errors |
| Projection and lifecycle fidelity | Whether derived retrieval units remain correct, attributable, and erasable | canonical-source coverage, stale-hit rate after supersession, post-erasure searchability, projection completeness |
| Fidelity and scale | Retrieval fidelity at a specified corpus size and backend | vector-stage recall@10, bootstrap CI, completeness, manifest-qualified scale envelope |

## Portfolio board

Priority orders authorized effort; it does not authorize credentials, paid
work, or a release claim. Status is verified from the cited plan and execution
receipts, not inferred from a branch name or narration.

| ID | Priority | Track and decision | Status and dependency | Current plan |
| --- | --- | --- | --- | --- |
| SAFETY-01 | complete | Campaign controls and safe experiment receipts | Complete; retain as infrastructure | [track plan](tracks/safety-01-campaign-controls.md) |
| TRACE-01 | complete | Projection lifecycle integrity: can every derived retrieval unit retain canonical source identity and be superseded or erased without a stale hit? | Complete canary: `trace-projection.v1` is contract-tested on a fixed synthetic lifecycle fixture, independently reviewed, and integrated at `ca5b656d`. It enables lifecycle-dependent preparation; it does not authorize a live benchmark. | [track plan](tracks/trace-01-projection-lifecycle-integrity.md) |
| LOCOMO-01 | P1 | LOCOMO self-characterization: which FathomDB retrieval configuration is eligible for answer scoring? | Directional GPU data complete: the 26-cell RTX 3090 grid identifies turn-level `hybrid_ce_alpha_10_pool_20` as the retrieval survivor. Formal CPU+GPU closure was intentionally not pursued. | [track plan](tracks/locomo-01-self-characterization.md) |
| PARENT-01 | P1 | Parent-child LOCOMO screening: does retrieving a turn/child and returning its session/parent plus bounded neighbors improve evidence and answer context over parent-only retrieval? | Directional GPU data complete: bounded parent context does not improve child-evidence retrieval over the turn baseline, so it remains opt-in. | [track plan](tracks/parent-01-parent-child-screening.md) |
| SCALE-01 | complete | TC-5 eu7 scale fidelity: what manifest-qualified all-real GPU-primary vector-stage fidelity envelope is observed? | Complete: the registered 17,272-document, 100-query GPU primary provides bounded vector-stage fidelity and uncertainty evidence. CPU was not required. | [track plan](tracks/scale-01-tc5-fidelity.md) · [primary receipt](../../experiments/runs/tc5-gpu-primary-20260822T1605Z-2d574205/record.json) · [smoke receipt](../../experiments/runs/tc5-gpu-smoke-20260822T1446Z-2d574205/record.json) |
| CORPUS-01 | complete, limited | Agent-memory gold coverage: are temporal change, knowledge update, supersession, and erasure represented well enough for a broad personal-memory claim? | Two human reviewers judged both remaining supersession cases `insufficient_evidence` at `seq-268`. No qualified supersession gold exists, so the broad mutable-memory claim remains unsupported. | [track plan](tracks/corpus-01-gold-coverage.md) · [result](2026-08-23-corpus-01-supersession-human-review-result.md) |
| ANSWER-01 | P2 | LOCOMO shortlist scoring: does the accepted retrieval winner improve answer and temporal quality over A0? | Complete: the [32-question paired receipt](../../experiments/runs/answer-01-shortlist-live-20260822T1234Z-8a050808/record.json) did not show the required overall or temporal improvement; retain A0. | [track plan](tracks/answer-01-shortlist-scoring.md) |
| TEMPORAL-01 | P3 | Time-scoped retrieval: do temporal filters and version-aware projections return the correct state without stale superseded evidence? | Synthetic TRACE validity passed. The official releases lack an external validity-window manifest, so external-corpus comparison remains blocked on a reviewed manifest and adapter. | [track plan](tracks/temporal-01-time-scoped-retrieval.md) · [synthetic receipt](../../experiments/runs/temporal-01-trace-validity-20260823T1625Z-af0c03f1/record.json) · [source review](2026-08-23-temporal-01-source-manifest-review.md) |
| EXTRACT-01 | complete, limited | FathomDB-native extracted semantic memory: do provenance-linked facts improve knowledge-update quality enough to justify extraction and lifecycle costs? | Complete on 78 LongMemEval-S knowledge-update cases. The +1/78 descriptive quality delta does not overcome failed value-changing supersession; retain raw A0. Preferences, episodes, general memory, and confidence calibration are outside the claim. | [track plan](tracks/extract-01-semantic-memory.md) · [receipt](../../experiments/runs/extract-01-knowledge-update-20260823T2236Z-59e805cb/record.json) · [result](2026-08-23-extract-01-implementation.md) |
| MEMORY-01 | complete | Native Mem0 comparison: is the selected FathomDB memory profile near-parity or better under the official harness? | Pass: A0 scored 75.19% versus Mem0 OSS at 67.21%; the +7.99-point paired delta has a +5.78-point one-sided 95% lower bound. Mem0 retains an 8.16-point multi-hop advantage, so class-wide dominance is not claimed. | [track plan](tracks/memory-01-native-mem0-comparison.md) · [result](2026-08-24-memory-01-result.md) · [receipt](../../experiments/runs/fathomdb-vs-mem0-locomo-comparison-20260824T2140Z-01e702be/record.json) |
| SCALE-02 | complete | F-17 advisory scale envelope: what measured local-first range is supportable for the selected projection profile? | The original fixed-policy A0 envelope remains 17,272. The rank-boundary off-shoot passes through 50k with exact retrieval equivalence. HITL `seq-267` approved `stream_default`; the production FTS path is landed and code-verified. | [track plan](tracks/scale-02-local-first-envelope.md) · [original result](2026-08-22-scale-02-fts-followup-result.md) · [scale extension](2026-08-22-scale-02-scale-extension-result.md) · [rank-boundary result](2026-08-22-scale-02-rank-boundary-result.md) · [implementation note](2026-08-23-scale-02-stream-default-implementation.md) |
| LATENT-01 | P3 | Long-context/late-chunking feasibility: does a token-output, long-context embedder address a diagnosed cross-window discourse failure at acceptable cost? | Parked until LOCOMO-01/PARENT-01 diagnose that failure. Requires a model/interface preflight, labelled subset, and separate quality-and-cost contract; it is not a stride sweep. | [track plan](tracks/latent-01-late-chunking-feasibility.md) |
| GRAPH-01 | P3 | FathomDB graph-projection self-characterization: do high-confidence, provenance-backed graph projections improve multi-hop retrieval enough to justify extraction and maintenance? | Planned; requires a bounded graph design and supporting-evidence protocol before native graph comparison is prioritized. | [track plan](tracks/graph-01-projection-characterization.md) |
| GLOBAL-01 | P3 | Global sensemaking: can a bounded source-linked coverage treatment close the first run's coverage gap without losing directness, attribution, lifecycle fidelity, or acceptable cost? | Stopped, invalid witness. A/A passed, but no complete witness answer was generated; held-out was not run and no quality decision is eligible. | [track plan](tracks/global-01-native-graphrag.md) · [execution note](2026-08-29-global-01-witness-execution-note.md) · [receipt](../../experiments/runs/global-01-lazy-witness-20260829T1924Z-aa159044/record.json) |
| REASON-01 | parked | Native HippoRAG-2 comparison: cross-check multi-hop retrieval and QA. | Parked pending GRAPH-01 relevance, Python 3.10, official credential, and official-corpus reconciliation. | [track plan](tracks/reason-01-native-hipporag2.md) |
| SEARCH-01 | complete | IR-C FTS population: establish a FathomDB-only descriptive retrieval baseline. | Complete historical baseline; not a competitor, answer-quality, lifecycle, or latency claim. | [track plan](tracks/search-01-ir-c-baseline.md) |

## Execution order

Every track below is governed by the Codex-native
[Track Runner control](TRACK-RUNNER.md): a coordinator integrates isolated,
reviewed worker lanes; it is not a release-ladder workflow. The control fixes
the worker handoff, review, WIP, and authorization boundaries for the duration
of this program.

```text
SAFETY-01
├── TRACE-01 ──→ LOCOMO-01 ──→ PARENT-01 ──→ ANSWER-01 ──→ MEMORY-01
│                                     │
│                                     └──→ diagnosis: LATENT-01 or GRAPH-01
├── CORPUS-01 ──→ broad agent-memory claim eligibility
└── SCALE-01 ──→ SCALE-02

ANSWER-01 + CORPUS-01 + TRACE-01 ──┬──→ TEMPORAL-01
                                  └──→ EXTRACT-01
GRAPH-01 ──→ REASON-01
GLOBAL-01 starts only from a named global-synthesis failure and an eligible
graph or source-linked summary/map-reduce treatment.
```

LOCOMO-01 and PARENT-01 have accepted directional decisions and should not run
confirming grids. ANSWER-01 retains A0 as the answer-scored profile. SCALE-01
is complete on the registered GPU primary; no CPU bridge was required.
SCALE-02 retains its fixed-20-ms 17,272-record A0 envelope. Its separately
authorized rank-boundary off-shoot completed at 25k, 40k, and 50k. Streamed
BM25 boundary-tie completion preserved exact ordered top-100 and top-10
results, passed the scale-adjusted policy through 50k, and removed the need
for mmap128. HITL `seq-267` approved `stream_default` as the production FTS
path with shipped reader defaults. The production landing is complete; no
confirming benchmark was required.

CORPUS-01 is complete but limited: the two remaining supersession cases were
human-reviewed as `insufficient_evidence`, so the broad mutable-memory claim
remains unsupported. TimeQA, evaluation-only TimelineQA, and LongMemEval-S/oracle
are acquired, pinned, and registered as external evaluation inputs. The
upstream releases do not provide an external validity-window manifest, so
TEMPORAL-01's synthetic TRACE validity cell is complete; its corpus comparison
remains blocked on a reviewed manifest and adapter. EXTRACT-01 is complete for
its limited knowledge-update claim and retains raw A0 because native
value-changing extracted facts were not consolidated. LATENT-01 and GRAPH-01
start only from a diagnosed failure.
GLOBAL-01 stopped at an invalid development witness after A/A passed. No
held-out run or quality decision is eligible; reopen only with a new registered
execution contract. REASON-01 retains its independent native-run prerequisites.

## Track organization

Track files are the plans. Keep them short. Runnable detail goes in one small
configuration per run, and measured evidence goes in receipts:

```text
dev/performance-benchmarking/
  PROGRAM.md                  # this portfolio board and sequencing
  contracts/                  # shared comparison and measurement contracts
  tracks/                     # one short plan per decision-bearing track

experiments/
  configs/<track>/            # typed executable configurations
  <track runner>.py           # adapters and runners
  runs/<run-id>/              # safe typed receipts
  index.jsonl                 # append-only execution ledger
  INDEX.md, SCOREBOARD.md     # generated execution views
```

Do not create `dev/performance-benchmarking/results/`: measured evidence
belongs in `experiments/`. A concise narrative result note may interpret that
evidence, but it must link to the run ID and its receipt rather than duplicate
metrics. Historical result notes remain immutable evidence.

## Maintaining this board

Update this board when a track changes state, priority, dependency, or decision
question. In the same change, update the track plan and link the relevant
`experiments` run IDs. Do not update historical plans solely to make their
past state resemble the current portfolio; instead, link them from the active
track or result note that supersedes their role.

Before execution, the track plan must name its inputs, run, decision rule, and
stop condition. Put parameter lists in configuration rather than expanding the
plan. Do not let this portfolio overview become an executable measurement
contract.
