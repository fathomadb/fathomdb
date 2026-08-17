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

`e7226124` (`test(perf): cover tc5 executor review findings`) preceded the
hardening correction. It failed on the intended containment, receipt-digest,
finite-sigma, and Rust/engine-feature provenance assertions before
`2ed76050` fixed them.

## Verification

- `python3 -m pytest tests/experiments/test_tc5_live_executor.py
  tests/experiments/test_tc5_characterization.py
  tests/experiments/test_tc5_manifest.py -q` — 43 passed.
- `python3 -m pytest tests/experiments -q` — 123 passed.
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
