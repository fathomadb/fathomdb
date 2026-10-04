---
title: Offline physical erasure diagnosis and completion
date: 2026-10-03
status: accepted
target_release: 0.8.27
supersedes: ADR-0.8.25-dependency-lifecycle-closure.md (operator recovery limitation)
---

# ADR: offline physical erasure completion

## Context

The [dependency closure decision](ADR-0.8.25-dependency-lifecycle-closure.md)
requires an exact retry of the originating purge or erasure operation to finish
an owed physical closure. A failed WAL checkpoint can leave all rows deleted
and a durable closure owed, yet the original argument may be unavailable to an
operator. The closure ids and zero proofs remain in the database.

## Decision

The owner approved a narrow operator-only exception for Slice 103. `doctor
check-integrity` reports each owed physical closure without exposing its source
identity and recommends an exact `recover` command for WAL checkpoint
obligations. A telemetry obligation instead directs the operator to reattach
the original sink and retry the originating erasure. `recover
--accept-data-loss --complete-erasures <db_path>` runs offline under the
canonical product lock with one read-write SQLite connection and no Engine
reader pool or projection runtime. It uses the existing WAL recovery admission
checks, validates the stored and current physical zero proof, discharges any
telemetry obligation only through the correct original sink, truncates the WAL,
and marks a closure complete only after all obligations are discharged.

The recovery acknowledgement remains mandatory because truncation destroys
the last recoverable WAL copy of erased content. A missing or unauthenticated
telemetry sink is a refusal, leaving the redaction queue and closure owed.
The command never guesses a sink from current configuration or clears the
queue solely because the database rows are gone.

## Consequences

- The earlier keyed-only SDK closure lookup remains unchanged. The new
  discovery and completion surfaces belong only to the operator CLI.
- `doctor` stays read-only. An owed closure exits 65.
- Successful offline completion exits 64; held lock or busy checkpoint exits
  71; invalid zero proof or unavailable original telemetry sink exits 70.
- An invocation with no owed physical closure is a no-op that exits 0 without
  truncating unrelated WAL contents.
- Automatic completion at `Engine::open` is outside this decision.
