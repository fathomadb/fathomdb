---
title: Slice 135 exact-source provider and close replay
status: FOCUSED_EXACT_SOURCE_DIAGNOSTIC
target_release: 0.8.27
---

# Provider and close replay on current source

The [manifest](manifest.json) binds this replay to source
`b723791975ec8c5170b7ddb33b7df0e7c84a209a`, both test-binary hashes,
`Cargo.lock`, the selected feature set, host and kernel, exact commands and
raw-file digests. Five direct runs each of the existing foreground-dispatch
and projection-capacity targets passed: 10 and 4 distinct cases respectively,
or 70 case executions. The [independent audit](audit.py) checked every named
case against a fixed list, all ten passing summaries, raw hashes, commands,
resource counters and the 15 expected injected-provider-panic reports. Its
[summary](audit.json) found zero child swaps and 31,880–37,136 KiB peak RSS.
The [raw foreground log](foreground-01.stdout),
[projection log](projection-01.stdout) and their adjacent stderr/resource
files are retained in this directory. Recomputed-digest mutations that
removed a case line or changed a panic report were rejected in the
[negative controls](negative-controls.json).

These tests exercise a real database for provider timeout/error, bounded
dispatch, close cancellation, pending projection recovery and selected
provider panic behavior. The panic cases assert the Rust caller boundary.
They do not establish panic containment through installed Python or
TypeScript bindings, and five repetitions of selected targets do not close
the full Phase 1 provider, kill, erasure or resource matrix. No full workspace
gate was run for this receipt-only increment.
