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

## Test and code review cycles

- **Test review:** PASS (T-1 fixed by this record).
- **Pre-existing failures fixed:** release-profile and default-feature engine
  test builds (19 targets) now compile warning-free; `agent-typecheck.sh`
  gates default, release, and release `test-hooks,operator` test builds.
- **Code review:** four FIX cycles hardened the AC-050c removal gate
  (`scripts/security/check_removal_changelog.py`): cfg-gated re-exports never
  cancel removals, removed `pub use` names count, whole-file diff context,
  per-side cfg-aware parsing, owner-keyed items, `--no-renames`. The symmetric
  gate surfaced 29 genuine 0.8.0 operator-seam default-build removals, now in
  the CHANGELOG 0.8.0 Removed section.
- **Open at cycle cap (Y-1, P2):** FIX-4's same-crate move cancellation matches
  bare names without public reachability, so a public item moved into a
  private module without a root `pub use`, or deleted while an unrelated
  same-named item exists in another diffed file, passes silently (the pre-FIX-4
  gate caught these). Also Y-2 (P3: `examples/`, `benches/`, `build.rs` share
  the crate key) and Y-3 (P3: associated items keyed by last type segment).
  Awaiting HITL direction.
