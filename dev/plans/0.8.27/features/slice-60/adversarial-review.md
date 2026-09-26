---
title: FathomDB 0.8.27 Slice 60 - post-closeout adversarial review
status: IN_PROGRESS
reviewed_on: 2026-09-25
reviewed_range: 517e0545..792aa6c9
---

# Slice 60 post-closeout adversarial review

This review was requested after Slice 60 closed on `release/0.8.27`. It checks
requirements and design first, then tests, then implementation. No Steward or
Orchestrator role is used. Findings are resolved within these limits: three
design FIX cycles, three test FIX cycles, and four code FIX cycles.

## Design review

### Cycle 1 - FAIL

The required first review used `gpt-5.6-sol` at high reasoning, read-only.

| ID | Severity | Finding | Design FIX-1 |
| --- | --- | --- | --- |
| D1 | P1 | AC27-60B and the master plan claimed commit-boundary coverage, but `pre_tx_hook` runs before `BEGIN`; neither real `tx.commit()` exit had a refusal oracle. | Add a `test-hooks`-only, one-shot writer commit-abort seam shared by the trigger-suppressed and row-trigger commit exits. Require full-snapshot and next-cursor cases for both paths, with non-vacuity mutants. |
| D2 | P2 | AC27-60A required the modules to own exactly the approved inventory, while the design moved `legacy_revision_id` through a forbidden `pub(crate) legacy_*` seam and the implementation returned it to root to satisfy AC-050a. | Amend the inventory and acceptance criterion with an explicit private-root exception; use `revision_hash_field` as the narrow sibling seam. |
| D3 | P2 | The design authorized recording, rather than repairing, a new broken rustdoc link caused by the move. | Permit a link-only/plain-code-span repair and require the post-move broken-link warning set not to grow over baseline. |

Changed in Design FIX-1:

- `dev/plans/0.8.27/features/slice-60/plan.md`
- `dev/plans/0.8.27/features/slice-60/design.md`
- this review record

### Cycle 2 - FAIL

A distinct `gpt-5.6-terra` high-reasoning reviewer found three issues in
Design FIX-1.

| ID | Severity | Finding | Design FIX-2 |
| --- | --- | --- | --- |
| D4 | P1 | The proposed hidden public setter would itself violate the no-new-public-path and hidden-surface equality requirements. | Replace the setter and engine field with a private TEMP-marker consumer armed through the existing `execute_for_test` surface. |
| D5 | P2 | The commit cases were `test-hooks`-gated, but no gate executed the boundary suite with that feature. | Require focused debug and release `test-hooks` runs in AC27-60B/G and gate step 7. |
| D6 | P3 | The audit record called the updated seam debug-only. | Correct the record to `test-hooks` and describe the private marker design. |

Changed in Design FIX-2:

- `dev/plans/0.8.27/features/slice-60/plan.md`
- `dev/plans/0.8.27/features/slice-60/design.md`
- this review record

### Cycle 3 - PASS

The Cycle 2 reviewer rechecked Design FIX-2 and returned PASS. The private
TEMP marker preserves public and hidden surfaces, the legacy-root exception is
internally consistent, and the feature-enabled execution receipts are now
mandatory. No actionable design finding remains.

## Test review

### Cycle 1 - FAIL

A distinct `gpt-5.6-terra` high-reasoning reviewer audited the tests and gate
receipts after design PASS.

| ID | Severity | Finding | Test FIX-1 |
| --- | --- | --- | --- |
| T1 | P1 | The boundary suite still enumerated 11 tests and exercised neither real writer `tx.commit()` exit. | Add `test-hooks` cases for the trigger-suppressed and row-trigger exits, then add the private TEMP-marker consumer and one-shot SQLite commit hook used by both exits. |
| T2 | P2 | The corrected rustdoc criterion contradicted the retained 58-to-59 broken-link regression and stale 11-test verification receipt. | Repair the moved-symbol link, compare baseline and candidate warning counts, and supersede the test receipts. |

#### RED/GREEN and non-vacuity

Before production changed, each new commit test failed at the full-snapshot
assertion because the write committed. After the private seam landed:

- debug `test-hooks`: 13/13 passed;
- release `test-hooks`: 12/12 passed (`pre_tx_hook` is debug-only); and
- each required mutant, which passed `false` at exactly one commit-helper
  call, made only its corresponding commit test fail at the snapshot
  assertion.

The rustdoc comparison used the exact pre-move characterization commit
`2300e11b` and the Test FIX-1 tree with private items documented. Both emitted
58 `rustdoc::broken_intra_doc_links` unresolved-link warnings, and the sorted
warning-message multisets were identical. The newly added
`enforce_provenance_retention` warning is gone.

Changed in Test FIX-1:

- `src/rust/crates/fathomdb-engine/tests/write_boundary_atomicity.rs`
- `src/rust/crates/fathomdb-engine/src/write.rs`
- `src/rust/crates/fathomdb-engine/src/write_commit.rs`
- `src/rust/crates/fathomdb-engine/src/lib.rs`
- `dev/plans/0.8.27/features/slice-60/tdd-chronology.md`
- this review record

### Cycle 2 - PASS

The Cycle 1 reviewer independently reran the complete boundary suite in both
required profiles and returned PASS. It verified both commit-path assertions,
the private one-shot seam, the mutant receipts, and the repaired rustdoc
warning-set receipt. No actionable test finding remains.

## Code review

Pending test PASS.

## Final status

IN PROGRESS.
