---
title: Slice 82 production erasure design
status: APPROVED — Astra pre-code review 2026-11-03
target_release: 0.8.28
---

# Slice 82 production erasure

Astra approved the exact production design after review of native debt, sink
identity, offline secure deletion, legacy closure recovery and vector proof scope.

Implements CH1 Phase 1 under the existing global physical-debt fence. No new
governed operation, asynchronous acceptance, background scheduler or guarantee.
The [phase plan](../../../ch1-production-erasure-plan.md) owns scope and CH1-1A–1F.

## Existing mechanisms and production delta

`erasure.rs` already freezes/drains projection workers, atomically deletes roots,
registered projections/provenance/receipts, queues telemetry redaction and drives
TRUNCATE. `dependency_closure.rs` persists and validates registered-dependent
zero proofs and fences writes. Operator recovery owns the single offline writer.
The row-owned projection registry is authoritative, including vector spans.

Batch and scalar erasure share `erase_row_projections` and the original
`delete_row_owned_projection` mutation owner, using a single/slice target enum.
The slice is copied and JSON-encoded once; the closed mutation-site/caller
inventory remains unchanged. No parallel batch mutation door is introduced.

Net-new work is a root-level extension to native debt, durable per-family FTS
sanitation, honest retained-term diagnostics, original-sink validation and
late-emission prevention. Native FTS scan corrections follow actual profiling.

## Durable state and admission

Use versioned JSON values in existing `_fathomdb_open_state`; no DDL or canonical
data migration and no format bump. Missing keys mean pre-repair current-format
state, not clean history. A genuinely new store receives a clean certificate only
when main, WAL and rollback-journal are absent/zero-length under the product lock
before writer open, schema-before is zero, and fresh schema/boot initialization
succeeds. Empty canonical tables in an existing file never suffice. Astra
reviewed this correction after unchanged CLI clean-store tests exposed the
missing fresh-store distinction. Reject malformed/unknown-version state at open and on
every state load. Unsupported store formats continue to refuse before mutation.

The singleton native root journal stores a retry fingerprint (domain-separated
SHA-256 of verb and argument), numeric erased cursor scope, admitted write
boundary, exact post-delete dependency generation, related closure IDs and the
mapped vec0 vector row IDs captured before deleting vector ownership sidecars.
It stores no target body, logical ID, source label or test marker. Only one
physical operation can be pending under the existing global fence. Related
registered-dependent closures keep their existing roots/counts/proofs; do not
fabricate a dependent or change `affected_count` to cover no-dependent roots.

Insert the journal in the same transaction as deletion and measured structural
zero proof, including root-only purge and empty/idempotent erasure. Rollback
admits nothing. Surviving journal authority is the committed zero proof plus
exact dependency generation, a write boundary upper bound and revalidation of
numeric cursor/projection absence under the global write/publication fence.
Vec0 absence uses captured vector IDs, not canonical cursors: a record may own
multiple spans whose vector IDs differ from its write cursor. Preserve mapped
vector deletion validation when batching the remaining projection families.
Purge may delete the maximum cursor: include its journal boundary when restoring
the open-time cursor floor. No read-time cleanup. No successful open may run a
projection backfill/reconciliation that mutates state while root debt is active.

Extend native write/publication guards and exact-retry lookup to root debt.
Retain existing read semantics. Keep the projection freeze until the synchronous
attempt finishes; busy/fault exits unfreeze but leave durable guards in place.
Any error after commit leaves retryable debt, regardless of blocker bookkeeping.

## Retained FTS sanitation

Separate store-wide state tracks `dirty`, `cleaning` (family index and started
bit), `finalizing`, or `clean`. Absence is legacy dirty. A clean certificate
applies only until the next supported FTS mutation. Shared projection insert,
delete, truncate and attribute-drop owners invalidate it transactionally; ordinary
mutations do not execute merges. Table creation/recreation starts dirty (fresh
creation and supported rebuilds included). Raw external SQL is outside supported
mutation ownership. Reads never claim that a prior target completion certified
all historical identities or external copies.

Start a whole-store pass atomically with root admission; an offline dirty-only
pass is protected by the exclusive product lock. Derive FTS families from the
row-owned registry. For each family commit one `merge=-64`, then `merge=64`
continuations until the command changes fewer than two rows, as specified by
SQLite FTS5. Commit each merge and its family/started transition together. A
rollback cannot advance progress; crash/retry resumes the positive continuation,
not a fresh negative start. An already optimized, empty or zero-token family is
valid no-work only under this full-pass invariant. Positively populate each
family in tests; test post-delete segment geometry explicitly.

This preserves existing FTS contents, row IDs, BM25 statistics and authorized
history; no active-only rebuild. Conservative fencing prevents family mutation
during the controlled pass. Missing or corrupt families fail closed. There is
no eager rebuild fallback. Continue default FTS deletion plus sanitation unless
paired ordinary-mutation measurements justify prospective secure-delete; merely
enabling a policy can never discharge legacy debt.

After all families finish, retain `finalizing` until original telemetry redaction,
zero-proof revalidation and successful TRUNCATE. Only then atomically mark store
clean, complete related closures and remove root journal. Completion bookkeeping
is content-free; replay after crash revalidates and repeats finalization safely.
An additional checkpoint after bookkeeping cannot erase the durable obligation
early; pre-bookkeeping TRUNCATE is the physical boundary.

Astra's implementation review found provider reattachment could defer vector
grafting past erasure without resuming it. The reviewed correction replays the
existing idempotent survivor vector graft after zero-proof validation and before
sanitation, then revalidates. This changes enrolments/readiness only; it preserves
FTS continuation. Do not blanket-rederive property FTS on debt-bearing reopen.
Other startup repair gates remain outside read-time cleanup; admitted current
format writes already maintain their tuples atomically. A later normal open
retains the existing startup reconciliation behavior.

## Telemetry and external finalization

Register canonical sink path fingerprints durably before enabling capture. Never
unregister historical sinks. Queue all registered sink fingerprints alongside
erased IDs in the deletion transaction, even when no sink is currently attached.
Reattaching another path cannot discharge it. After each authenticated rewrite,
atomically retire only that queue item's matching fingerprint; keep its IDs
until all original sinks finish. Multiple original sinks may require successive
reattach/retry attempts. Never infer identity for unbound pre-repair queue rows:
refuse them. Pre-registry historical sink paths cannot be reconstructed or
certified; current-path enrollment establishes prospective coverage only. Malformed
queue payloads fail closed; missing sink and busy WAL remain existing typed
incomplete outcomes. Preserve the original JSONL rewrite semantics and selective
redaction. A path identifies the declared sink, not arbitrary rotated copies or
backups; missing paths still refuse as today.

Before appending a materialized search event, validate result cursors against
current canonical membership while holding the writer connection, then the
telemetry mutex (same lock order as deletion admission). Emit `[erased]` for a
cursor no longer present. This closes late-emission races without changing the
already materialized caller result or holding its SQLite read snapshot. Apply to
both explanation and ordinary event finalization. Opt-out stays cheap. Test a
search paused after materialization, erasure completion, then emission.

## Operator and interface changes

Doctor reports root debt even without dependent closures and a separate
`retained_terms` maintenance item when the store is unassessed/dirty. Its identity
is content-free and cannot be passed to governed closure lookup. Offline
`complete-erasures` recognizes root, registered closure and retained-term debt,
validates native proofs, sanitizes, refuses unavailable original sink, truncates
and completes. A genuinely clean store is still a no-op. Extend the offline ADR
and Rust/CLI contract for these additive operator observations; Python/TypeScript
erase/purge meanings remain synchronous physical completion with existing errors.

Set `PRAGMA secure_delete=ON` on the offline writer before any merge or metadata
deletion. An admitted pre-repair dependent closure with no root journal retains
its existing proof authority: exact retry/offline completion starts or resumes
whole-store sanitation without readmitting deletion. Include that legacy-pending
case separately from a dirty-only store in recovery tests.

## TDD, qualification and review

| Group | Human-authored oracle and focused witnesses |
| --- | --- |
| CH1-1A | All four families positively populated; content/MATCH/shadow/file/WAL marker witnesses before/after; survivor identities and history unchanged. |
| CH1-1B | Already-deleted legacy ghosts, optimized/empty/Unicode/zero-token families, repeat/rebuild/update invalidation and no ordinary-write merge. |
| CH1-1C | Root-only and dependent admission rollback/commit; subprocess exit after deletion and merge; atomic progress rollback; strict malformed-state refusal; exact retry/reopen/offline convergence. |
| CH1-1D | Pinned external WAL reader, unavailable and wrong sink, malformed queue, delayed search emission; debt survives and exact retry finishes after blocker removal. |
| CH1-1E | Existing lifecycle/provenance/projection-registry/receipt/SDK suites plus survivor-only differential fixtures; deterministic FTS work-count assertions for measured correction. |
| CH1-1F | Reuse frozen Slice 81 masters and offline provider; stage/SQL profile at 1k/5k/10k, three paired 10k node timings and one edge/property pair; ordinary policy-cost comparison, separate sanitation time. |

Keep profiling/measurement driver in the existing experiment package and index it
in its README. Preserve original outputs. Freeze command/runtime/input hashes and
limits before timed campaigns; no concurrent builds/tests during timings. Use the
existing approved experiment envelope; increased budgets require owner approval.
Tests use existing writer transaction/fault hooks where possible. Add only a
scoped engine-owned rendezvous if subprocess/finalization coverage requires it.
Property sequences exercise real SQLite dirty/admit/unit/rollback/reopen/retry
invariants. Astra design review precedes code; Astra code review and independent
Astra verification precede completion. One full final gate, scoped iteration.

Measurement commands use the existing `phase1 MASTER DISPOSABLE_OUTPUT` driver
in the Slice 81 experiment package, release profile, SQLite 3.54.1, performance
runtime and offline fixed provider. Preserved masters are
`/tmp/fathomdb-slice81-campaign/{node,edge,property}-default.sqlite`; SHA-256:
node `6973c7cdd54ccec38ded1ac7aafd247f9df937b437d609665ec9507743aba76d`,
edge `395ebd4eb86ef1a98afd23efafbfaedcdd4e9dc93b0718b0c8a6386c82adaa04`,
property `68c3878118f37eac0048a2046ff16711bd31aea0cb2e6251af7a52c83fa8d2c0`.
Copies are separate; no cache flush, warm host, no parallel build/test load.
Keep the prior 45-minute total experiment envelope, 8-GiB child RSS and 12-GiB
scratch ceilings; 600-second per native command guard. Increased budgets require
approval. Report truncated/incomplete measurements honestly, never as passes.
