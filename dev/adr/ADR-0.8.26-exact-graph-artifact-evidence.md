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

The public reference grammar is exactly `fdbgev1.` plus 696 lowercase
hexadecimal characters encoding a 16-byte nonce, encrypted 300-byte selector,
and 32-byte MAC. The selector commits independently to database, semantic frozen
context, normalized request, target and predecessor identities, terminal edge
kind, and both revision identities. Encryption uses ten counter-separated HMAC
stream blocks; stream, MAC, and every commitment have distinct domains.

Evidence hydration retains the already-selected terminal edge cursor. Parallel
edges therefore use the traversal's existing ascending incident-edge tuple; no
second edge selection occurs during hydration. Nonempty results issue exactly
two bounded hydration statements. Nullable ordinal-preserving joins allow a
global authorization phase over target and edge rows before any locator, hash,
or source bytes are interpreted. Eligibility predicates, including source
metadata and attributes, are compiled into those statements rather than issued
as helper queries.

Evidence always applies start-inclusive, end-exclusive validity to targets,
winning edges, and canonical sources. An authenticated frozen context with
`includeOutOfWindow=true` is refused at
`/context/context/view/includeOutOfWindow`. Snapshot validation remains first:
storage mutation after context mint returns frozen-read `state_drifted`; detailed
provenance precedence is observable only for state authenticated by the supplied
frozen context.

## Consequences

- Memex can cite the exact graph artifacts selected by one frozen traversal.
- Empty evidence-bearing results need no hydration query; nonempty results use
  exactly two bounded class-specific hydration statements.
- `GraphTargetV1.writeCursor` remains public and unchanged; evidence sidecars
  and resolved evidence expose immutable revision identity, not mutable cursors.
- No schema, durable evidence table, raw-ID resolver, batch resolver, migration,
  or parallel V2 API is introduced.
- Current-context graph evidence is deliberately refused rather than weakened.

## Alternatives rejected

- Re-searching target text: probabilistic and not exact.
- Raw revision/logical-ID lookup: insufficiently bound to disclosure and
  eligibility.
- Reusing ranked `resolve_evidence`: would falsely require ranking contribution
  and projection-origin claims for graph-selected artifacts.
