# Overall performance benchmarking and experiments program

This is the authoritative plan for FathomDB's performance benchmarking and
experiments program. It prioritizes work, records dependencies, and names the
decision that each track may inform. It does not replace a track's frozen
measurement contract, and it is not a results ledger.

## System of record

Three layers have distinct responsibilities:

| Layer | Answers | Source of truth |
| --- | --- | --- |
| Program planning | What should run next, why, and after which prerequisites? | This document and its track plans |
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

## Metric families

| Family | Measures | Examples |
| --- | --- | --- |
| Retrieval quality | Whether the needed evidence is retrieved | R@10, R@5/R@20, MRR, r@1, nDCG@10, supporting-passage recall/precision |
| Answer and task quality | Whether retrieved evidence yields a useful answer | answer accuracy, temporal correctness, MuSiQue EM/F1, AutoE comprehensiveness/diversity/empowerment |
| Time and resource cost | Observed work cost under a named treatment | query p50/p95/p99, ingest acknowledgement and ready-to-search time, throughput, seed/open cost, stress errors |
| Fidelity and scale | Retrieval fidelity at a specified corpus size and backend | vector-stage recall@10, bootstrap CI, completeness, manifest-qualified scale envelope |

## Portfolio board

Priority orders authorized effort; it does not authorize credentials, paid
work, or a release claim. Status is verified from the cited plan and execution
receipts, not inferred from a branch name or narration.

| ID | Priority | Track and decision | Status and dependency | Current plan |
| --- | --- | --- | --- | --- |
| C0 | complete | Campaign controls and safe experiment receipts | Complete; retain as infrastructure | [execution runbook](2026-08-14-experiment-campaign-execution-plan.md) |
| L0 | P1 | LOCOMO FathomDB self-characterization: which retrieval configuration is eligible for scored comparison? | Active; complete Phase-A provenance, timing, and GPU readiness before the full grid | [LOCOMO campaign](2026-08-14-locomo-fathomdb-capability-campaign-plan.md) |
| T0 | P1 | TC-5 eu7 scale fidelity: what manifest-qualified all-real CPU fidelity envelope is observed? | Planned; manifest and test-only runner precede execution | [TC-5 plan](2026-08-14-eu7-tc5-scale-envelope-rebaseline-plan.md) |
| L1 | P2 | LOCOMO shortlist scoring: do the L0 retrieval winners improve answer and temporal quality? | Blocked on L0 shortlist and its scorer/cost preflight | [LOCOMO campaign](2026-08-14-locomo-fathomdb-capability-campaign-plan.md) |
| M0 | P3 | Native Mem0 comparison: is a selected FathomDB profile near-parity or better under the official harness? | Blocked on L0/L1 selection plus Docker, official credential, LongMemEval acquisition, and a spend ceiling | [Mem0 readiness](plans/2026-08-11-mem0-oss-baseline-readiness.md) |
| F0 | P3 | F-17 advisory scale envelope: what measured local-first range is supportable? | Blocked on T0's primary receipt and a pre-registered workload matrix | [execution runbook](2026-08-14-experiment-campaign-execution-plan.md) |
| G0 | parked | Native GraphRAG comparison: reproduce and then compare global sensemaking fairly | Blocked on a declared cost ceiling, metering/abort rule, and native reproduction | [GraphRAG readiness](plans/2026-08-11-graphrag-baseline-readiness.md) |
| H0 | parked | Native HippoRAG-2 comparison: cross-check multi-hop retrieval and QA | Blocked on Python 3.10, official credential, and official-corpus reconciliation | [HippoRAG-2 readiness](plans/2026-08-11-hipporag2-baseline-readiness.md) |
| I0 | complete | IR-C FTS population: establish a FathomDB-only descriptive baseline | Complete historical baseline; not a competitor or latency claim | [initial population](2026-08-11-initial-population-plan.md) |

## Execution order

```text
C0 → L0 → L1 → M0
      └──────→ T0 → F0
G0 and H0 remain parked until their independent prerequisites are met.
```

L0 and T0 may proceed in parallel once their own test and provenance
preconditions are met. GPU enablement gates only GPU L0 cells; CPU/FTS L0 cells
do not wait for it. M0 never starts from an arbitrary FathomDB profile: it uses
only the selected L0/L1 configuration and the shared native-comparator
contract.

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
