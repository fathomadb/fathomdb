---
title: FathomDB 0.8.27 Slice 10 - approved repository preparation plan
status: COMPLETE
target_release: 0.8.27
depends_on: prework Slice 9
---

# Slice 10 - approved repository preparation plan

## Outcome and boundary

Prepare a truthful, secure, reproducible repository for F27-01 and the refactor
pilot. Do not implement erasure behavior, move product code, change schema,
expand public surface, push, tag, publish, or mutate registries.

## Entry reconciliation

Enumerate changes since this plan, related main/release work, the exact files
allocated by prework, and any new advisory/public-truth/security evidence.
Narrow or stop if an item is already fixed or requires product behavior.

## Requirements and acceptance

| ID | Requirement | Acceptance |
| --- | --- | --- |
| R27-10A | Current public and planning entry points state exact 0.8.26/0.8.27 truth. | Existing public-doc truth RED closes; broadened fixtures cover root/site/install/API entry points; MkDocs and index checks pass. |
| R27-10B | Development tooling contains no known `smol-toml <=1.7.0` DoS advisory. | Lock resolves >=1.7.1; markdown neutrality/lint/tooling self-tests pass with no unrelated update. |
| R27-10C | Action comments and platform/release metadata match runtime pins and published truth. | Two comments change only text; exact SHAs stay; platform and roadmap/state checks pass. |
| R27-10D | Benign performance digests have one exact, adversarially tested authority. | Positive fixtures pass; path/key/value and credential-shaped mutations fail; current-tree Gitleaks passes. |
| R27-10E | Slice 30 has reproducible owned tooling/storage prerequisites. | Exact comparator tool/nightly decision, owned scratch/build roots, and route-specific disk budget are documented and focused probes pass. |

## TDD and implementation sequence

1. Preserve the existing public-doc-truth failure, add focused RED fixtures for
   the additional maintained entry points, then correct only current truth.
2. Add/retain an advisory witness, update only the root resolution needed for
   `smol-toml >=1.7.1`, then run markdown-neutrality and tooling tests.
3. Correct the two upload-artifact comments mechanically after confirming the
   official SHA/tag mapping; assert the SHAs are byte-identical.
4. Add RED security fixtures proving duplicated or broad digest exceptions and
   credential-shaped values are rejected; centralize exact reviewed metadata;
   then run current-tree Gitleaks. Security review is mandatory.
5. Probe, do not globally install, Slice 30's comparator/nightly and disk
   requirements. Record exact versions and owned paths; keep global
   serialization/provisioning out.
6. Obtain independent code review and independent verification. Run focused
   validators first, then `agent-verify`; do not run release/platform matrices.

## Close

Write design, TDD chronology, review, verification, and status records with
exact commits/commands/exits. Update release state to Slice 20 only when every
accepted item is green. Clean only disposable paths proven owned by this slice.
