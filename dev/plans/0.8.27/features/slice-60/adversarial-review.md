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

Pending design PASS.

## Code review

Pending test PASS.

## Final status

IN PROGRESS.
