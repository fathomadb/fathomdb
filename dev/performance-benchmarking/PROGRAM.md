# Overall performance benchmarking and agent-memory experiments program

This is the authoritative plan for FathomDB's performance benchmarking and
experiments program. It prioritizes work, records dependencies, and names the
decision that each track may inform. It does not replace a track's frozen
measurement contract, and it is not a results ledger.

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
| LOCOMO-01 | P1 | LOCOMO self-characterization: which FathomDB retrieval configuration is eligible for answer scoring? | Active; complete Phase-A provenance, timing, and GPU readiness before the full grid. Report factoid, temporal, and multi-session results separately. | [track plan](tracks/locomo-01-self-characterization.md) |
| PARENT-01 | P1 | Parent-child LOCOMO screening: does retrieving a turn/child and returning its session/parent plus bounded neighbors improve evidence and answer context over parent-only retrieval? | Planned; TRACE-01's contract dependency is satisfied. One bounded, pre-registered treatment may join the LOCOMO grid only through a frozen amendment; otherwise it follows LOCOMO-01 before an answer-scoring or comparator winner is chosen. | [track plan](tracks/parent-01-parent-child-screening.md) |
| SCALE-01 | P1 | TC-5 eu7 scale fidelity: what manifest-qualified all-real CPU fidelity envelope is observed? | Planned; manifest and test-only runner precede execution. May proceed independently of LOCOMO work. | [track plan](tracks/scale-01-tc5-fidelity.md) |
| CORPUS-01 | P1 | Agent-memory gold coverage: are temporal change, knowledge update, supersession, and erasure represented well enough for a broad personal-memory claim? | Planned; prepare corpus/license and human-gold readiness in parallel. LOCOMO alone is insufficient because it has no knowledge-update class. | [track plan](tracks/corpus-01-gold-coverage.md) |
| ANSWER-01 | P2 | LOCOMO shortlist scoring: do the selected retrieval survivors improve answer and temporal quality? | Blocked on the selected LOCOMO-01/PARENT-01 survivor and scorer/cost preflight. Preserve class-level results and attribution. | [track plan](tracks/answer-01-shortlist-scoring.md) |
| MEMORY-01 | P3 | Native Mem0 comparison: is the selected FathomDB memory profile near-parity or better under the official harness? | Blocked on ANSWER-01 selection plus Docker, official credential, LongMemEval acquisition, and a spend ceiling. Report raw-evidence and extracted-semantic-memory regimes separately. | [track plan](tracks/memory-01-native-mem0-comparison.md) |
| SCALE-02 | P3 | F-17 advisory scale envelope: what measured local-first range is supportable for the selected projection profile? | Blocked on SCALE-01's primary receipt, a selected LOCOMO-01/PARENT-01 profile, and a pre-registered workload matrix. Measure canonical records and derived projection rows separately. | [track plan](tracks/scale-02-local-first-envelope.md) |
| LATENT-01 | P3 | Long-context/late-chunking feasibility: does a token-output, long-context embedder address a diagnosed cross-window discourse failure at acceptable cost? | Parked until LOCOMO-01/PARENT-01 diagnose that failure. Requires a model/interface preflight, labelled subset, and separate quality-and-cost contract; it is not a stride sweep. | [track plan](tracks/latent-01-late-chunking-feasibility.md) |
| GRAPH-01 | P3 | FathomDB graph-projection self-characterization: do high-confidence, provenance-backed graph projections improve multi-hop retrieval enough to justify extraction and maintenance? | Planned; requires a bounded graph design and supporting-evidence protocol before native graph comparison is prioritized. | [track plan](tracks/graph-01-projection-characterization.md) |
| GLOBAL-01 | parked | Native GraphRAG comparison: reproduce and compare global sensemaking fairly. | Parked pending GRAPH-01 relevance, a declared cost ceiling, metering/abort rule, and native reproduction. This is global-sensemaking calibration, not a default personal-memory gate. | [track plan](tracks/global-01-native-graphrag.md) |
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

GLOBAL-01 and REASON-01 remain parked until GRAPH-01 relevance and their
independent external prerequisites are met.
```

TRACE-01, CORPUS-01, LOCOMO-01, and SCALE-01 may prepare in parallel once their
own test, provenance, and corpus preconditions are met. CPU/FTS LOCOMO-01 cells
do not wait for GPU enablement; it gates only GPU cells. PARENT-01 may join the
LOCOMO-01 grid only through a frozen amendment; otherwise it runs before
ANSWER-01 selects its shortlist. MEMORY-01 never starts from an arbitrary
FathomDB profile: it uses only the selected answer-scored profile and the
shared native-comparator contract.

SCALE-02 is not a generic document-count exercise. Its workload matrix names
the selected profile, canonical-record count, and every derived projection-row
or vector count. It must report those quantities separately so that a
parent-child, summary, or graph projection multiplier cannot be hidden behind
a user-visible memory count.

## Track organization

The existing dated documents remain the current executable plans during this
transition. New or materially rewritten plans use this logical layout:

```text
dev/performance-benchmarking/
  PROGRAM.md                  # this portfolio board and sequencing
  contracts/                  # shared comparison and measurement contracts
  tracks/                     # one decision-bearing plan per active track

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

Before executing a row marked "New plan required" or "New plan or
LOCOMO-plan amendment required", create the dated plan, register its track ID
here, and freeze its measurements, uncertainty treatment, cost limit, and
eligibility rule. Do not let this portfolio board itself become an executable
measurement contract.
