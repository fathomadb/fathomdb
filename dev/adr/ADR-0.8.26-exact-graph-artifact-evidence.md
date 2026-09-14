---
title: Exact graph artifact evidence
date: 2026-09-14
target_release: 0.8.26
status: accepted
decision: D26-01 option A, HITL seq-290
---

# Exact graph artifact evidence

## Context

Constrained graph expansion returns deterministic targets and compact origins,
but 0.8.25 cannot resolve the exact target revision or winning terminal edge to
canonical evidence. Re-searching target text is probabilistic and top-K-bound;
raw stable-ID resolution would bypass the eligibility decision made by the
frozen graph read.

## Decision

Add an opt-in `include_evidence` member to `GraphExpandRequestV1`. It is valid
only with an authenticated frozen context. A successful response carries one
positional sidecar entry per target with immutable target and terminal-edge
revision identities plus distinct opaque references.

Add `resolve_graph_evidence` / `resolveGraphEvidence`. It accepts only an opaque
graph reference and its equivalent frozen context. Resolution reauthenticates
the database and context, rechecks artifact and canonical-source visibility,
and returns the exact node or edge plus canonical bytes, locator, hash,
lifecycle, and an optional direct dependency. It makes no ranking claim.

References are database-bound, context-bound, request/disclosure-bound,
tamper-evident, and confidentiality-protected. Invalid, foreign, stale, or
unauthorized references fail nondisclosingly. Ordinary expansion request and
response bytes remain unchanged when the option is omitted or false.

## Consequences

- Memex can cite the exact graph artifacts selected by one frozen traversal.
- Empty evidence-bearing results need no hydration query; nonempty results use
  exactly two bounded class-specific hydration statements.
- No schema, durable evidence table, raw-ID resolver, batch resolver, migration,
  or parallel V2 API is introduced.
- Current-context graph evidence is deliberately refused rather than weakened.

## Alternatives rejected

- Re-searching target text: probabilistic and not exact.
- Raw revision/logical-ID lookup: insufficiently bound to disclosure and
  eligibility.
- Reusing ranked `resolve_evidence`: would falsely require ranking contribution
  and projection-origin claims for graph-selected artifacts.
