---
title: 0.8.25 Slice 50 — compact source-complete evidence
status: DESIGN_REVIEW_PENDING
depends_on: 45
design: design.md
design_status: DRAFT_RECONCILED_REVIEW_PENDING
---

# Slice 50 plan

## Outcome and boundary

Implement S50-R1 through S50-R7 from the
[`design`](design.md): an opt-in, stateless, one-source evidence sidecar for
frozen search and an eligibility-bound resolver that returns the exact
canonical source revision and selected UTF-8 evidence span. Keep ordinary
`SearchHit` and every existing search entry point unchanged.

Persisted evidence leases/receipts, multi-source provenance, narrower-context
authorization, replayable retention, semantic entailment, answer citations,
and graph-path replay remain outside 0.8.25.

## Delivery sequence

1. Reconcile design v4 against implemented Slices 15–45, architecture v2,
   R25/AC25-50, the approved scope adjustment, and current SDK/wire rules.
   Obtain independent design review; allow at most four documented FIX-n
   cycles. Unresolved implementation-shaping P1/P2 findings block READY.
2. Add a successor evidence ADR and update Rust/Python/TypeScript/wire
   interfaces with the exact additive contract and non-disclosure precedence.
3. Commit failing real-database, codec/property, lifecycle, concurrency,
   privacy, default-path, wire, and binding tests before product implementation.
   Preserve the RED commit and verbatim first diagnostics.
4. Implement the content-free reference codec, same-snapshot evidence search,
   exact resolver, and Rust facade. Then add Python and TypeScript parity. No
   migration or schema-version increment is expected; stop and review if
   implementation proves persistent state necessary.
5. Obtain independent code review at the exact implementation commit. Allow at
   most seven documented FIX-n cycles; each product correction receives a
   focused regression and re-review.
6. Give a separate read-only verifier the exact commit, requirements matrix,
   and commands. Run focused, fast, heavy, all/applicable-feature,
   source-independent fresh-wheel/npm, and Windows native routes.
7. Write `status.md`, update design/interface successor pointers and release
   state through its JSON authority, regenerate views, run final clean gates,
   commit, and push the explicit release branch.

## Acceptance and verification

The slice closes only when all seven S50 acceptance rows pass and independent
review has no unresolved P1/P2 finding. Verification must prove:

- result-index and immutable-revision sidecar association is exact and atomic;
- the reference contains no query, body/span, source/logical/natural identity,
  locator text, or caller attribute value;
- current and valid-as-of sources resolve exact whole-source and selected-span
  bytes with a recomputed SHA-256 match;
- malformed, foreign, context-mismatched, invisible, superseded, inactive,
  erased, closure-fenced, or stale references disclose only one typed
  `evidence_unavailable` result;
- authenticated and currently authorized corruption or incomplete provenance
  returns only its documented typed outcome;
- restart and an equivalently reminted context work when authority remains
  valid, while narrower or broader envelopes reject;
- default search shapes, serialization, SQL paths, database/WAL bytes, and
  behavior remain unchanged; and
- Rust, Python, TypeScript, Linux packages, and Windows native artifacts agree.

## Stop gates

Stop on stale-reference disclosure; plaintext private identity in a reference;
provenance lookup outside the search snapshot; any partial sidecar; hidden
contribution truncation; mandatory expansion of every hit; a new persistent
lease/receipt or schema table; default-search hot-path change; public-surface
drift without parity; or test-oracle relaxation.

## Selected routes

Selected: focused Rust/schema-free migration guard, codec properties, fast,
heavy, all, applicable all-feature, Windows CPU/native Rust/Python/Node, fresh
Linux wheel, offline packed npm/native, and privacy scans. Operator, CUDA/GPU,
live-model, and registry publication routes are N/A: evidence resolution is a
CPU storage/visibility operation and changes no dense/rerank dispatch.
