---
title: Slice 135 graph traversal row-error repair
status: FOCUSED_REGRESSION_PASSED_FULL_GATE_PENDING
target_release: 0.8.27
---

# Graph traversal row-error repair — 2026-10-07

The graph arm of text search silently dropped a malformed traversed edge row
because `rows.flatten()` discarded its SQLite row-decode error. A real-database
[regression test](test-source.rs) first confirmed the graph neighbor was
reachable, then changed the committed edge `source_id` to invalid UTF-8. On
the pre-fix source `22603e6f3c27e0d0dbff60eea50e9f26e8c840fe`, search
returned `Ok` with only the anchor, rather than a storage error. The
[RED log](red.log) records that failed assertion (exit 101 in [red.exit](red.exit)).

Product commit `3ce1a63529a3fc1116cb76b42c35c484ab36f0de` collects graph
rows as `rusqlite::Result<Vec<_>>` before traversing them, so row conversion
errors propagate. The [GREEN log](green.log) and [resource report](green-resource.txt)
come from the exact committed source: the regression and adjacent graph seeding
and graph-arm suites passed, **10 tests total**. Rust formatting passed. The
full workspace gate has not been rerun for this commit; this focused result
does not claim a system-wide green verdict.

The [SHA-256 manifest](SHA256SUMS) seals the retained RED/GREEN artifacts.
Earlier candidate performance and installed-artifact receipts were generated
before this source change, so final Phase 1 comparisons require rebuilt
candidate artifacts and affected cell reruns.
