# GRAPH-01 protected bridge-completion design

## Purpose

`protected_bridge_v1` tests one remaining graph hypothesis: a graph may be
useful for identifying a missing bridge passage even though undirected BFS and
diffuse PPR ranking failed. It makes graph structure a bounded candidate
promotion signal while preserving the strongest eight control passages.

## Projection

The runner creates a new FathomDB 0.8.23 database and writes canonical document
nodes, question-namespaced entity nodes, and body-less relation edges. Exact
paragraph IDs are written as relation `source_id` values. This corrects the old
M1 database's question-level edge attribution without changing the inherited,
question-blind extraction payload.

The native database is the measured projection. Retrieval reads active nodes
and edges back from that database; it does not score directly from the input
JSON. A parity check requires the admitted edge count written to equal the
active attributed edge count read back.

## Retrieval

The control produces the BM25/dense RRF ranking. The graph profile builds an
adjacency list from admitted active edges, resolves exact query anchors, and
enumerates only paths of depth one or two ending in control-seed entities. It
promotes no more than two path-source paragraphs from control ranks 11–20 into
ranks 9–10. Stable lexical tie-breaking makes the result deterministic.

This shape controls the known graph failure modes:

- exact question anchors and type-conflict exclusion limit entity co-mingling;
- admitted edges require verbatim endpoints and exact source attribution;
- two-hop path and candidate-depth caps prevent graph fan-out;
- protected ranks prevent graph-only context loss;
- no graph score is fused with lexical score, so this is not the rejected PPR
  treatment in another form.

## Paid seams and resilience

The independent edge audit is batched and question-blind. Answer generation is
conditional on a complete graph-quality and retrieval run, uses the same model
and prompt for both arms, and shares one result where contexts are identical.
The checkpoint stores each successful cell plus usage and cost before the next
call. The caller reserves worst-case request cost, honors provider backoff, and
cannot cross the $20 cap.

## Failure semantics

Malformed extraction rows, missing provenance, input hash drift, a non-fresh
database root, missing native rows, malformed model JSON, exhausted retries,
or incomplete required cells stop the run. They are not converted into empty
edges, abstentions, or zero scores.
