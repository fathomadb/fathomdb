---
title: Slice 135 current-wheel installed Python S02 functional smoke
status: FUNCTIONAL_SMOKE_NO_TIMING_COMPARISON
target_release: 0.8.27
---

# Current-wheel installed Python S02 smoke — 2026-10-08

The installed wheel from clean source
`d465cd56d2e863f900ea9a9da8bc372ca6a077c5` has SHA-256
`33d37be7c80183b063788889954da698ea959997fc13b61e356006a3c26c8894`.
The [raw functional observation](raw.json) exercises a fresh real database
through open, 36 governed writes, projection readiness, text/vector/hybrid
retrieval, graph/evidence, erasure and idempotent retry, close and reopen.
The [independent check](audit.json) verifies the wheel and native-module
bytes, runner hashes, 18 stages and reopened canonical counts. A changed
reopened graph-edge count was rejected.

This one validation-inclusive sequence establishes functional feasibility,
not latency. The [frozen current-source paired subset](../../s02-python-current-comparison-protocol.json)
retains the previously qualified baseline-only noise pilot, fixture, timing
boundary, 20 measured sequences per block and five alternating pairs. Its
exact protocol SHA-256 is
`966a9dbfa086d6c3651611fcffd430b5b60c38f7082fa53b3f9c4b961d644594`.
It was saved before candidate paired timing.
