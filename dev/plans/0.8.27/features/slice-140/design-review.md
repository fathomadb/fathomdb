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
noted that the then-current plan required a human owner verdict for each
review-band file. The owner removed that requirement after the review; see
the policy update below.

## Follow-up review — `gpt-6-sol` high, 2026-10-10

Independent read-only review of the corrected plan, design, and register.
Verdict: **APPROVE**, with no remaining P0/P1/P2 findings. The reviewer
recounted all 39 tracked code files in the threshold bands (19 first-party,
19 vendored, one generated), including the CLI file, and checked the
compile-negative controls against the hidden-oracle selection limit. The
master carryovers, current seam owners, Slice 117 handoff condition, and Slice
137 exclusion aligned. At review time, the human owner file verdicts remained
pending. The later owner ruling removed that gate; the review did not mark
final-candidate AC27-140C coverage complete.

## Subsequent owner ruling — 2026-10-10

Human review or approval of the large-file register is no longer required
for Slice 140 or release closure. The register remains referenced in the
release and is updated for final-candidate file coverage. This changes the
acceptance policy after the reviews; it does not alter their historical
findings on the original draft.

## Post-Slice 117 handoff review — 2026-10-10

An independent `gpt-6-sol` high follow-up reviewed the committed release
handoff `1476d83e0` and returned **PASS**, no findings. It confirmed the
qualified-but-unpublished Jetson boundary, Slice 160 publication ownership,
the absence of human large-file approval, and the still-open implementation
and verification work. Slice 140 implementation integrates that handoff at
`f1d4571b1`; final code review and acceptance verification are separate.

## Default-proof and publication-doc follow-up — 2026-10-10

Independent `gpt-6-sol` high read-only review returned **APPROVE**, with no
P0/P1/P2 findings. The reviewer checked default T20/T25 targets, the structural
unit proof, unchanged C1 obligation/category/hash with evidence relocation and
mutation controls, production-opener scanner controls, and 0.8.27 publication
documentation with actual publication limits retained in internal status.
The reviewer did not run tests; verification receipts remain separate.

## Generated-doc amendment review — changes requested

Independent `gpt-6-sol` high read-only review requested changes:

| Finding | Response before implementation |
| --- | --- |
| P1: the generated-doc master-scope paragraph was accidentally inserted under Slice 80. | Moved the paragraph into the Slice 140 section. No generated block was edited. |
| P2: checkout-local freshness cannot stop a dispatched Jetson candidate from overwriting newer main documentation. | Both whole-site Pages routes now require freshly fetched main to be an ancestor of the exact candidate before build/upload and repeat that check before deploy. Full ancestry, fail-closed errors, unchanged publication privileges, and focused Git-history/workflow tests are specified. The normal main route is covered too. |

The amendment remains pending subsequent `gpt-6-sol` high approval. No
generated-doc implementation was added in response to this review.
