---
title: Slice 83 production availability design
status: APPROVED — Astra pre-code review 2026-11-03
target_release: 0.8.28
---

# Slice 83 production availability

## Baseline reconciliation

Baseline is `abc4f7ad7`, including completed Slice 82. The draft predates these
changes: atomic root debt now covers erasure without dependents; four registered
FTS families have negative-first/positive-continuation durable sanitation;
root/dependent proofs require exact dependency generation and a global write
boundary; native batch deletion replaces per-cursor FTS scans; original sinks
are fingerprinted and late telemetry is redacted; fresh-store certification and
mapped-vector validation have been corrected. The original node workload now
finishes in 1.229 seconds rather than 271.090. Native deletion remains atomic;
the individual sanitation transactions still retain the primary connection mutex
across the whole loop. Reopen preserves debt but requires caller-driven retry.
These facts replace the draft's assumptions; no further FTS deletion rewrite is
justified. Detailed writer/drain/checkpoint profiling remains owed by Slice 82.

The allocated work is CH1-2A–2F in the shared plan. Other CH units, expiry,
public cleanup/status API, reader cancellation and numerical SLAs remain outside
this slice. Existing physical-success meanings remain mandatory.

## Proposed minimal production mechanism

Keep one admitted root operation. Retain atomic canonical/dependency/projection
removal unless separated stage measurements show it dominates writer ownership.
Before discovery, refuse stores above 1 GiB of logical SQLite pages, 100,000
canonical rows, 1,000,000 registered vector mappings, or 64 MiB of provenance/
receipt metadata. These checks execute under the writer lock before target-set
collection. A target set above 16,384 cursors refuses before destructive work;
within the finite store envelope the discovery allocation is bounded. Canonical
body size is capped at 2 MiB per row and 256 MiB total for this admission path.
These are internal supported-work limits, not new SDK configuration. No data is
removed on refusal; offline recovery remains separately controlled.
A row limit is not a duration bound. The admitted immutable cursor cut, closure
membership and target scope never change on retry.

Yield the primary mutex after every committed sanitation unit. One Engine-owned
maintenance worker shares the primary connection; it opens no SQLite connection.
A separate service mutex serializes public erasure and worker attempts. The
worker resumes committed debt on reopen without future user writes, uses bounded
retry backoff for blockers, and stops/joins before the primary connection closes.
It must not own the Engine or form an Arc cycle. Public erase/purge retain the
original report after successful physical completion; no accepted-only success.
An attempt that cannot finalize returns the existing truthful incomplete error.
Maintenance marks a priority turn before acquiring the primary connection; new
ordinary write admission waits for that turn, while already-admitted operations
finish. The worker relinquishes the priority turn after each committed unit and
allows a foreground opportunity before requesting another. Backoff is interruptible
on close (100 ms blocked retry, 5 ms between successful units). No OS scheduling
or unconditional time bound is claimed. Eligible writes recheck the resource
envelope inside their transaction and roll back if the result exceeds it; they
cannot grow admitted work beyond the supported envelope.

Narrow only the ordinary-write seam, not the global mutation guard. Permit
plain node/edge writes proven disjoint from denied source identities, logical/
generated identities and edge references. Survivor supersession is eligible only
when every affected prior owner is a legacy artifact with no source links,
dependencies, soft closure or receipt/provenance obligation. Check both old and
new source/identity; include unnamed-edge endpoint/kind supersession and earlier
members of the same batch. Keep provenance/dependency operations, admin, rebuild,
lifecycle, operational writes and projection publication fenced. Legacy debt
without scoped denial metadata retains the full fence.

Enable all registered FTS families' prospective secure-delete policy in the
admission transaction, before allowing survivor updates. Reopen restores this
policy while debt exists. It avoids new deletion ghosts while the existing
negative-first pass removes old ones; it does not itself discharge old debt.
Restore the normal policy only at completion. Verify interleaved supersession
across family transitions and measure the temporary mutation overhead. Ordinary
writes retain their current default policy when no erasure is pending.

Capture denial hashes before deletion for the full erased closure, including
mapped dependent vectors, source identities, logical and generated identities.
Hash domains distinguish source from logical identities and ignore public/operator
verb spelling. Low-entropy hashes are dictionary-testable temporary denial data.
After sanitation, original-sink redaction and exact-proof validation, replace the
journal atomically with a redacted-finalization tombstone containing only version,
phase, numeric cursor floor/safe watermark and opaque closure IDs. Remove the new
denial hashes and root retry fingerprint before TRUNCATE. All mutations remain
globally fenced in this persisted phase. On busy checkpoint, release new-reader
admission after the finite attempt, retain the tombstone, and retry with backoff;
external readers can indefinitely prolong this final global mutation pause.
On reopen, this phase resumes checkpoint/completion without reconstructing target
identities: the persisted phase certifies the validated transition under an
unbroken mutation fence. Only successful TRUNCATE permits closure completion and
journal removal. The final metadata commit contains no new subject identity.
Existing contract-authorized audit/source-label and dependency-closure proof
metadata remains retained as before; this does not claim those records are erased.

A redacted tombstone has no target retry fingerprint. Any erase/purge invocation
first completes that singleton finalization, then processes its own argument
normally. The original in-flight caller retains its report in memory; process
crash/retry follows existing idempotent report behavior. Never return success for
a different argument merely because another operation completed.

Retain exact dependency-generation equality while only certified plain unrelated writes
are admitted. Preserve immutable admission boundaries; a separate monotone safe-
write watermark may advance in the same transaction as a certified unrelated
write. Closure replay can consult that watermark only when the new journal binds
its closure IDs and exact target scope. Never drop generation checks merely to
make unrelated writes pass. Validate exact cursor/vector absence at each unit and
finalization. Post-admission dependency registration remains forbidden.
Before tombstone redaction, scoped validation checks the original immutable
proofs, exact dependency generation, bound closure IDs, full erased scope and
certified safe-write watermark. Redacted-finalizing completion instead uses the
durable validated-transition authority under the persisted global fence; it
must not demand removed scope fields or the original global-boundary equality.
Legacy debt retains its original validator. State/open validation recognizes
both versions and rejects inconsistent phase/proof combinations. Restore the
normal FTS policy in the completion transaction while the mutation fence holds.

FTS continuation must not restart when unrelated writes arrive. Qualify existing
negative-first/positive-continuation semantics with interleaved insertion and
family transitions. Qualify the temporary secure-delete policy before enabling survivor
supersession. A target completion must not falsely certify new whole-store
history. No eager rebuild fallback.

## Reads and finalization

Atomic logical deletion is the read linearization point. New snapshots see fully
removed canonical rows and projections, with no partial-deletion ranking problem.
Existing snapshots run to completion. Read inventory: body/property/dense/hybrid,
projected-text/BM25F, get/collection/canonical pages, graph neighbors/expand/seeds,
search-expand/evidence, operational pages, relaxed/historical/frozen contexts;
writer-direct importance and frozen-context authority are included. Independent
survivor-only fixtures compare IDs/order/score bits and evidence authority.

Finalization closes new Engine reader admission for a finite attempt, permits
existing owned snapshots to finish without cancellation, fences writes/publication, redacts
original sinks, revalidates proofs and truncates WAL before completion. Queued
requests must obey the same gate; a check before snapshot admission alone is not
sufficient. External readers can keep it incomplete. Reader drain waits at most 100 ms per attempt, followed by the existing finite
checkpoint-attempt policy. Before tombstone commit, failure releases the temporary
reader gate and leaves scoped write admission available. After tombstone commit,
only the reader gate releases; the durable global mutation fence persists until
completion. Every failure retains debt and a root-only blocker as well as closure
blockers. No attempt waits indefinitely for an owned reader.
A worker must not spin or hold the writer during backoff. Late telemetry remains
serialized with the writer and cannot restore erased identities after redaction.

## TDD and acceptance mapping

| Acceptance | RED/GREEN and verification |
| --- | --- |
| CH1-2A | Pause after a committed native sanitation unit; real unrelated Engine read/write completes with registered dependency debt; affected mutations refuse. |
| CH1-2B | Survivor-only route comparisons varying target length/duplicates/fanout; explicit refusal for uncertified routes; frozen/evidence authority and pre-cut run-to-completion. |
| CH1-2C | Same-source and dependent identity recreation across source labels, queued projection/rebuild, child-process crash/reopen, autonomous idle completion and mapped-vector residue. |
| CH1-2D | Singleton saturation and discovery envelope refuse without deletion; within-batch duplicate and unnamed-edge supersession; bounded loaded mutations; close/error joins and releases ownership. |
| CH1-2E | Pinned external reader plus arriving Engine readers, absent/wrong/multiple sinks, finite gate release and retry convergence; completion bookkeeping crash replay. |
| CH1-2F | Phase-82 baseline versus candidate, three paired node/edge/property 10k/five-revision/10%-target loaded runs and no-erase controls. Distinguish offered/issued/completed/engine-refused/load-generator-missed/unfinished, total native, drain, queue/writer wait/hold, units and checkpoint. |

Use committed RED then GREEN per behavior, real SQLite property sequences and
existing fault hooks. Astra reviews the settled design and code; independent
verification reruns decisive tests and timing arithmetic. One full final gate,
scoped checks while iterating. Reuse the experiment package, baseline executable
built against the actual Phase-82 source, frozen masters and 15-second windows.
Freeze commands before timing under the existing 45-minute/8-GiB/12-GiB envelope;
no concurrent builds during timed work. No p99 claim from incomplete streams.

## Contract disposition

The [successor ADR](../../../../adr/ADR-0.8.28-resumable-physical-cleanup.md) authorizes the
scoped mutation exception and automatic reopen service. Rust/CLI and affected
binding contract descriptions must retain synchronous physical success and
truthful incomplete errors. The final status reports supported mutation envelope,
measured pauses and all external-reader/sink/downtime exclusions. The end-of-slice
HITL choice of physical guarantee/API remains open even when engineering passes.

Independent verification found the existing Phase 82 large-input fixture is
1,050,029 bytes, slightly above an exact MiB. Retain that existing oracle and
provide 2-MiB per-row admission headroom; the new boundary test follows the
revised limit. Total body/file/metadata ceilings remain unchanged.
