# SCALE-01 released live-executor worker handoff

**Date:** 2026-08-16

**Worker branch:** `experiments/performance-scale-live-executor-20260816`

**Base:** `a79ab744a585ece6103c16971cbe51902e516d4c`

**Final implementation commit:** `2ed76050bd670524a73c2c8b850b2703d59b8e5f`

## Scope delivered

- Separate `experiments.tc5_live_executor` module and committed
  `tc5-live-executor.v1` configuration.
- Coordinator-release validation bound to integrated SHA, frozen config,
  qualified manifest, action, external runner hash, and expiry.
- Bridge-then-primary external execution protocol, strict safe arm-result
  validation, resolved-root/symlink containment, safe external two-arm receipt,
  and resume-safe reuse.
- Human-intended red-first tests and the dated live-executor contract.

## Red-first evidence

`50790f43` (`test(perf): specify tc5 live executor release gate`) preceded the
module and stopped at the intended
`ModuleNotFoundError: experiments.tc5_live_executor` collection failure.

`e7226124` initially added the review cases, but its fixture was not compatible
with the pre-hardening arm-result schema; it is retained as regression coverage,
not cited as red evidence for the hardening correction.

`4b414d65` is the unmasked historical red replay against pre-hardening
`0aa3d173`. It first proved a fully valid legacy baseline, then independently
mutated arm and receipt symlink containment, receipt digest projection,
`NaN`/positive-infinite/negative-infinite sigma, Rust version, and sorted
engine features. It produced **seven** genuine behavioral failures: both
containment escapes, the missing digest projection, `NaN`, positive infinity,
Rust-version drift, and engine-feature drift (no generic provenance-key
failure). The legacy executor already rejected negative infinity as
non-negative; its failure against a new `finite` wording expectation was not a
behavioral defect and is not counted. Negative infinity remains explicit
regression coverage for the hardened finite-only rule. The intentionally
failing replay file is removed by this handoff correction so the shipped suite
stays executable; the corrected, passing regression assertions remain in
`test_tc5_live_executor.py`.

This replay establishes the missing historical red evidence. It does **not**
claim that the already-landed `2ed76050` implementation chronologically
followed this later replay, and the existing release/live-config binding test
is regression coverage rather than new red evidence.

## Verification

- `python3 -m pytest tests/experiments/test_tc5_live_executor.py
  tests/experiments/test_tc5_characterization.py
  tests/experiments/test_tc5_manifest.py -q` — 44 passed.
- `python3 -m pytest tests/experiments -q` — 124 passed.
- `python3 -m ruff check experiments/tc5_live_executor.py
  tests/experiments/test_tc5_live_executor.py`, `./scripts/agent-lint-md.sh`,
  and `git diff --check` — passed.
- `./scripts/agent-verify.sh` — passed; no live command was invoked.

## No-live assertion

This preparation worker did not acquire or inspect a corpus payload, invoke an
embedder/model/EU7 harness, run a smoke or benchmark, write a campaign artifact
or index row, make an external request, use GPU or paid services, push, or
start remediation or SCALE-02. Test fixtures create only temporary fake
external sidecars.

## Reviewer focus

1. Confirm the existing disabled preparation boundary and historical EU7 files
   remain unchanged.
2. Confirm the release sidecar rejects absent/tampered/stale/mismatched
   authority before external output creation.
3. Confirm the executor can run only bridge then primary, rejects GPU,
   padding/substitution, `NaN`/infinite uncertainty, incomplete or
   provenance-drifted results (including Rust version and sorted engine
   features), and keeps all receipts free of raw paths, document IDs, payloads,
   and SCALE-02 claims.
4. Confirm a complete two-arm receipt is merely index-eligible and does not
   append the experiment index itself; it must carry safe digests for config,
   frozen execution config, release sidecar, and runner.
5. Confirm resolved arm and receipt destinations cannot escape the external
   root through symlinks before any directory, driver, or receipt operation.

## Future coordinator command and prerequisites

Use the exact command and factual-preflight checklist in the linked
[live-executor contract](2026-08-16-scale-01-live-executor-contract.md). The
next gate is independent read-only review, then coordinator-only integration;
no live release or execution is requested by this handoff.
