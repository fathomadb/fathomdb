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

## Slice 82 extension (0.8.28)

The owner-approved CH1 Phase 1 and its Astra-reviewed
[production design](../plans/0.8.28/features/slice-82/design.md) extend the same
native physical-debt boundary to erasures without registered dependents and to
retained FTS terms in admitted current-format stores. A versioned root journal
and a separate sanitation state use the existing open-state table; no schema
migration or new governed API is introduced. Existing dependency closure lookup,
root identity and affected-count semantics remain unchanged.

Doctor additionally reports content-free `root-erasure` and `retained-terms`
maintenance identities where applicable; these are operator diagnoses, not
governed closure IDs. Missing sanitation state means unassessed legacy debt.
Ordinary supported FTS mutations invalidate the clean certificate without doing
cleanup or globally fencing writes. An admitted root operation fences mutations
and publication until real finalization, including when it had no dependents.

Offline completion validates root and existing closure proofs, enables SQLite
secure deletion, commits resumable negative-first/positive-continuation FTS merge
units and requires successful TRUNCATE before atomically completing obligations.
It handles legacy dirty-only stores and pre-repair pending closures. No-work
recovery is reserved for a clean certificate with no native/sink debt. Recovery
still cannot authenticate an original attached telemetry sink and refuses sink
obligations; original sinks must be reattached to the Engine for exact retry.
Prospective queue entries authenticate registered canonical sink path fingerprints;
unbound legacy queue entries remain owed. Historical unregistered sink paths
cannot be reconstructed or certified by this extension.

The existing success/error meanings, conservative fence and offline-only recovery
policy stand. The eventual guarantee/API decision remains at the end of Slice 83.

## Slice 83 scheduling successor

[Resumable physical cleanup](ADR-0.8.28-resumable-physical-cleanup.md) supersedes
the exact-retry-only scheduling and unconditional physical mutation fence above.
Certified unrelated legacy writes may progress during sanitation; automatic idle
and reopen service resumes debt. Physical success, original-sink authority and
existing audit/closure semantics remain unchanged.
