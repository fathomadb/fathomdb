---
title: Slice 135 exact-source installed Python and TypeScript FFI boundary probe
status: AUDITED_SELECTED_BOUNDARY_EXERCISE
target_release: 0.8.27
---

# Installed FFI write-validation boundary at source 224e44c59

The exact-source installed [Python wheel](python-result.json) and
[TypeScript npm package](typescript-result.json) each rejected an embedded
NUL and an unpaired surrogate in an operational-store write body with the
typed `WriteValidationError`. The [Python](python-probe.py) and
[TypeScript](typescript-probe.mjs) probes used a fresh real database, checked
the write counter did not change, then closed and reopened it and checked
the counter again. The [independent result audit](independent-audit.json)
rechecked both typed refusals, counters and installed runtime identity.

The release-built wheel and npm addon omit the test-only forced-panic hook,
so these installed artifacts cannot execute the AC-067 injected-panic case.
The source-level PyO3 `call_engine` and N-API async/sync dispatchers are
`catch_unwind` wrapped, and earlier test-hooks builds exercised representative
panic survival. That source structure and earlier tests do not prove every
exported path on these exact release artifacts. This remains a stated
boundary-coverage risk in the Phase 1 checkpoint.
