# GRAPH-01 protected bridge result

The registered 300-question reused-cohort characterization rejected
`protected_bridge_v1`. The treatment did not meet the retrieval or answer
route, so it must not replace the selected fused non-graph baseline.

The graph substrate itself was eligible. A fresh FathomDB 0.8.23 database held
21,033 admitted edges and 56,613 active nodes with complete paragraph source
links, no endpoint orphans, no active stale edges, and passing supersession and
erasure canaries. The independent question-blind audit judged 94/100 sampled
edges supported; its Wilson 95% interval was 0.875 to 0.972. Storage
amplification was 0.801 times the pinned corpus.

The bounded bridge treatment was not useful:

- On 143 defined three-/four-hop pairs, complete-bridge delta was -0.0070
  with a 95% bootstrap interval of -0.0210 to 0.0.
- Supporting-passage recall delta was -0.00233. The two-hop complete-bridge
  delta was +0.00758, but treatment and control differed on only 2.67% of all
  questions, below the registered 10% minimum.
- Graph add-on p95 was 5.98 ms, inside the 25 ms boundary.
- DeepSeek V4 Pro answer F1 was 0.11882 for both arms on all 144 registered
  three-/four-hop questions. Paired delta and its interval were exactly zero.
- Total measured model cost, including invalid and retried audit cells, was
  $0.3293 against the authorized $20 cap.

Retain the fused RRF control. Do not promote or tune `protected_bridge_v1`
against these reused-cohort outcomes, and do not start REASON-01 on this
evidence. The result supports a narrower conclusion: FathomDB can maintain a
provenance-complete, lifecycle-clean graph at acceptable query and storage
cost, but this exact-anchor protected bridge caller does not convert that
substrate into better multi-hop retrieval or answers.

The complete safe evidence is the
[GRAPH-01 receipt](../../experiments/runs/graph-01-protected-bridge-20260830T0035Z-d6e7c4b2/record.json).
