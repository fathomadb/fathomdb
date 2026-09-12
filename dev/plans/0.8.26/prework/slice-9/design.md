---
title: FathomDB 0.8.26 Slice 9 — provisional preparation design
status: PROVISIONAL DRAFT
---

# Slice 9 design — provisional preparation design

## Approved items

None. Slice 8 has not occurred.

## Design constraints

- The final design is generated from explicit HITL rulings, not the initial
  recommendations in this planning scaffold.
- Preparation changes remain separable from feature changes.
- Slice 6–7 fixes enter Slice 9 only when independent of post-10 product
  artifacts; later dependencies remain in their explicitly assigned slices.
- Dependency upgrades preserve documented pins unless the pin's original
  constraint is disproved by target evidence.
- Cruft disposition is reversible when practical; history is archived or
  deprecated in place unless deletion is specifically approved.
- Accepted contract changes preserve ADR supersession and interface-document
  authority.
- Build and CI/CD fixes use non-publishing regression witnesses first and do
  not weaken tests, preflight, gitleaks, action pins, or credential boundaries.
- Test-infrastructure repairs must not skip or reinterpret a failing product
  test.

## Evidence placeholders

The Slice 8 replacement must provide a row for each approved item containing:
source failure record or requirement, files, red test or mechanical-change
exception, implementation commit, focused evidence, full gate, reviewer
verdict, final HITL ruling, and confirmation that no later-allocated work was
pulled forward.
