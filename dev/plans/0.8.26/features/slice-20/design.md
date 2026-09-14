---
title: FathomDB 0.8.26 Slice 20 — exact graph artifact evidence design
status: DRAFT
---

# Slice 20 design — exact graph artifact evidence

## Proposed public shape

Add immutable artifact-revision identity to graph output for the target and,
when present, the terminal edge through the V1 shape selected by D26-01. Stable
identity supports joining but is not authorization. If the sidecar is selected,
it also carries authenticated opaque references bound to the frozen graph
disclosure; resolution accepts a reference plus equivalent frozen context,
never an arbitrary raw revision ID.

The response should be a versioned intrinsic `ResolvedArtifactEvidenceV1`, not
the ranked-search response with invented values. It contains only facts the
artifact and frozen authority establish: artifact class and logical identity,
kind/body or edge endpoints, canonical bytes/span or locator, source
identity/version, hash, lifecycle, and direct dependency when applicable. It
does not fabricate ranking contribution, query score, projection generation,
or projection origin. Ranked contribution remains exclusive to
`resolve_evidence` handles produced by ranked search.

Exact names and the in-place-result-versus-first-generation-V1-sidecar shape
remain open pending Slice 15 evidence and a subsequent D26-01 HITL ruling.
`seq-283` excludes a parallel V2 result or graph method.

## Transaction and disclosure model

Evidence-bearing graph expansion requires a frozen context. One primary-
connection transaction validates the equivalent frozen context and the
authenticated disclosure reference, finds the exact immutable revision,
evaluates eligibility/authorization, and materializes the response.
Nonexistent, ineligible, unauthorized, and outside-boundary revisions use the
same privacy-safe refusal family. The method never falls back to current state,
logical ID, body search, or a new snapshot.

Target resolution applies frozen target-node eligibility. Terminal-edge
resolution rechecks lifecycle/source access and the graph request's committed
edge selection; it must not apply the target-oriented `SearchFilter.kind` as an
edge-kind rule. Evidence-bearing expansion refuses as a whole if any selected
target or terminal edge has incomplete provenance.

## Graph semantics

The target revision proves the target artifact. The terminal-edge revision
proves the last traversed edge when one exists. Neither represents the ordered
path. Full path evidence and graph continuation remain outside 0.8.26.

## Persistence

Existing artifact revision and provenance storage should suffice. Any required
schema migration stops this slice for explicit release-scope reconsideration.
