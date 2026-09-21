# Graph benchmark implementation plan

**Project:** Performance Gauntlet v1.2 graph extension

**Proposed cells:** `GRAPH-EVIDENCE-01`, `GRAPH-EXPAND-01`,
`GRAPH-RETRIEVAL-01`

**Status:** implementation plan; no live execution authorized by this document

**Program relationship:** proposed successor tracks to the
[performance benchmarking program](../PROGRAM.md). Before implementation
begins, add the three track states and short track plans to that program so the
existing Track Runner can brief and record them.

## Outcome

Add three repeatable graph cells using the same structure as the existing
performance program:

- typed, hash-bound configurations;
- thin runners that reuse FathomDB and existing experiment helpers;
- fresh databases and explicit runtime identities;
- safe external raw artifacts plus committed aggregate receipts;
- separate quality, fidelity, and latency outputs; and
- top-level gauntlet adapters that orchestrate the cells without duplicating
  their measurement logic.

The first implementation target is FathomDB 0.8.26. The contracts must remain
runnable against later compatible releases so the result becomes a versioned
comparison rather than a one-off release demonstration.

## Community benchmark research

No one public benchmark covers FathomDB's combination of bounded graph
expansion, frozen reads, exact target-and-terminal-edge evidence, retrieval,
supersession, and erasure. The implementation should therefore adopt standard
datasets and reporting conventions by measurement family while stating where
the adapter is not an official benchmark implementation.

| Source | What should be reused | What must not be claimed |
| --- | --- | --- |
| [LDBC SNB Interactive](https://ldbcouncil.org/benchmarks/snb/interactive/) | Scale factors, generated power-law graph data, neighborhood/path query shapes, continuous updates, validation before performance, warm-up, throughput, and single-query timings | Official or audited LDBC compliance; FathomDB does not implement the complete SNB query/update surface |
| [LinkBench](https://github.com/facebookarchive/linkbench) | Power-law degree distribution, hot/cold access, adjacency-list operations, configurable concurrency/rate, latency percentiles, throughput, and resource reporting | A LinkBench score unless the official driver and complete graph-store adapter contract are implemented |
| [LDBC Graphalytics](https://ldbcouncil.org/benchmarks/graphalytics/) | Directed BFS correctness fixtures, reference outputs, dataset scaling, and repeated-run validation | A Graphalytics platform result; its full-graph analytics workload differs from bounded embedded traversal |
| [MuSiQue](https://aclanthology.org/2022.tacl-1.31/) | Existing 2–4-hop questions, supporting-paragraph labels, hop stratification, complete-bridge retrieval, support recall, and answer EM/F1 | A pristine held-out result when the retained 300-question cohort or cached extractions are reused |
| [HotpotQA](https://hotpotqa.github.io/) | Supporting-fact and joint answer/evidence evaluation as an independent two-hop confirmation dataset | A graph-specific comparison without a question-blind, identically built graph projection |
| [STaRK](https://github.com/snap-stanford/stark) | Native textual-plus-relational retrieval task, official splits, entity qrels, MRR, MAP, R-precision, Recall@K, and Hit@K | A STaRK score from a sampled or structurally altered knowledge base |

STaRK is the strongest external match for graph-assisted retrieval because its
queries jointly depend on textual and relational knowledge. Its official
datasets are large: STaRK-Prime has roughly 129,000 entities and 8.1 million
relations, while Amazon and MAG are larger. Acquisition and a full-scale
resource preflight therefore precede any headline STaRK cell.

Graph provenance research supplies useful concepts, but no mature benchmark
found in this survey exercises FathomDB's exact opaque-reference and frozen
authorization contract. `GRAPH-EVIDENCE-01` must consequently be described as
a deterministic FathomDB conformance and lifecycle-fidelity benchmark, not a
cross-vendor score.

## Shared implementation contract

### Delivery gates

Implementation begins only after two short, durable artifacts are approved:

1. `GRAPH-BENCHMARKS-REQUIREMENTS.md` assigns stable `GE-*`, `GX-*`, and
   `GR-*` identifiers to every requirement and executable acceptance
   criterion. Each criterion names its test, configuration or result field,
   and pass condition. Requirements and acceptance criteria are frozen before
   runner or scorer code is written.
2. `GRAPH-BENCHMARKS-DESIGN.md` maps those requirements to modules, schemas,
   offline oracles, data flows, timing boundaries, immutable workload
   manifests, failure states, and gauntlet integration. It records the exact
   standalone, gauntlet, receipt-comparison, and end-to-end smoke commands,
   including graph selector, configuration paths, and new external output-root
   arguments, so final verification runs approved commands rather than
   reconstructing them from prose.

An independent, read-only review of the design returns either `APPROVE` or a
numbered `FIX-n` verdict with concrete defects. Apply and re-review at most
four rounds (`FIX-1` through `FIX-4`). If the design is not approved after the
fourth fix, stop and narrow or revise the design; do not begin implementation
under an unresolved review.

Slices 20 through 80 use acceptance-test-driven TDD. For each behavior, first
add the named test and retain the exact command and expected failing result
(`RED`), then add the minimum implementation and retain the passing result
(`GREEN`). Existing tests and reviewed oracles are not weakened merely to
make implementation pass. The slice receipt or commit history provides the
compact RED/GREEN evidence; no separate process report is required.

### Reuse instead of duplication

Build on these existing assets:

- `experiments._lib` for canonical configurations, run IDs, receipts, and the
  append-only experiment index;
- `experiments.fathomdb_test_setup` for new output roots, fresh databases,
  device policy, and doctor evidence;
- `experiments.graph_01` for pure question-blind relation admission,
  deterministic projection writes, candidate mechanics, and paired bootstrap
  helpers;
- `experiments/graph_01_live.py` for MuSiQue input loading, external artifact
  separation, checkpointing, resume behavior, projection construction, and
  lifecycle canaries. Extract a public shared loading/projection helper before
  reuse; do not import its private functions or duplicate them;
- the 0.8.26 installed graph-evidence matrix and validator for exact-evidence
  cases rather than reimplementing its correctness oracle; and
- `scripts/perf-experiments/run_gauntlet.py` and `gauntlet_cells.py` for
  orchestration, selection, preflight, status, and summary generation.

New code should extract reusable helpers from an existing module only when the
new cell otherwise has to copy logic. Historical configurations and receipts
remain immutable.

### Proposed artifacts

```text
dev/performance-benchmarking/gauntlet-v1.2/
  GRAPH-BENCHMARKS-REQUIREMENTS.md
  GRAPH-BENCHMARKS-DESIGN.md
experiments/
  graph_evidence_01.py
  graph_expand_01.py
  graph_retrieval_01.py
  configs/graph-evidence-01/exact-evidence.v1.json
  configs/graph-expand-01/bounded-traversal.v1.json
  configs/graph-retrieval-01/musique-native-expand.v1.json
  configs/graph-retrieval-01/stark.v1.example.json
scripts/perf-experiments/
  run-graph-evidence01.py
  run-graph-expand01.py
  run-graph-retrieval01.py
tests/experiments/
  test_graph_evidence_01.py
  test_graph_expand_01.py
  test_graph_retrieval_01.py
  test_perf_gauntlet_graph_cells.py
```

The three `experiments.*` modules own configurations, workloads, metrics, and
receipts. The `scripts/perf-experiments` files are output-local adapters only.
The gauntlet gains three optional cells after their standalone runners pass;
they should not silently alter the original v1.2 ten-cell default.

### Common output

Each run writes:

- the immutable resolved configuration and its digest;
- source commit, package version, native-extension and CLI hashes;
- corpus/generator version and hashes;
- host, SQLite, CPU, memory, and storage identity;
- per-operation raw observations beneath the external artifact root;
- `metrics.json` with aggregate quality or performance metrics;
- `record.json` using `experiments.record.v1`; and
- one append-only `experiments/index.jsonl` entry.

The gauntlet summary references each cell receipt and artifact digest. It does
not copy per-query records or recompute cell metrics.

### Measurement identity and statistics

Every registered configuration pins the query or operation count,
repetitions, warm-up rule, pairing unit, bootstrap seed and draw count where
used, and percentile estimator. Receipts report attempted, completed, error,
and typed-refusal denominators separately. Results lacking those identities
are exploratory and descriptive only; they cannot support version-to-version
inferential claims.

## GRAPH-EVIDENCE-01

### Question

For every target selected by one frozen graph traversal, can FathomDB resolve
the exact disclosed target revision and winning terminal-edge revision to the
authorized canonical source, deterministically and without stale disclosure?

### Fixture and matrix

Start from the existing installed-artifact evidence profile and its Cartesian
matrix:

- explicit and query seeds;
- outgoing, incoming, and both directions;
- depths 1 and 2;
- ordinary and atomically actuated edges; and
- parallel-edge cases whose winning terminal edge is known in advance.

Add deterministic lifecycle cases rather than another hand-authored graph:

1. Active target and edge resolve successfully.
2. Parallel edges resolve the exact traversal winner, not a reselected edge.
3. Target supersession after the frozen context produces the documented
   nondisclosing refusal or state-drift result.
4. Edge supersession does the same.
5. Canonical-source erasure makes both target and terminal-edge references
   unresolvable.
6. Foreign-database, foreign-context, cross-request, replayed, and bit-tampered
   references fail nondisclosingly.
7. Empty results perform no evidence hydration.
8. `include_evidence=false` remains byte-compatible with the non-evidence
   response shape.

Generate semantic expectations from the fixture before opening FathomDB: the
target revision, winning terminal-edge revision and endpoints, edge kind,
canonical source bytes and hash, and expected refusal class. Opaque
`fdbgev1` references are minted by the system under test and are therefore not
fixture oracles. Validate them with the installed-artifact evidence profile,
`scripts/release/slice50-evidence-matrix.py`, and
`scripts/release/smoke/frozen-evidence-python.py`: check syntax, context and
database binding, resolution to the fixture-owned semantic expectations,
nondisclosure, replay isolation, and tamper rejection. Never derive expected
semantics from a reference returned by the system under test.

### Fidelity metrics

Report counts and rates, not one blended score:

- eligible targets;
- evidence sidecar positional completeness;
- target-revision exact-match rate;
- winning-terminal-edge revision exact-match rate;
- canonical-source hash and byte-match rate;
- deterministic resolved semantic-identity digest across repetitions;
- stale target disclosures after supersession;
- stale edge disclosures after supersession;
- post-erasure successful resolutions;
- unauthorized/tampered references accepted; and
- correct typed refusals by refusal class.

The conformance expectation is 100% for all positive exact-match rates and
zero for stale, post-erasure, foreign, and tampered acceptance. Report failures
as fidelity failures, not latency outliers.

### Cost metrics

In a separate table, measure evidence disabled, sidecar only, and sidecar plus
both resolutions:

- p50/p95/p99 graph expansion latency;
- p50/p95/p99 resolution latency per reference and per target pair;
- response bytes and evidence-reference bytes per target;
- SQL statement count when the supported internal witness is available;
- RSS delta; and
- evidence-on minus evidence-off additive latency.

Use fresh processes for RSS-sensitive arms. Run correctness before timing and
exclude mutation/refusal cases from success-path latency distributions.

## GRAPH-EXPAND-01

### Question

How does bounded `graph.expand` behave as graph size, degree skew, frontier
size, depth, seed count, filters, and concurrency change, and does every result
remain identical to an independent BFS oracle within the declared work bound?

### Datasets

Implement two tiers:

1. **Correctness tier:** vendor-neutral directed and undirected BFS fixtures
   derived from the small Graphalytics test graphs, with pinned source and
   reference hashes. Independently compute depth-limited reachability,
   shortest hop, direction, and deterministic target order before loading the
   database.
2. **Performance tier:** a deterministic LinkBench/LDBC-inspired generator
   producing typed nodes, directed edges, power-law degree skew, hot and cold
   seeds, multiple edge kinds, and configurable scale. The generator emits a
   manifest and reference adjacency digest. It must not use FathomDB to
   generate expected answers.

Qualification may use preflight to propose local-first scales so that the
small point runs in seconds and the largest point is materially beyond cache.
Before the first 0.8.26 measured run, freeze a hash-bound workload manifest
containing exact node and edge counts, graph and adjacency digests, generator
version and seed, registered cells, operation mix, hot/cold distribution,
open- or closed-loop model, offered rates, warm-up, measured operation count
or duration, repetitions, topology, and percentile estimator. A reasonable
candidate is 10k, 50k, and 100k nodes with approximately 5, 20, and 50 edges
per node. Later releases run the same manifest or report `blocked_resource`;
they never silently shrink or requalify a point downward.

### Workload matrix

For each qualified scale, exercise:

- explicit seed counts 1, 4, and 16;
- query seeds with ranked limits 1, 5, and 10;
- directions outgoing, incoming, and both;
- canonical depths 0, 1, and 2; any depth-3 legacy-API diagnostic has a
  separate non-comparable cell identity and cannot satisfy the canonical
  benchmark;
- low-, median-, and high-degree seed strata;
- no filter, edge-kind filter, target-kind filter, and eligibility filter;
- current and frozen contexts;
- explanation off/on;
- evidence off/on where frozen evidence is valid;
- concurrency 1, 2, 4, and 8; and
- exact work bound plus one below/above only for registered fixtures whose
  oracle work is within 2–9,999; depth zero and API endpoints use their
  separately valid boundary rules.

Do not execute a full Cartesian explosion. A typed configuration lists the
registered cells: a core pairwise matrix plus isolated factor sweeps. Every
cell retains its workload identity so later releases run the same set.

### Correctness and bounded-work outputs

For every operation verify:

- exact target IDs and ordering;
- shortest hop and terminal direction;
- deduplication across seeds and paths;
- eligibility-before-expansion behavior;
- work-unit count;
- completeness and degradation codes;
- exact success at the required work bound; and
- typed all-or-nothing refusal below the required bound, with no partial
  target set reported as success.

### Performance outputs

Follow LinkBench/LDBC reporting conventions while keeping a FathomDB-specific
name:

- load time and edges/nodes per second;
- database and WAL bytes;
- warm-up duration and measured duration;
- operation count;
- p50/p95/p99/max latency per query shape;
- operations per second at each concurrency or offered-rate point;
- returned targets and native work units, plus independently computed oracle
  edge-row counts labelled as estimates rather than native visit telemetry;
- response bytes;
- errors and typed refusals;
- process RSS and CPU time; and
- latency-versus-throughput curve.

The result note must say **LinkBench/LDBC-inspired bounded traversal**, not
`LinkBench`, `LDBC SNB`, or `Graphalytics` compliance.

## GRAPH-RETRIEVAL-01

### Question

Does native, bounded graph expansion improve labelled evidence retrieval over
the same non-graph search control, and on which hop counts and query shapes?

### Primary comparable protocol: MuSiQue

Reuse the existing MuSiQue assets first because they provide continuity with
GRAPH-01 and HippoRAG-style reporting:

- official answerable development set: 2,417 questions, comprising 1,252
  two-hop, 760 three-hop, and 405 four-hop questions;
- supporting-paragraph labels and answer aliases;
- current local corpus hash and the official-reproduction hash both recorded;
- existing 300-question cohort and cached question-blind extractions for
  historical continuity; and
- existing relation admission and projection rules.

Before a new scored run, reconcile the local `musique_dev.jsonl` representation
with the official MuSiQue/HippoRAG reproduction bundle. If they cannot be made
identity-equivalent, keep two explicitly named cells: `historical-300` for
continuity and `official-dev` for external comparison. Never pool them.

The retained GRAPH-01 observations contain only top-10 control rankings. Before
scoring @20, materialize a one-time top-20 fused seed manifest from the pinned
historical corpus/configuration using the pinned BGE model and CLS pooling on
an RTX 3090. Refuse CPU fallback. Qualification requires exact top-10 parity
for every retained question before the new ranks 11–20 are accepted. Retain
the manifest externally and bind its digest, model files, GPU UUID, and
materializer identity into every retrieval receipt. A live retrieval run must
never regenerate this manifest implicitly.

### Arms

Use one frozen projection and identical initial retrieval for every arm.
Materialize the initial ranked passage list once per question in a hash-bound
seed manifest; every arm consumes that exact list instead of rerunning search.
The runner fails before scoring if question IDs, ranked seed IDs, or seed
digests differ across arms:

1. `fused_rrf_k60`: the retained BM25 plus BGE passage-dense control, top 10,
   candidate depth 20.
2. `native_expand_depth1`: the same search seeds followed by native bounded
   graph expansion to depth 1.
3. `native_expand_depth2`: the same search seeds followed by native bounded
   graph expansion to depth 2.

Expanded graph artifacts are not inherently ranked passages. Convert them to
evidence only through their exact terminal-edge/source evidence, then apply a
single deterministic, predeclared candidate policy within the same top-10
passage budget. The reviewed design freezes the promotion limit, protected
prefix, evidence-to-passage mapping, deduplication, tie-break order, and
candidate budget. Reuse the protected-prefix and maximum-promotion mechanics
from GRAPH-01 where possible. Do not tune a new promotion rule on the retained
300-question outcomes.

The native treatment must read traversal results back from FathomDB. It must
not score an in-memory adjacency structure built from the extraction input.

### Retrieval outputs

Report overall and separately for two-, three-, and four-hop questions:

- all-supporting-passages-present@5, @10, and @20;
- mean supporting-passage recall@5, @10, and @20;
- supporting-passage precision@10;
- MRR of the first supporting passage;
- nDCG@10 where graded relevance is well defined;
- candidate-set change rate;
- useful expansion rate: questions where expansion adds a missing supporting
  passage;
- harmful displacement rate: questions where a supporting passage is removed
  from the fixed budget;
- no-op expansion rate; and
- graph-on minus graph-off paired deltas with question-level bootstrap
  intervals.

Keep answer EM/F1 in a separate optional phase with an identical answerer,
prompt, context budget, and failure policy. Retrieval can complete without
authorizing paid answer generation.

### External comparability extension: STaRK

After the MuSiQue runner is stable, qualify one full official STaRK knowledge
base and split. Prefer STaRK-Prime only if the complete 8.1-million-relation
projection fits the declared local resource envelope; otherwise stop as
`blocked_resource` rather than publish a sampled STaRK score.

The qualification manifest pins the STaRK repository commit, dataset variant
and split, evaluator version and digest, exact baseline configuration, and
entity, relation, query, and qrel counts and hashes. It also proves complete
entity/relation mapping and reproduces the official evaluator on identical
predictions within the numeric tolerance frozen in the requirements. Any
missing identity, incomplete mapping, or parity failure blocks the STaRK cell.

Map official entities to canonical nodes, official relations to edges, textual
attributes to searchable bodies, and answer entity IDs to qrels. Preserve the
official split and evaluator metrics:

- MRR;
- MAP;
- R-precision;
- Recall@5/10/20/50/100; and
- Hit@1/3/5/10/20/50.

Compare FathomDB text-only, native graph-expand, and the official STaRK BM25 or
hybrid baseline under the same entity corpus. Model-backed STaRK baselines are
separate cells with pinned models and cost. A full, unchanged official
knowledge base and split is required before the label `STaRK` appears in a
result table.

HotpotQA distractor development may be added later as an independent two-hop
supporting-fact confirmation. It should not delay the MuSiQue or STaRK work.

## Implementation sequence

### Slice 10 — Qualify inputs and approve the contract

- Pin upstream revisions and licenses for Graphalytics test graphs, the chosen
  LinkBench/LDBC generator inputs, MuSiQue official dev, and STaRK.
- Record hashes in the corpus registry; keep EVAL-ONLY payloads outside Git.
- Reconcile MuSiQue local and official representations.
- Produce zero-run size, disk, memory, and expected-time estimates.
- Write the requirements and executable acceptance criteria with stable IDs
  and test/schema/result mappings.
- Write the design described under Delivery gates, including the frozen
  GRAPH-EXPAND workload and GRAPH-RETRIEVAL seed and candidate policies.
- Obtain an independent `APPROVE` verdict, applying no more than four
  `FIX-n` review cycles.

**Exit:** every required local input is hash-bound or has a typed blocker, and
the requirements and design are approved and frozen. No runner, scorer, or
gauntlet implementation begins before this exit passes. Any smaller runnable
tier remains explicitly distinct from the blocked full benchmark.

### Slice 20 — Shared schemas and offline oracles

- **RED:** add unit/property and schema tests for ordering, cycles, parallel
  edges, duplicate seeds, hop counts, work bounds, retrieval metrics, and
  malformed or drifted inputs; retain their expected failures.
- **GREEN:** define strict v1 configurations and result schemas and implement
  the minimum independent BFS, evidence-semantic, and qrel scorers needed to
  pass, with no live database dependency.

**Exit:** malformed or drifted configurations fail, and every scorer passes
fixed hand-computed fixtures.

### Slice 30 — Implement GRAPH-EVIDENCE-01

- **RED:** add the approved conformance, lifecycle, refusal, and output-shape
  acceptance tests and retain their failures against the absent adapter.
- **GREEN:** adapt the existing installed-artifact evidence matrix with the
  minimum implementation needed to pass the acceptance tests.
- Add lifecycle, authorization, tamper, deterministic replay, and overhead
  arms.
- Emit separate fidelity and timing tables.

**Exit:** a fresh 0.8.26 database produces a complete typed receipt and all
negative cases fail nondisclosingly.

### Slice 40 — Implement GRAPH-EXPAND-01 correctness

- **RED:** add the approved graph fixtures, BFS parity, ordering, and
  all-or-nothing work-bound tests and retain their failures.
- **GREEN:** add Graphalytics-derived small fixtures and the minimum
  independent BFS adapter needed to pass the acceptance tests.
- Exercise seed, direction, depth, filter, cycle, deduplication, ordering, and
  work-bound behavior.

**Exit:** exact output parity is established before performance measurement is
enabled.

### Slice 50 — Implement GRAPH-EXPAND-01 performance

- **RED:** add tests for workload-manifest drift, timing boundaries,
  denominator accounting, and required performance fields; retain their
  failures.
- **GREEN:** add the deterministic power-law generator and qualified scale
  points with the minimum measurement implementation needed to pass.
- Add warm-up, rate/concurrency sweeps, resource sampling, and fresh-process
  RSS arms.
- Keep correctness assertions active during timed cells.

**Exit:** repeated runs emit stable identities and complete latency,
throughput, work, storage, and resource fields.

### Slice 60 — Implement GRAPH-RETRIEVAL-01 MuSiQue

- **RED:** add tests that reject seed-manifest drift, mismatched arms, budget
  violations, and candidate-policy drift; retain their failures.
- **GREEN:** reuse the GRAPH-01 projection and scorers and add the minimum
  native-expansion adapter needed to pass.
- Replace in-memory treatment traversal with native FathomDB expansion and
  exact source evidence.
- Run the historical cohort only as a compatibility dry run, then execute the
  registered official-dev protocol when qualified.

**Exit:** graph-off, depth-1, and depth-2 arms contain identical question IDs,
initial retrieval, candidate budgets, and complete per-hop metrics.

### Slice 70 — Add full STaRK adapter if qualified

- **RED:** add qualification tests for all pinned identities, complete
  mappings, official evaluator parity, and sampled-data rejection; retain
  their failures.
- **GREEN:** load one complete official knowledge base and split with the
  minimum adapter needed to pass qualification.
- Reproduce one official baseline before interpreting FathomDB results.
- Emit official retrieval metrics and separate system-cost measurements.

**Exit:** full official identity and evaluator parity pass; otherwise preserve
the blocker and do not emit a STaRK score.

### Slice 80 — Gauntlet integration

- **RED:** add orchestration tests for exact invocation, failure isolation,
  resume, and standalone-versus-gauntlet receipt equality; retain their
  failures.
- **GREEN:** add the three cells to the shared cell registry as optional named
  cells with the minimum orchestration changes needed to pass.
- Reuse normal preflight, resume, failure isolation, output hashing, and
  summary generation.
- Add an explicit graph suite selector; do not change the original default ten
  cells until a later decision does so deliberately.

**Exit:** dry-run shows exact invocations and inputs, standalone and gauntlet
stable receipt-identity projections match, and one failed graph cell cannot
erase the others' evidence. Run-specific timestamps, run IDs, paths, and
latencies are excluded from that projection.

### Slice 90 — Verification

- Run the focused graph suite, including configuration-drift, receipt-forgery,
  scorer, and orchestration negatives:

  ```bash
  PYTHONPATH=src/python .venv/bin/python -m pytest \
    tests/experiments/test_graph_evidence_01.py \
    tests/experiments/test_graph_expand_01.py \
    tests/experiments/test_graph_retrieval_01.py \
    tests/experiments/test_perf_gauntlet_graph_cells.py -q
  ```

- Execute the reviewed design's exact standalone, gauntlet, and
  receipt-comparison commands; require identical resolved configurations and
  artifact digests.
- Review metric definitions, timing boundaries, dataset labels, and external
  benchmark claims.
- Execute the reviewed design's exact end-to-end smoke command with fresh
  databases, a new external output root, and no paid models. Bind source and
  benchmark inputs to recorded Git and artifact identities; disclose
  unrelated dirty files and prove they are not consumed rather than requiring
  a globally clean shared checkout.
- Run `./scripts/agent-verify.sh` last and preserve its unaltered diagnostic if
  it fails.

**Exit:** the implementation is repeatable from configuration, all retained
artifacts are hashed, and the summary keeps fidelity, retrieval quality, and
speed in separate sections.

## Decision and reporting boundaries

- A run is comparable only when its registered configuration pins the
  measurement identities and statistics above. Otherwise it is directional
  and descriptive only.
- `GRAPH-EVIDENCE-01` is a FathomDB contract test, not a competitor score.
- `GRAPH-EXPAND-01` is inspired by established graph-database benchmarks but
  is not an official LDBC, LinkBench, or Graphalytics result.
- Reusing the GRAPH-01 300-question cohort is historical continuity, not a new
  held-out claim.
- The prior `protected_bridge_v1` result remains rejected; native expansion
  does not reopen it or permit tuning on its outcomes.
- Graph retrieval quality and graph latency are reported separately. A fast
  traversal with no relevance lift is a valid negative result.
- GPU is used for corpus embedding or model inference when such a cell is
  enabled. Pure graph traversal and evidence resolution remain CPU/database
  workloads and must not be described as GPU work.
- Corpus acquisition, full-scale STaRK execution, GPU/model work, paid answer
  generation, and external publication each retain their normal explicit
  authorization boundary.
