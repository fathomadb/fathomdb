# Slice 40 post-closeout adversarial review

Reviewed range `5f5c1798..e3358800` on 2026-09-22 (requirements/design, then
test review). Verdict: code move verbatim and behavior-preserving; findings
below remediated on `review/0.8.27-slice40`.

## Hidden-items surface evidence (AC27-40C)

`cargo public-api` omits `#[doc(hidden)]` items, so the Slice 30 comparator
rows contain none of the moved hooks. The reviewer instead generated rustdoc
JSON with `--document-hidden-items` (nightly-2026-04-24) for the pre
(`5f5c1798`) and post (`e3358800`) trees under default, `test-hooks`,
`slice72-test-hooks`, `operator`, and `operator,test-hooks`, and compared the
root-reachable name sets: identical in all five. The JSON was not retained;
re-run the same command pair to reproduce.

## Findings and remediation

- **R-1 (P2)** comparator blind to hidden items: AC27-40C wording corrected;
  hidden-items oracle for later slices tracked as a todos-ledger consideration.
- **R-2 (P3)** release default build warned on the unused `use super::*` in
  `test_hooks.rs`: glob gated on `any(debug_assertions, feature = "test-hooks")`.
- **R-3 (P3)** plan goal "default builds exclude test hooks" narrowed without a
  trace: recorded as a todos-ledger consideration for HITL ruling.
- **T-1 (P3)** the hidden-items claim had no committed record: this file.
- **Pre-existing build breaks (found during review, not introduced by Slice
  40):** `cargo check --release -p fathomdb-engine --lib --tests` and default
  feature `--all-targets` did not compile. Remediated in the same branch.
