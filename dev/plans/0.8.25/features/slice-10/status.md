---
title: 0.8.25 Slice 10 status
status: COMPLETE_ON_RELEASE_BRANCH
completed: 2026-09-03
implementation_review: PASS_CYCLE_3
verification: PASS
---

# Slice 10 status

## Outcome

Slice 10 is complete on `release/0.8.25`. The experiment harness now enforces
source-bound measurement layers, exact component ownership, witnessed
`Engine.search` execution, derived comparison arms, immutable classification
sidecars, historical prefix closure, and pre-index receipt finalization. It
changes no Engine, SDK, wire, or database surface.

## TDD and review

- Initial RED: `c57682a8`, `caa3408d`; GREEN: `35088047`.
- FIX-1 RED: `435b2f6d`; GREEN: `2d465657`.
- FIX-2 RED: `29c15862`; GREEN: `0d5044f4`.
- FIX-3 RED: `9a2d1a98`; GREEN: `dd736cda`.
- Independent design review passed after FIX-3; design v6 confirmation passed
  without another design FIX.
- Independent implementation review passed at cycle 3 with no P1/P2 findings.

## Receipts

- Historical GLOBAL-01 decision runs have v2 classifications and a closed
  inclusion/exclusion manifest. The original comparison remains
  `unknown_historical`; the held-out treatment records exact lower-bound search
  witnesses without changing its end-to-end answer metrics.
- Provisional native v1 run
  `measurement-classification-native-search-20260903T2255Z-b5f420c4` remains
  byte-original at its canonical path, is hash-pinned by policy, and is
  mechanically ineligible as evidence.
- Final successful witness
  `measurement-classification-native-search-20260903T2332Z-851dbebe` binds
  implementation `dd736cda`, executes exactly one native `Engine.search`, and
  returns the expected source at rank 1.
- Blocked continuation witness
  `measurement-classification-native-search-20260903T2332Z-ab554c17` records
  `expected_hit_missing`, validates as blocked, and cannot support a claim.

## Verification

- Focused classifier/library/receipt tests: 59 passed.
- Portable repository classification: PASS.
- Historical raw-evidence audit through `data`: PASS.
- Commission manifest: PASS; Slice 10 selected 17 paths and 20 anchors.
- `agent-verify.sh --tier=fast`: 103/103 suites passed, no skips.
- Heavy tier: 2/3 suites passed, one explicit TypeScript skip because
  `src/ts/node_modules` is not installed; zero failures.
- Independent verifier: PASS at `f383ec82`, with no P1/P2 defects.

The additional `AGENT_LONG=1 ./scripts/check.sh` diagnostic entered the
release-wide canonical performance fixtures and exposed the pre-existing
`ac_013_vector_retrieval_latency` failure. It was stopped after that failure;
it is unrelated to Slice 10's evaluation-only Python/docs surface and remains
release-level performance debt.
