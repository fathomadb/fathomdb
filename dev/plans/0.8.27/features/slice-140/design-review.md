---
title: FathomDB 0.8.27 Slice 140 — independent design review
status: COMPLETE
target_release: 0.8.27
---

# Slice 140 design review

## First review — `gpt-6.1-sol` high, 2026-10-10

Independent read-only review of [plan.md](plan.md), [design.md](design.md),
the master Slice 140 scope, current source, allocated prior-slice work,
ADRs and interfaces. Verdict: **APPROVE WITH FIXES**, no P0/P1.

| Finding | Resolution |
| --- | --- |
| P2: the eight-file production count omitted `fathomdb-cli/src/lib.rs` (2,792 lines), and the scan domain was underspecified. | Plan and design now count nine first-party production files and require one tracked-code scan with explicit owned, generated, and vendored dispositions. The register remains review evidence, not a size-based CI gate or authority for unrelated moves. |
| P2: the hidden oracle's release probe cannot prove absence after a path disappears from the default rustdoc row. | Design and AC27-140D now require isolated external-dependency compile-negative controls for removed public paths in default dev and release profiles, paired with positive explicit-feature controls. The hidden oracle remains a supplementary diff/test-inventory witness. |

The reviewer found the three-SDK taxonomy, Slice 117 concurrency boundary,
Slice 137 exclusion, conditional ADR acceptance, historical-memo labels, and
dense-state precedence edit consistent with the examined contracts. It also
noted that a human owner verdict on each review-band file remains a Slice 140
closure requirement; model review prepares the register but cannot supply
that verdict.

## Follow-up review — `gpt-6-sol` high, 2026-10-10

Independent read-only review of the corrected plan, design, and register.
Verdict: **APPROVE**, with no remaining P0/P1/P2 findings. The reviewer
recounted all 39 tracked code files in the threshold bands (19 first-party,
19 vendored, one generated), including the CLI file, and checked the
compile-negative controls against the hidden-oracle selection limit. The
master carryovers, current seam owners, Slice 117 handoff condition, and Slice
137 exclusion aligned. The human owner file verdicts remain pending for Slice
140 implementation closure; the review did not mark AC27-140C complete.
