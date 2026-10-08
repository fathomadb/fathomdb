---
title: Slice 135 Phase 1 full workspace verification — green rerun
status: PASSED
target_release: 0.8.27
---

# Phase 1 full workspace verification — green rerun

The required `./scripts/agent-verify.sh` run completed with exit 0 on
committed checkout `418fc23c4`. Lint, typecheck and strict security passed;
security reported zero violations, blockers and downgrades. The test step
reported **186/186 suites passed**, with no skipped or excluded suites. The
[exact gate output](agent-verify.stdout) and its
[gate-only Git exclude](gate-excludes.txt) are retained and hashed in
`SHA256SUMS`.

The exclude hides only untracked local Slice 135 raw-result files from Git
status during the Python test-hook clean-source precondition. It neither
changes ordinary Git status outside the run nor removes the retained raw
archives. The committed runtime product trees under `src/rust/crates`,
`src/python/fathomdb` and `src/ts/src` remain byte-identical to exact
measurement candidate `224e44c593c13d86ece648adabe445723db04070`.

The earlier [red gate receipt](../2026-10-08-phase1-verification/README.md)
records the three Python failures. Commit `418fc23c4` repaired the verifier
child's import path and made the Python surface check recognize only the
approved, interface-documented post-Slice-130 deltas while preserving the
historical snapshot and stub hash as the oracle for every other declaration.
