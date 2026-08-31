---
title: FathomDB data-plane architecture v1 review
status: COMPLETE
verdict: APPROVED
target_release: 0.8.25
review_cycles: 2
---

# FathomDB data-plane architecture v1 review

## Scope

Independent read-only review of the proposed performance-program foldback
against the completed PROGRAM evidence, the current FathomDB contracts, the
Memex integration findings, and the 0.8.20–0.8.24 architecture history.

## Review record

| Cycle | Verdict | Findings and resolution |
| ---: | --- | --- |
| Initial | APPROVE WITH FIXES | Distinguish GLOBAL-01's storage-only witness from its `Engine.search` held-out path; frame graph direction as a combined-expansion gap; extend the existing filter grammar; keep ordinary hits compact; preserve single-source provenance. |
| FIX-1 | Re-review required | Architecture and plan were revised for the five findings. Review found one remaining P2: evidence resolution did not clearly distinguish an immutable Engine reference, a caller source-version identifier, and visibility under `ReadView`. |
| FIX-2 | APPROVE | Defined Engine-owned opaque `EvidenceRef`, explicit view-dependent resolution, superseded-visible versus not-visible outcomes, and the separate caller source-version field. No P1/P2 finding remained. |

## Approved architectural changes

1. Classify the two GLOBAL-01 paths and all future metrics by system layer.
2. Extend combined graph expansion with the controls direct graph reads already
   demonstrate; do not characterize graph traversal as wholly missing.
3. Extend the existing allowlisted filter grammar and native projections; do
   not replace it with a new predicate language.
4. Use an opt-in evidence resolver instead of adding source-complete payloads
   to every `SearchHit`.
5. Preserve the existing single-source provenance decision; multi-source
   causal provenance is separate future scope.

This approval grounds architecture and planning only. Each implementation
workstream still requires its own requirements, acceptance criteria, design
review, TDD RED/GREEN evidence, implementation review, and verification.
