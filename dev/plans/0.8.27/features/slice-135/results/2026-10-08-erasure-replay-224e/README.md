---
title: Slice 135 exact-candidate interrupted erasure replay
status: AUDITED_REAL_DATABASE_REPLAY
target_release: 0.8.27
---

# Interrupted erasure replay at source 224e44c59

Three focused real-database tests passed on exact candidate source
`224e44c593c13d86ece648adabe445723db04070`: precommit proof failure
rolled back every primary plane; postcommit incomplete erasure stayed
truthful and retry was idempotent; and rotated telemetry-sink redaction
survived close and fresh engine reopen. The [correction output](correction.stdout)
reports two passes, and the [reopen output](reopen.stdout) reports one pass.

The [independent audit](independent-audit.json) checked the structured
reopen record: `ErasureIncomplete:telemetry_redaction` before and after
reopen without the sink; pending work remained at one until the sink was
restored; retry reduced it to zero; victim rows stayed absent, the control
record remained, and SQLite integrity was `ok`. The earlier
[interrupted-erasure](../2026-10-07-erasure-interruption-current/README.md)
and [reopen](../2026-10-07-erasure-reopen-current/README.md) receipts provide
the test design and additional fault context. The temporary database was
deleted after the exact-source replay test; its log and assertions remain.

These positions cover precommit, postcommit and telemetry side effects.
A process kill during an erasure commit or WAL checkpoint is not simulated;
that gap remains in the checkpoint's residual-risk matrix.
