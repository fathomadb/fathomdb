---
title: FathomDB 0.8.26 Slice 10 — frozen evidence repair design
status: APPROVED
reviewed_on: 2026-09-13
review_result: independent PASS after one focused clarification cycle
---

# Slice 10 design — frozen evidence repair

## Existing machinery and the required delta

| Surface | Existing behavior | Slice 10 delta |
| --- | --- | --- |
| shared search core | assembles ranked results and optional explanation | none |
| ordinary explained search | runs `finalize_search_observability` before return | none; reuse its finalizer |
| `search_frozen` | returns before explanation correlation finalization | finalize only when the returned explanation is present |
| `search_with_evidence` | adds evidence references, then returns before finalization | finalize the nested search result only when its explanation is present |
| Python and TypeScript mapping | rejects an empty explanation correlation identity | regression coverage; no mapper or public-shape change expected |
| release package witness | verifies a fresh, non-editable wheel | add one focused, source-independent frozen-evidence profile |

The defect is a response-completion omission, not a scoring, eligibility,
evidence-reference, binding, or persistence defect. Both affected Engine paths
must call the existing finalizer after all successful result assembly and
before returning the public result.

## Completion boundary and observability

For each affected call:

1. Complete the reader transaction, authority checks, search-result assembly,
   and—where applicable—evidence-sidecar assembly.
2. Populate the explanation embedder identity as today.
3. If and only if the returned result contains an explanation, invoke
   `finalize_search_observability` exactly once.
4. Return the result.

The conditional is important. The existing finalizer captures telemetry when
no explanation is present, while explanation-disabled frozen operations are
currently telemetry-silent. Slice 10 preserves that behavior. With telemetry
enabled, finalization assigns the existing `q...` correlation identity; with
telemetry disabled, it assigns the existing unique `x...` fallback identity.
Failed operations are not finalized, and evidence search is finalized only
after its complete sidecar has been assembled successfully.

No ranking, score, hit identity, projection cursor/fallback, eligibility,
frozen-context authority, evidence-reference format, or persisted state is
changed. The fix also does not manufacture ordinary-search lifecycle events or
new counters.

## Equivalence oracle

Regression tests use paired, equivalent frozen contexts and normalize away
only the explicitly requested explanation field. They compare:

- complete ordered hits, scores, identities, cursor, and fallback state;
- positional evidence artifact-revision identities, not opaque evidence
  reference bytes, because each reference includes fresh randomness; and
- independently resolved canonical bytes/span, locator, lifecycle,
  dependency, projection origin, and ranking contribution.

Separate explained calls are expected to receive distinct correlation
identities. Exact string equality between their identities is not an oracle.

## Public frozen-evidence guidance

The new guide and maintained interface references use this decision table:

| Consumer need | Supported path |
| --- | --- |
| frozen top-K ranking without evidence exposure | `freeze_read_context` → `search_frozen` |
| evidence-backed answer | `freeze_read_context` → `search_with_evidence` → `resolve_evidence` for selected positional hits |
| canonical collection pagination | `canonical_page`; search remains top-K, not a page stream |
| evidence for an exact graph target | Slice 20 point-resolution capability; never body or logical-ID re-search |

The guide states that an evidence reference locates governed evidence but is
not authority by itself. The frozen context remains restart-portable only
while its bound state remains available and unchanged; there is no elapsed-time
expiry promise. Mutation may produce frozen-state drift. Evidence resolution
may instead return `EvidenceError(evidence_unavailable)` when its bound
artifact is unavailable. Either condition requires restarting the whole
attempt: obtain a new frozen context, rerun evidence search, and resolve only
new references. Non-frozen search is never a fallback.

The guide also records current scope: one direct dependency, nondisclosure
behavior, and graph-arm evidence that identifies only the exact final
contributing edge rather than a complete path. Graph-target point lookup
remains Slice 20. The guide does not pull actuation or future graph-edge design
into Slice 10.

## Installed-wheel witness

`scripts/verify-release-python-wheel.sh` remains the package boundary. A
source-independent profile under `scripts/release/smoke/` is invoked with
`PYTHONPATH` unset from an external working directory after the freshly built
wheel is installed. The profile:

1. proves Python and native-module import provenance is inside the fresh
   environment;
2. seeds the minimal canonical source, derived artifact, and direct dependency;
3. freezes context, exercises both explained operations and their disabled
   controls, resolves selected evidence exactly, and retains the frozen context
   and one evidence reference;
4. closes and reopens the unchanged database, consumes the retained context in
   an explained frozen query, resolves the retained reference, and verifies the
   current direct dependency; and
5. closes cleanly and exits successfully.

This is deliberately not the combined P0–P2 release profile. Actuation replay,
semantic edge authoring, graph point lookup, projection readiness, broad
pagination, and the integrated consumer profile remain in their assigned later
slices, with Slice 50 owning the combined installed-artifact gate.

## Verification boundary

Focused Rust Engine tests establish finalization and telemetry behavior.
Python and TypeScript tests exercise their real public calls and strict
mappers without duplicating Engine telemetry internals. The guide's canonical
Python flow is mirrored and exercised by the source-independent installed-
wheel profile, while the strict MkDocs build validates navigation and
references. The verifier fixture proves the external profile cannot be
skipped, followed by one real fresh-wheel run.

Because the explanation-disabled hot path gains only a presence check and the
enabled path reuses an existing finalizer, Slice 10 does not add a performance
spike or run the long platform/performance matrix. It runs the focused blast
radius, `agent-verify`, and full-workspace clippy/check. Any observed ranking,
eligibility, disabled-path telemetry, or persisted-format change is a stop
condition rather than accepted scope.
