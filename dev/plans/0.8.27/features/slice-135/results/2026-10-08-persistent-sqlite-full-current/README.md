---
title: Slice 135 persistent SQLite-full write and recovery result
status: FOCUSED_EXACT_SOURCE_DIAGNOSTIC
target_release: 0.8.27
---

# Persistent SQLite-full write and recovery

Five direct runs of the same focused real-database test binary passed on
Slice 135 source `1509818c64cde4c45478c5a2404ead4472cd9667`. The
binary SHA-256 is
`4ee6defa14652bc041b1210ada0c536f73b189247f9a4396eeec7b60dd3e8304`;
the exact command, features, source/test and lockfile hashes, host, and each
raw-file digest are in the [manifest](manifest.json). This commit adds the
test on top of the explanation repairs; it does not change product bytes.

Each run opened a temporary FathomDB database, wrote a durable base row, and
set SQLite `max_page_count` to `page_count + 1` (128 → 129). Three separate
1 MiB governed writes were attempted while the same cap remained active.
All 15 attempts returned `EngineError::Storage`. Point reads after every
failure found the base row and none of the attempted records. After the cap
was lifted, a recovery write succeeded; a fresh engine reopen showed exactly
the base and recovery rows. An independent SQLite query counted two physical
canonical rows, `PRAGMA integrity_check` returned `ok`, and process file
descriptors were 4 before and after each case. GNU Time reported zero child
swaps and 25,108–26,112 KiB peak RSS across the five runs. These resource
values describe the probe, not product latency.

The [independent audit](audit.py) rechecks all five raw
[run logs](run-01.stderr), their hashes, command selection, three failure
states per run, exact reopened records, physical row count, integrity, file
descriptors and resource counters. Its [summary](audit.json) accepted all
five. Two [negative controls](negative-controls.json) recomputed the altered
file digest before auditing; changed reopened data and a child swap both
failed the semantic/resource checks. The test source is
[slice135_robustness_first.rs](../../../../../../../src/rust/crates/fathomdb-engine/tests/slice135_robustness_first.rs).

This covers a persistent bounded SQLite capacity fault at a governed write
boundary. It does not simulate actual device exhaustion, permissions failure,
power loss, a crash inside commit, or a persistent provider failure. The
other rows of the Phase 1 robustness matrix remain open. No full workspace
gate was run for this focused test-only increment.
