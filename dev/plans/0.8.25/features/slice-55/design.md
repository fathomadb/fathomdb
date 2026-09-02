---
title: 0.8.25 Slice 55 — basic tracing, explanation, and integrity design
status: DRAFT_SCOPE_RECONCILED_BLOCKED_ON_SLICE_7
design_version: 5
target_release: 0.8.25
depends_on: 50
architecture: dev/design/fathomdb-data-plane-architecture-v2.md
---

# Slice 55 design

## Authority and boundary

Implements the retained subset of R25/AC25-55, Memex needs 19/20 and its share
of 12, N25-02, and A25-05/A25-06. It explains structural provenance and
operational integrity, never semantic truth, entailment, relevance, or answer
quality.

Persisted trace pages, expanded exclusion/not-selected explanation, frozen
integrity jobs, reverse-index rebuild orchestration, and repair planning are
allocated to the post-0.8.25 notes.

## Reciprocal trace contract

```text
TraceDirection = to_source | to_dependents
TraceRequestV1 {
  schema_version: 1, root_revision_id, direction,
  limit: 1..100, context?
}
TraceResultV1 {
  schema_version: 1, root_revision_id, direction,
  nodes, dependency_edges, complete: true, read_boundary
}
```

For a derived root, `to_source` returns zero or one active Slice 20 relation
and exact source revision. For a canonical source root, `to_dependents`
returns at most 100 directly derived revisions in stable revision/dependency-ID
order. If more than the requested or hard limit exist, the call fails
`trace_bound_exceeded`; it does not return a partial trace or continuation.
Eligibility, current access, lifecycle, erasure, and closure fences apply
before collecting nodes. No source bytes are returned; callers use Slice 50
after authorization when exact evidence is required.

## Compact explanation

```text
InclusionState = included | degraded
StructuralExplanationV1 {
  schema_version: 1, operation_id, correlation_id,
  artifact_revision_id, inclusion_state,
  retrieval_contributions, projection_origins,
  dependency_state?, lifecycle_state?, fallback?, degradation_codes
}
```

Explanation is opt-in. It reports why a returned result was included and
whether execution degraded or fell back. It does not answer why an arbitrary
non-result was excluded, expose rejected candidates, or claim semantic
correctness. Correlation IDs are Engine-generated and content-free. Existing
0.8.8 explanation/telemetry remains compatible and supplies contribution data.

## Bounded integrity checks

```text
IntegrityCheckKind = dependency_endpoints | dependency_reciprocity |
  active_searchable_orphans | projection_generation | mutation_readiness
IntegrityCheckRequestV1 { schema_version: 1, checks, max_findings: 1..100 }
IntegrityFindingV1 {
  schema_version: 1, code, severity,
  artifact_revision_ids, dependency_id?, projection_generation_id?
}
IntegrityCheckResultV1 {
  schema_version: 1, read_boundary, checked_count,
  findings, complete: true
}
```

`check_integrity` is a synchronous, bounded, read-only Engine/doctor operation.
It checks Slice 20 endpoint/forward-reverse consistency, Slice 30 active or
searchable orphans, and Slice 40 generation/readiness invariants at one read
boundary. If work or findings exceed the declared bound, or state changes
during the check, it returns `integrity_bound_exceeded` or
`integrity_read_drifted` and no passing result. It creates no job, cursor,
repair plan, projection rebuild, lifecycle mutation, or shadow index.

## Privacy, compatibility, and verification

Trace/explanation/integrity storage is ephemeral. Responses and telemetry
contain opaque revision/dependency/generation IDs and closed reason codes, not
query text, predicates, source bytes, payloads, or low-entropy natural keys.
Current access and erasure always win. All types follow Slice 15 wire/SDK
rules.

RED/GREEN tests cover reciprocal traces, overflow, visibility and erasure,
compact inclusion/degradation reasons, injected missing endpoint/reverse row,
active/searchable orphan, projection generation/readiness mismatch, drift,
privacy/WAL scans, close/reopen, and cross-SDK equality. Tests prove no repair
or mutation occurs and semantic correctness is never asserted.

Run fast, heavy, all/all-feature/operator, Windows Rust/Python/Node, and locally
packed explanation/doctor routes. CUDA, live-model, and pre-publication
registry routes are N/A. A formal independent READY review remains required
after Slice 7 and Slice 50 complete.
