---
title: FathomDB 0.8.26 Slice 5 — draft verification matrix
status: COMPLETE
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
| breaking V1 boundary | the changed-in-place V1 contract is the only actuation grammar; no V2 pair or historical replay/receipt/integrity path exists | 35, 40 |
| fresh database boundary | fresh bootstrap succeeds; representative earlier database refuses before mutation; no migration matrix | 35, 40, 50 |
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

## Completed adequacy findings

Existing evidence, actuation, dependency-closure, and integrity suites provide
a broad baseline. This release needs focused delta proofs, not a new global
test program. The explicit R26/AC26 register in Slice 3 closes the prior
ownership gap; owning slices must replace draft criteria with accepted global
or release-local trace before implementation.

Additional required cases are:

- Slice 10: telemetry-enabled identity, telemetry-disabled unique fallback,
  both frozen APIs, disabled control, and Rust/TypeScript/Python public proof;
- Slice 20: separate target/edge identities, restart and one-transaction
  linearization, plus indistinguishable lifecycle/refusal cases;
- Slice 30: published-artifact invocation, lock-held/quiescence behavior,
  incompatible schema response, and before/after process mutation evidence;
- Slice 35/40: single changed-in-place V1 ingress, absence of V2 routing,
  fresh-database boundary, approved endpoint policy, current V1 edge
  revision/digest round-trip property, and closure/supersession/erasure effects;
  and
- Slice 50: fresh-artifact cross-SDK/native matrix with no publication.

Fault injection is required only at interruption boundaries introduced or
materially changed by a slice. Thin bindings should share fixtures and test
their conversion boundary instead of duplicating engine semantics. Focused
blast-radius verification is appropriate during feature work; the full gate
and platform matrix belong at slice close or Slice 50. The executable frozen
retrieval recipe belongs in Slice 10, while Slice 50 validates the published
shape.
