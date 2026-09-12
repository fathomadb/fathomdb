---
title: FathomDB 0.8.26 Slice 20 — exact graph artifact evidence design
status: DRAFT
---

# Slice 20 design — exact graph artifact evidence

## Proposed public shape

Add immutable artifact-revision identity to graph output for the target and,
when present, the terminal edge. Add a frozen point operation provisionally
named `resolve_artifact_evidence(context, artifact_revision)`.

The response should be a versioned intrinsic `ResolvedArtifactEvidenceV1`, not
the ranked-search response with invented values. It contains only facts the
artifact and frozen authority establish: artifact and logical identity, kind,
canonical bytes/span or locator, source identity/version, hash, lifecycle,
dependency, and projection origin where applicable. Ranked contribution
remains exclusive to `resolve_evidence` handles produced by ranked search.

Exact names and additive-versus-successor graph response shape remain Slice 6
decisions.

## Transaction and disclosure model

One reader transaction validates the frozen context, finds the exact immutable
revision, evaluates eligibility/authorization, and materializes the response.
Nonexistent, ineligible, unauthorized, and outside-boundary revisions use the
same privacy-safe refusal family. The method never falls back to current state,
logical ID, body search, or a new snapshot.

## Graph semantics

The target revision proves the target artifact. The terminal-edge revision
proves the last traversed edge when one exists. Neither represents the ordered
path. Full path evidence and graph continuation remain outside 0.8.26.

## Persistence

Existing artifact revision and provenance storage should suffice. Any required
schema migration stops this slice for explicit release-scope reconsideration.
