---
title: Slice 135 interrupted-erasure engine-reopen diagnostic
status: INTERIM_ROBUSTNESS_RESULT
target_release: 0.8.27
---

# Interrupted erasure survives engine reopen — 2026-10-07

**Result:** one focused, real-database test passed on the integrated 0.8.27
candidate product bytes. A rotated telemetry sink caused a typed incomplete
erasure after the canonical deletion committed. The pending redaction survived
`Engine::close`, handle drop and a fresh `Engine::open`. Retrying without the
original sink attached again refused completion; restoring that sink and
retrying removed the victim ID, retained the control ID and cleared the pending
obligation. This is one row of the Phase 1 robustness matrix, not its completion.

## Identity and execution

- Product source: `cdf253cd223a82e954591db532397a3d78a2027a`, the integrated
  post-Slice-132 candidate with both off-ladder landings. The Slice 135 branch
  descended from this commit; `git diff --exit-code cdf253cd223a82e954591db532397a3d78a2027a
  -- Cargo.toml Cargo.lock src/rust/crates/fathomdb-engine/src src/python src/ts`
  exited zero before this receipt. The new test changes no product code.
- Test source: `src/rust/crates/fathomdb-engine/tests/erasure_completeness.rs`,
  SHA-256 `a5e3fc4ed943592ab24d1baac0ae9c04ad6373811e9798837d791ca3e05c4a20`.
  `Cargo.lock` SHA-256:
  `9e9d7b5e82184a0bddfbe96de28fa0615ef4443fc84639b271c1cff29600ccfe`.
  Executed test binary SHA-256:
  `dc6a13020f8f97453c47d06b93af91a6db69f8e54e8e670c1a4f4071646feea7`.
- Rust 1.95.0, Linux 7.0.0-38-generic x86_64. The test uses fresh temporary
  SQLite and telemetry files, a real `Engine`, and an independent SQLite
  connection for raw row and `PRAGMA integrity_check` assertions.
- Command, run with a 120-second timeout and `/usr/bin/time -v`:

  ```sh
  cargo test --offline --locked -p fathomdb-engine --features operator \
    --test erasure_completeness \
    pending_telemetry_redaction_survives_engine_reopen -- --exact --nocapture
  ```

  The retained [test log](test.log) reports **1 passed, 0 failed** and the
  structured state record. [Resource output](resource.txt) reports exit zero,
  0.67 seconds total wall time, 485,992 KiB maximum RSS, 603 major faults and
  zero swaps. Compilation and test setup are included; these figures are
  resource context, not system-latency samples.

## State oracle and independent check

| Step | Raw observation asserted by the test |
| --- | --- |
| Sink rotation, first erasure | `ErasureIncomplete:telemetry_redaction`; victim canonical rows 0; one durable pending-redaction row; rotated sink still contains victim ID. |
| Close, drop and fresh engine reopen | Retry without a telemetry sink returns `ErasureIncomplete:telemetry_redaction`; the pending row and rotated victim ID remain. |
| Restore original sink and retry | Pending rows 0; victim ID absent from sink; control ID retained; victim canonical rows 0; control canonical rows 1; SQLite integrity `ok`. |

The [audit summary](audit.json) was recomputed from the raw log by
[`slice135_erasure_reopen_audit.py`](../../../../../../../scripts/slice135_erasure_reopen_audit.py).
It requires exactly one selected passing test, one structured observation and
every expected transition value with its JSON type. The [auditor test
log](audit-tests.log) reports seven passes, including rejection of altered
pending state, victim/control state, sink state, integrity, duplicate records
and a missing success result. [SHA256SUMS](SHA256SUMS) covers all retained log
and summary files. The fixture's source assertions, rather than the printed
JSON alone, are the behavioral oracle.

## Limits and next work

Rotation is a controlled filesystem fault, not a process kill or power-loss
simulation. This test does not exercise a failed WAL checkpoint, sustained
SQLite pressure, a concurrent erasure caller, installed SDK boundaries or a
persistent telemetry failure. It is candidate-only robustness evidence; no
0.8.26 latency comparison or release performance verdict follows from it.
Complete those remaining fault/schedule rows and account for resource cleanup
in the full Phase 1 robustness matrix.
