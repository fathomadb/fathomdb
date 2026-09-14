# Frozen evidence

Use frozen reads when ranking and evidence must share one authenticated view of
validity, eligibility, and database state.

## Choose the operation

| Need | Use |
| --- | --- |
| Frozen top-K ranking without source evidence | `freeze_read_context` → `search_frozen` |
| Evidence-backed answer | `freeze_read_context` → `search_with_evidence` → `resolve_evidence` for selected hits |
| Stable collection pagination | `read.canonical_page`; search is top-K, not a page stream |
| Evidence for an exact graph target and winning edge | `graph.expand` with `include_evidence=True` / `includeEvidence: true` → `resolve_graph_evidence` / `resolveGraphEvidence` |

Do not call `search_frozen` before `search_with_evidence` in the normal grounded
path. That double-search adds work and does not transfer evidence identity.
`search_frozen` remains useful when no evidence will be exposed and for
diagnostic ranking experiments.

## Grounded Python flow

```python
import fathomdb

context = engine.freeze_read_context(
    fathomdb.ReadContextV1(
        eligibility=fathomdb.SearchFilter(kind="claim"),
    )
)
result = engine.search_with_evidence(
    fathomdb.EvidenceSearchRequestV1(
        query="what changed?",
        context=context,
        include_explanation=True,
    )
)

selected = result.evidence[0]
evidence = engine.resolve_evidence(
    fathomdb.EvidenceResolveRequestV1(
        evidence_ref=selected.evidence_ref,
        context=context,
    )
)
```

Evidence entries are positionally associated with search hits. Check
`result_index` or use the same tuple position. An evidence reference locates
governed evidence but is not authorization by itself; resolution rechecks the
equivalent frozen context and current visibility. Reference strings contain
fresh randomness, so compare artifact revision identity and resolved semantics,
not reference bytes from separate searches.

## Exact graph evidence

Graph evidence is also frozen and opt-in. It does not use body re-search or a
raw ID lookup:

```python
from dataclasses import replace

evidence_request = replace(
    base_graph_request,
    context=fathomdb.FrozenGraphReadContextV1(
        schema_version=1, type="frozen", context=context
    ),
    include_evidence=True,
)
graph_result = fathomdb.graph.expand(
    engine,
    evidence_request,
)

selected = graph_result.evidence.entries[0]
target_evidence = engine.resolve_graph_evidence(
    fathomdb.GraphEvidenceResolveRequestV1(
        evidence_ref=selected.target_evidence_ref,
        context=context,
    )
)
edge_evidence = engine.resolve_graph_evidence(
    fathomdb.GraphEvidenceResolveRequestV1(
        evidence_ref=selected.terminal_edge_evidence_ref,
        context=context,
    )
)
```

The sidecar is positional and covers both the returned target and the winning
terminal edge. Resolution rechecks the frozen authority and present visibility.
Do not persist or reinterpret opaque references, and do not substitute
non-frozen graph expansion if exact evidence is required.

When requested, both frozen search operations return a non-empty correlation
identity. With local telemetry enabled it is the matching `q...` query identity;
without telemetry it is an Engine-minted `x...` identity. Disabling explanation
returns no sidecar and does not capture telemetry on these frozen paths.

## Drift and retry

A frozen context has no elapsed-time expiry promise. It can be reused after a
clean restart while its bound database state remains available and unchanged.
Mutation may instead cause a frozen-read state-drift refusal. Evidence
resolution can return `evidence_unavailable` when the bound artifact is no
longer available or authorized.

For either outcome, restart the whole attempt: mint a new frozen context, rerun
`search_with_evidence`, and resolve only references returned by that new search.
Do not fall back to non-frozen search or re-search a hit's body.

## Current scope

Resolution returns one exact canonical source and its direct dependency when
registered. It does not return an arbitrary dependency chain. For graph-arm
hits, evidence identifies the final contributing edge rather than a complete
traversal path. Normal nondisclosure rules collapse unauthorized, stale,
foreign, or unavailable evidence to the same public outcome.
