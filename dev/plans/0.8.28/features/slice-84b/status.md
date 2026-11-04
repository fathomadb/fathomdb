---
title: Slice 84b — certification work and transactional totals
status: IN_PROGRESS
date: 2026-11-03
---

# Slice 84b status

Both implementation targets are implemented and Astra-reviewed. Thirteen focused
certificate tests and retained physical/availability suites pass independently;
default and test-hooks/operator engine Clippy/typechecking pass. Performance
screening and final workspace verification remain pending. No service benefit
or complete slice is claimed yet.

The [plan](plan.md) and [design](design.md) reconcile retained Slice 84 source
`77e2de796`, enumerate actual mutation effects and preserve its negative result.
No scheduling, public API, schema format or physical-success contract changed.
A primary-connection cleanup-job certificate skips repeated structural proof on
proved plain writes. Actual SQLite class/cursor rows and exact retention deletes
supply transactional envelope deltas. Unknown drift reconstructs authority;
COMMIT failure discards it; finalization always uses the original full proof.

RED initially failed on missing totals/tokens. GREEN implements both targets.
Focused tests compare real writes with full totals/zero proof, generated retained
history, same-batch supersession, unnamed/NULL edges, configured property FTS,
Unicode/NUL, opposite-class cursors, maximum deletion, retention across the cap,
all logical cap boundaries, actual COMMIT failure and the external COMMIT/cache
publication race. Separate warm counters show zero full-envelope reconstructions
and zero repeated zero proofs; changed-row accounting remains active. Small-store
VM work was 7411 cold versus 6725 warm; this is not a latency qualification.

Astra code findings (exact class/cursor accounting and optional reconstruction
fallback) are fixed. The screening controller was tested RED/GREEN and its
review finding (idle recovery cannot count as initial public-call success) fixed.
No new branch/worktree was created. Existing development checkout is the sole
writer; final landing and release records follow verification.
