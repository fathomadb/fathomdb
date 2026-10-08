---
title: Slice 135 exact-candidate focused robustness replay
status: EXACT_SOURCE_REAL_DATABASE_REPLAY
target_release: 0.8.27
---

# Focused robustness replay at source 224e44c59

Three existing Rust engine targets were rerun with `test-hooks,operator`
against exact source `224e44c593c13d86ece648adabe445723db04070`:
`slice135_robustness_first`, `tc91_projection_commit_hardening` and
`slice135_persistent_provider`. The [raw test output](replay.stdout)
reports **12 passed, zero failed, two intentionally ignored**. The ignored
targets are the parent-controlled crash victim and a deliberately wrong
negative-control assertion.

The [state records](replay.stderr) include the bounded two-writer/two-reader
overlap, before-write and after-acknowledged-write process kills, one-shot
pretransaction refusal, bounded and persistent SQLite-full faults, and a
persistent provider failure through its retry budget followed by explicit
rebuild and reopen. The six TC-91 cases cover projection-commit busy,
storage and panic failures, rollback, redispatch and stop/reopen. The
[replay audit](replay-audit.json) checked the test summaries, exact observed
versus expected reopened state in the emitted records, integrity results and
provider terminal/recovery states; hashes bind its raw logs.

All fixtures used temporary real SQLite databases with direct assertions;
those databases were deleted after the tests. The logs are retained, but a
second reader cannot independently reopen those particular files. The
separate exact-source [crash and persistent-fault probe](../2026-10-08-crash-permission-224e/README.md)
adds a named queue-cleanup interruption and persistent SQLite lock and
read-only connection cases. The earlier five-run receipts retain additional
raw resource observations and independent negative controls.
