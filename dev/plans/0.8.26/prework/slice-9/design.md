---
title: FathomDB 0.8.26 Slice 9 — approved preparation design
status: DRAFT — HITL SCOPE RULED, REVIEW PENDING
---

# Slice 9 design — approved preparation design

## Approved items

The narrow preparation bundle is ruled at `seq-286`:

- repair 0.8.25 publication-state truth and add lifecycle validation;
- create 0.8.26 state and board only after that repair;
- make the authoritative main ref explicit in preflight;
- correct the exact maintained authority/platform/CLI documentation identified
  by Slice 2; and
- correct `download-artifact` v4.3.0 comments without a runtime upgrade.

Checkout-owned tools, fresh artifact environments, and disk-budget checks are
execution conditions. Windows inventory consolidation belongs to Slice 50.
Markdown dependency remediation is excluded unless it becomes a hard build
blocker and HITL explicitly admits it.

## Change boundaries

### Publication and release state

The published 0.8.25 record, machine state, and generated views must agree.
Validation fails closed on impossible lifecycle combinations. The 0.8.26
single-writer state and board are created only after that invariant is green;
generated regions are produced from state rather than hand-edited.

### Preflight authority

Preflight names and reports the authoritative main ref it compares. A stale
local branch cannot masquerade as current remote authority. Remote-only and
offline behavior must be deterministic and diagnostic, without silently
changing the selected authority.

### Documentation truth

Only enumerated maintained documents and inaccurate action-pin comments change.
Historical evidence is not rewritten. No dependency, action runtime, product
contract, packaging, or feature behavior changes under a documentation fix.

## Design constraints

- Preparation changes remain separable from feature changes.
- Slice 6–7 fixes enter Slice 9 only when independent of post-10 product
  artifacts; later dependencies remain in their explicitly assigned slices.
- Accepted contract changes preserve ADR supersession and interface-document
  authority.
- Build and CI/CD fixes use non-publishing regression witnesses first and do
  not weaken tests, preflight, gitleaks, action pins, or credential boundaries.
- Test-infrastructure repairs must not skip or reinterpret a failing product
  test.

## Verification design

Each behavioral item has a failing regression witness before implementation.
P26-01 covers lifecycle consistency; P26-02 covers state/view validity and
preflight; P26-03 covers explicit-ref, stale-local, remote-only, and offline
cases. Documentation edits use source-of-truth comparison rather than invented
behavioral tests. Evidence records source finding, files, RED/GREEN commits,
focused checks, repository gate, reviewer verdict, `seq-286`, and proof that
later-allocated work was not pulled forward.
