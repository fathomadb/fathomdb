# Frozen evidence

Use frozen reads when ranking and evidence must share one authenticated view of
validity, eligibility, and database state.

## Choose the operation

| Need | Use |
| --- | --- |
| Frozen top-K ranking without source evidence | `freeze_read_context` → `search_frozen` |
| Evidence-backed answer (single hit) | `freeze_read_context` → `search_with_evidence` → `resolve_evidence` for one selected hit |
| Evidence-backed answer (batch up to 50 hits) | `freeze_read_context` → `search_with_evidence` → `resolve_evidence_many` / `resolveEvidenceMany` (`1..=50`, per-slot partial outcomes) |
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

### Batch resolution (`resolve_evidence_many` / `resolveEvidenceMany`)

To hydrate multiple hits from a `search_with_evidence` response in a single
snapshot lock, pass `1..=50` references (`EVIDENCE_RESOLVE_MANY_MAX = 50`) under
one `FrozenReadContextV1`:

```python
items = engine.resolve_evidence_many(
    fathomdb.EvidenceResolveManyRequestV1(
        evidence_refs=[entry.evidence_ref for entry in result.evidence],
        context=context,
    )
)
for item in items:
    if item.status == "resolved":
        print(item.resolved.canonical_source_body)
    else:
        print(item.reason, item.field_path)  # e.g. "evidence_unavailable", "/evidenceRefs/1"
```

Duplicate references preserve exact request order and arity. Invalid, tampered,
empty, oversized, wrong-context, erased, or superseded references return a
per-slot `refused` outcome (`reason="evidence_unavailable"`,
`field_path="/evidenceRefs/{i}"`) without failing valid slots. When a same-scope
edge write happens after freeze, node-only slots remain unaffected and resolve
while post-decode `Edge` or `GraphArm` slots whose edge partition drifted fail
closed per-slot with `evidence_unavailable`. Whole-context drift (such as a
same-`(scope, kind)` write, destructive mutation in `scope`, or global
structural change) fails the whole call with
`EvidenceError("evidence_unavailable")`.
Out-of-bounds lengths (`0` or `> 50`) fail before database access with
`InvalidArgumentError`. Opt-in search hints (`hints=True` / `hints: true`) are
also supported on `search`, `search_frozen`, and `EvidenceSearchRequestV1`,
populating `SearchResult.hints` without altering hit ranking.

## Trace registered dependencies

`trace_dependency` / `traceDependency` traverses the registered source-to-derived
relationship graph under the frozen context with explicit relation/work bounds.
It does not infer semantic relationships. A valid partial UTF-8 locator is
checked independently while the stored canonical digest authenticates the whole
source body; a digest of only the cited span is not a substitute. Forward and
reverse traces retain exact revision ownership and nondisclosure rules. Empty
locators may be accepted by writes/registration but are refused by trace's
nonempty-range policy. Unlike evidence resolution below, `trace_dependency` /
`traceDependency` enforces the full `ReadContextV1.eligibility` (including
`kind`, `source_type` / `sourceType`, and `property_predicates` /
`propertyPredicates`) symmetrically on both root and counterpart endpoints.

## Evidence eligibility and `GraphArm` source identity

When resolving evidence (`search_with_evidence`, `resolve_evidence`,
`resolve_evidence_many`, `graph.expand` with `include_evidence=True` /
`includeEvidence: true`, and `resolve_graph_evidence` / `resolveGraphEvidence`),
`ReadContextV1.eligibility` is tiered between the returned artifact and its
underlying canonical source:

- **Artifact-scoped selectors:** `kind`, `source_type` / `sourceType`, and
  `property_predicates` / `propertyPredicates` constrain the returned artifact
  only (they remain authenticated inside the frozen context digest, but the
  canonical raw source node does not need to carry derived-artifact properties
  such as extracted `confidence`).
- **Source-enforced guards:** Access-bearing `attributes`, `created_after` /
  `createdAfter`, `status`, `ReadView` validity and lifecycle, dependency
  closures, and Schema 39 `ScopePolicy` remain strictly enforced on both the
  returned artifact and its canonical source. Any mismatch fails closed with
  non-disclosing `evidence_unavailable`.

For `GraphArm` search hits (`branch == "graph_arm"`), `SearchHit.source_id` /
`sourceId` intentionally reports the traversed edge's legacy `source_id`, while
`ResolvedEvidenceV1.source_id` / `sourceId` resolves the returned node
artifact's own canonical provenance source (and `projection_origin.graph_origin`
/ `projectionOrigin.graphOrigin` separately identifies the terminal edge
revision). Do not treat equality between `SearchHit.source_id` and
`ResolvedEvidenceV1.source_id` as an invariant on `GraphArm` hits.

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

Graph evidence uses actual start-inclusive, end-exclusive validity for the
target, winning edge, and canonical source. It refuses a frozen context whose
view enables `includeOutOfWindow`; mint a normal frozen context instead. An
affecting database mutation after the context was minted (see
[Partitioned additive stability (Schema 39)](#partitioned-additive-stability-schema-39)
below for which writes drift a graph surface and which disjoint writes stay
stable) is reported as frozen-state drift before provenance detail is inspected.
Rerun the whole freeze, expand, and resolve sequence after any such drift.

Historical `graph.expand` requests (`as_of_write_cursor` / `asOfWriteCursor` or
`as_of_epoch_ms` / `asOfEpochMs`) also support `include_evidence=True` /
`includeEvidence: true` under frozen authority. The engine authenticates the
`as_of` bound inside the request commitment, verifies that both the artifact
and its canonical source existed at or before the resolved cursor, and mints
the same 704-character `fdbgev1.` references. When the historical revision was
later superseded in the frozen snapshot, `resolve_graph_evidence` /
`resolveGraphEvidence` succeeds for retained active revisions and reports
`artifact_lifecycle.superseded == True` (`artifactLifecycle.superseded === true`).
Physical erasure (`purge`, `erase_source` / `eraseSource`), transitioning an
artifact's active head out of `active`, or an active dependency-closure barrier
revokes both minting and resolution with `evidence_unavailable`.

When requested, both frozen search operations return a non-empty correlation
identity. With local telemetry enabled it is the matching `q...` query identity;
without telemetry it is an Engine-minted `x...` identity. Disabling explanation
(`include_explanation=False` / `includeExplanation: false`) returns no sidecar,
does not capture telemetry on these frozen paths, and skips full `explain=True`
vector-cosine, dropped-edge, and projection-status overhead while still
populating lightweight `EvidenceContributionV1` metadata.

## Drift and retry

A frozen context has no elapsed-time expiry promise. It can be reused after a
clean restart while its bound database state remains available and unchanged.

### Partitioned additive stability (Schema 39)

Schema 39 partitions frozen-read visibility watermarks by `(scope, partition_kind)`
so concurrent additive activity outside a frozen context's read surface does not
force spurious retries:

- **Stable (zero drift):**
  - Additive writes or actuations to a disjoint node `kind` when the frozen
    context specifies `eligibility.kind`.
  - Appends to `append_only_log` operational collections (`operational_mutations`)
    against every frozen surface, including `read.operational_state`.
  - Upserts to `latest_state` operational collections (`operational_state`)
    against canonical, search, evidence, graph, and trace surfaces (including
    unfiltered `SearchFilter()`), while still drifting `read.operational_state`.
  - Additive writes, lifecycle transitions, purges, and source erasures in a
    disjoint caller `scope` (including across close and reopen).
  - Background vector projection commits for a disjoint `(scope, kind)`.
- **Fail-closed drift (`StateDrifted` / `evidence_unavailable`):**
  - Any write to the same `(scope, kind)` (or any node write in `scope` when
    `eligibility.kind` is omitted).
  - Same-scope edge writes against graph surfaces (`graph.expand`,
    `search_expand_frozen`, `resolve_graph_evidence`, `use_graph_arm=True`,
    `trace_dependency`) or matching-`kind` edge writes; in
    `resolve_evidence_many`, affected `Edge` / `GraphArm` slots refuse per-slot
    while unaffected node-only slots still resolve.
  - Any destructive mutation (`transition`, `purge`, `erase_source`,
    `excise_source`) on any kind in the same `scope`, any dependency or closure
    mutation in `scope`, or any global structural change (projection
    registration/enrolment, embedder `mean_vec` pin, or uncoalesced SQL on
    serving tables).
- **FTS5 BM25 corpus statistics:** Additive writes to disjoint kinds or scopes
  share the underlying FTS5 `search_index` table. While frozen-context
  membership, eligibility, and resolved canonical evidence bytes remain strictly
  frozen across those additive writes, raw FTS5 `bm25()` scores and relative
  score tie-breaks can shift slightly as corpus document frequency updates.

When a mutation does cause a frozen-read state-drift refusal or
`evidence_unavailable`, restart the whole attempt: mint a new frozen context,
rerun `search_with_evidence`, and resolve only references returned by that new
search. Do not fall back to non-frozen search or re-search a hit's body.

## Current scope

Resolution returns one exact canonical source and its direct dependency when
registered. It does not return an arbitrary dependency chain. For graph-arm
hits, evidence identifies the final contributing edge rather than a complete
traversal path. Normal nondisclosure rules collapse unauthorized, stale,
foreign, or unavailable evidence to the same public outcome.
