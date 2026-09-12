---
title: FathomDB 0.8.26 Slice 5 — draft verification matrix
status: DRAFT
---

# Slice 5 design — draft verification matrix

## Critical-path proof allocation

| Critical path | Minimum proof | Owner |
| --- | --- | ---: |
| frozen explanation completion | regression on both frozen APIs, disabled-path control, ranking/evidence equivalence, installed wheel probe | 10 |
| public retrieval guidance | API-reference and guide truth tests plus executable documented recipe | 10 |
| graph evidence by revision | success and stale/expired/unauthorized/ineligible/nonexistent non-disclosure cases in one frozen reader transaction | 20 |
| target versus terminal edge | separate identities, edge-only proof semantics, deterministic response and cross-SDK parity | 20 |
| operator integrity | real database, quiescence refusal or contract, JSON schema/exit codes, bounds/privacy, no mutation, version mismatch | 30 |
| CLI distribution | clean artifact install/run on selected native targets and absence of doctor/recovery in governed SDKs | 30, 50 |
| derived-edge batch | red tests for all-or-none commit, prospective endpoints, digest/replay, crash/restart, duplicate key, dependency/lifecycle/erasure/projection effects | 40 |
| V1 preservation | golden/compatibility proof for encoding, digest, replay, bindings, and receipts | 40 |
| integrated release | clean wheel/npm/native/CLI matrix, cross-SDK reopen, restart, package metadata, non-publishing release gate | 50 |

## Adequacy rules

- Codec, digest, replay, and round-trip layers require property-based tests.
- Transaction and projection changes require deterministic fault injection at
  each meaningful interruption boundary.
- Privacy-sensitive lookup requires indistinguishable refusal tests.
- Cross-language models require shared fixtures or equivalent contract probes,
  not separately invented expectations.
- A wheel or CLI distribution claim requires installation into a clean
  environment from a freshly built artifact.
- Target-specific claims remain unverified until executed on that target; host
  emulation is not equivalent.
- Tests encode human-approved intent and no snapshot/golden oracle is
  regenerated autonomously.

## Baseline concerns for Slices 6–8

- Re-run the public-documentation truth gate because the current checker may
  still encode a prior release version.
- Run advisory tools with isolated writable caches; a sandbox cache failure is
  not a clean advisory result.
- Confirm the full native matrix versus the narrower Memex cutover target
  before assigning external evidence.
- Keep feature-independent test-infrastructure fixes eligible for Slice 9;
  assign feature-dependent or final-artifact assertions to their owning
  feature, reserved hardening, or Slice 50 plan.
