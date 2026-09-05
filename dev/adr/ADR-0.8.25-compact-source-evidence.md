---
title: ADR-0.8.25-compact-source-evidence
date: 2026-09-05
target_release: 0.8.25
status: accepted by approved 0.8.25 scope and Slice 50 execution authorization
supersedes: none
---

# ADR-0.8.25 — compact source-complete evidence

## Decision

FathomDB adds an opt-in evidence sidecar to frozen search and a separate
eligibility-bound resolver. Each search result is associated by position with
an Engine-authenticated, database-local `EvidenceRefV1`. Resolution returns
the exact one-source provenance already recorded by FathomDB: immutable
artifact revision, canonical source/version/revision, locator, exact UTF-8
source bytes and span, SHA-256, lifecycle state, projection origin, ranking
contribution, and an optional registered direct dependency.

The reference is bounded, stateless, content-free, and not authority. It uses
domain-separated HMAC commitments rather than plaintext identities or raw
identity hashes. Resolution requires an equivalent authenticated frozen
context and re-applies current artifact, source, graph-origin, lifecycle,
validity, eligibility, erasure, and dependency-closure constraints on one
reader transaction.

## Failure and disclosure boundary

Malformed, foreign, context-mismatched, missing, invisible, stale,
superseded, inactive, erased, or closure-fenced references return only
`evidence_unavailable` at `/evidenceRef`. FathomDB reports
`evidence_incomplete` or `evidence_corrupt` only after the reference and all
current authorization subjects have been validated. A graph-arm result
resolves the returned node's source and separately identifies only the exact
last edge that seeded or reached it; full graph-path replay is not promised.

## Compatibility and ownership

The two additive governed reads are `search_with_evidence` /
`searchWithEvidence` and `resolve_evidence` / `resolveEvidence`. Ordinary
search APIs and `SearchHit` remain unchanged. The feature adds no schema,
table, lease, retained snapshot, answer verifier, semantic judgment, or
multi-source provenance. Semantic citation policy remains outside FathomDB.

Retired projection-generation identity is retained and resolvable across
restart. Its outer-HMAC-authenticated selector uses a fresh per-reference
128-bit nonce and domain-separated HMAC-derived stream protection, then resolves
with a direct primary-key probe rather than a retained-history scan. Existing provenance,
frozen-read, dependency, lifecycle, and graph
mechanisms remain authoritative; this ADR defines their compact retrieval
composition.

The executable contract is
`dev/plans/0.8.25/features/slice-50/design.md`.
