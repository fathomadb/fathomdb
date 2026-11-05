---
title: FathomDB 0.8.28 Slice 50 — requirements and acceptance criteria
status: complete; implemented and verified
target_release: 0.8.28
observed_on: 2026-11-01
---

# Slice 50 requirements and acceptance criteria

The [plan](plan.md) reconciles the draft with baseline `630af0bfc`.
Provider requirements are complete under the
[provider record](built-in-provider-implementation-record.md). This document
specifies the remaining scan Unit S; obsolete refused-handle requirements
D50-1/D50-2 are unselected and do not gate completion.

## Requirements

- **REQ50-S1:** each scan arm contributes at most 256 predicate-matching
  rows to the outer sort, independent of the pending backlog. Rejected
  index entries are outside this bound.
- **REQ50-S2:** both scans return exactly the baseline ordered page for
  valid stores, at every cursor boundary, candidate through-bound and fixed
  validity instant. Valid stores have globally unique write cursors.
- **REQ50-S3:** preserve predicates, shared edge fragment, joins, generation
  singleton, page size, Rust eligibility, in-flight skipping and rewind.
  No schema, index, public production API or runtime toggle changes.
- **REQ50-S4:** projected rows, terminals, cursor, readiness and pending
  counts retain the baseline semantics.
- **REQ50-S5:** qualify held-drain scaling from 10k to 100k on format 34,
  current default cache and deterministic 768-d vectors using the previously
  declared rule. Report a failure without relabeling it a pass.
- **REQ50-S6:** document the ordered per-arm limit invariant in code; verify
  indexed streaming and bounded page cost on the shipped SQLite.
- **REQ50-R1:** commit re-runnable measurement tools; record outcomes,
  independent review, limitations and release state. Generated run data stay
  outside tracked source; compact results belong in status.

## Acceptance criteria

These are slice-local identifiers, not additions to locked `dev/acceptance.md`.

| AC | Required evidence |
| --- | --- |
| AC50-01 | Both production queries have bounded arm co-routines, range-index reads and no arm-local sort; plan test fails before the change. |
| AC50-02 | VM-step ratio at 16,000 nodes/4,000 edges versus 2,000/500 is ≤1.5 for both scans; this fails before bounds. |
| AC50-03 | Property checks compare both production pages to frozen baseline SQL across mixed eligibility states, terminals, cursor and through bounds. |
| AC50-04 | Mutating a bound to 1 or an arm order to DESC makes equivalence checks fail; restore before committing. |
| AC50-05 | Edge-first, node-first and interleaved layouts spanning multiple pages drain to the expected eligible vector set and terminal/readiness state. |
| AC50-06 | Affected drain, readiness, projection completion, generation and rewind suites pass. |
| AC50-07 | Paired scaling runs yield identical logical projected-store digests, excluding nondeterministic timestamps. |
| AC50-08 | Head median microseconds/vector ratio 100k:10k ≤1.3; head 10k median ≤1.05 × base. Report 100k speedup separately. |
| AC50-12 | One final full agent verification run; scoped correction reruns only, with truthful failures/limitations. |
| AC50-13 | Markdown and release-state validators pass. |
| AC50-14 | Committed scaling tool runs against current base and head. |

AC50-09–11 concerned obsolete refused-handle behavior and are retired by the
completed failed-open contract. Format 35, vec0 eligibility, AC-013 latency,
provider-free rebuild remediation and later release work are out of scope.
