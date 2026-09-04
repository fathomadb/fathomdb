---
title: 0.8.25 Slice 15 design review — cycle 0
status: COMPLETE
review_cycle: 0
reviewed_on: 2026-09-03
verdict: FAIL
---

# Slice 15 independent design review — cycle 0

## Verdict

**FAIL.** The scope and artifact/source revision split are sound, but the draft
is not executable against the current write, migration, and binding surfaces.

## Findings

| ID | Priority | Finding | Required correction |
| --- | --- | --- | --- |
| D15-01 | P1 | No exact public shape submits canonical versus derived provenance or returns Engine-minted revisions. | Define Rust and binding write inputs, receipt output, read access, and legacy-write behavior. |
| D15-02 | P1 | A non-null legacy `source_id` is only an erasure handle and cannot prove exact source bytes. | Mark every legacy row incomplete unless an explicit stored source revision exists; never fabricate completeness. |
| D15-03 | P1 | The proposed paged backfill does not fit the atomic migration runner and assumes a nonexistent database identity. | Use one additive schema step with no data backfill; derive stable incomplete legacy IDs without changing legacy rows. |
| D15-04 | P1 | Cross-table revision uniqueness, canonical artifact/source aliasing, and source-version scope are undefined. | Add one revision registry, define the allowed self-alias, and key source versions by source plus caller version. |
| D15-05 | P1 | Canonical bytes are ambiguous because TypeScript currently serializes object bodies. | Define the exact body accepted by provenance-bearing writes and hash only the resulting stored UTF-8 bytes. |
| D15-06 | P2 | Strict wire behavior and typed failure payloads are not bounded. | Apply strictness to the new versioned object only; define aliases, conflicts, reason codes, and field paths. |

The review was read-only and found no issue with preserving `IdSpace`, keeping
`SearchHit` compact, or limiting 0.8.25 to one canonical source per artifact.
