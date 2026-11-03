---
title: Resumable physical cleanup with scoped foreground admission
date: 2026-11-03
status: accepted
target_release: 0.8.28
supersedes: ADR-0.8.25-dependency-lifecycle-closure.md and ADR-0.8.27-offline-physical-erasure-completion.md (physical retry scheduling and mutation fence only)
---

# Resumable physical cleanup

## Context

Owner-authorized CH1 Phase 2 requires useful unrelated service while physical
cleanup remains owed. Slice 82 repaired the native scan cost; splitting atomic
canonical deletion is not justified by that evidence. The reviewed
[Slice 83 design](../plans/0.8.28/features/slice-83/design.md) records the bounded
admission envelope and qualification obligations.

## Decision

Keep synchronous physical-success semantics: success requires structural zero,
original telemetry redaction, retained-term sanitation and successful WAL
TRUNCATE. An incomplete call retains durable debt. No accepted-only success,
cleanup API, deadline or latency guarantee is introduced.

Admit one bounded native operation and delete its full canonical scope atomically.
While sanitation remains owed, allow only certified unrelated plain legacy node
and edge writes, including bounded supersession. Deny affected source and stable
identities, old and new edge endpoints and provenance-owned supersession.
Dependency registration, provenance, receipts, lifecycle operations, administrative
mutations and projection publication remain fenced. Same-batch rows are checked
after earlier effects. Temporary FTS secure deletion prevents survivor updates
from restarting erased-term debt. Existing snapshots run to completion; snapshots
admitted after deletion see complete canonical removal before scoring or limits.

The versioned root journal binds closure IDs, immutable admission scope, exact
dependency generation and a separately certified monotone safe-write watermark.
Missing or inconsistent authority refuses open. Legacy pending journals retain
their conservative validator and global fence.

One Engine-owned worker services committed units, including after reopen and
while idle, without requiring the original argument. It shares the primary
connection, requests priority against ordinary writers, yields between units,
backs off after refusal and joins before connection shutdown. This supersedes the
exact-retry-only and offline-only scheduling limitations of the prior ADRs.
Exact retry and existing offline operator recovery remain supported.

Finalization temporarily gates new owned reader snapshots and waits at most
100 ms per attempt for existing ones. It never cancels readers. After redaction
of original telemetry sinks, persist an identity-free finalization tombstone
under a global mutation fence, then require TRUNCATE before completion. Temporary
source/identity/retry hashes are removed before that checkpoint. The final commit
introduces no subject identity. Existing authorized closure and audit metadata
is retained. New readers resume after each attempt; an external pinned reader
can prolong the post-tombstone mutation fence without a finite upper bound.
A new erase/purge call completes any prior tombstone before processing its own
argument, and cannot return another operation's success as its own.

## Consequences

Root and closure blockers persist on synchronous and worker failures. Reopening
can make progress without another public call, but downtime, unavailable original
sinks and external readers remain completion exclusions. Sink identity is never
guessed; wrong, absent, unbound or multiple outstanding original sinks retain
truthful incomplete status. No new schema migration or public governed operation
is required. The end-of-Slice-83 owner decision on the broader physical guarantee
and any specific cleanup API remains open; this decision records implementation
mechanics authorized for Phase 2, not that future product guarantee.
