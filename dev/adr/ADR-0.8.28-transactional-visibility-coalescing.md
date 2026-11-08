---
title: Owned transactional read-visibility coalescing and store format 37
status: proposed; technical implementation design reviewed
date: 2026-11-08
target_release: 0.8.28 Slice 95
---

# Owned transactional visibility coalescing

The committed [Slice 95 requirements](../plans/0.8.28/features/slice-95/plan.md#ci16-production-repair-after-recovered-evidence)
authorize format37 and this mechanism. This technical record does not declare a
new HITL ruling. It succeeds the connection-wide suppression mechanism of
[Slice 71 phase 3](../plans/0.8.25/features/slice-71/write-regression-subplan.md#phase-3--select-and-independently-review-the-correction),
and advances the current-only boundary in
[explicit erasure maintenance](ADR-0.8.28-explicit-erasure-maintenance.md).
The [frozen-read contract](ADR-0.8.25-frozen-pagination-and-operational-state.md),
vector layout/cleanup, branch-sensitive nonce, WAL/NORMAL and public APIs remain.

## Exact layout and trigger manifest

`SCHEMA_VERSION=37`. Existing nonempty36 databases, including WAL-backed ones,
are rejected by `admit_current_database` before mutating open/setup. Historical
migration SQL stays byte-for-byte unchanged. Fresh bootstrap appends step37:
first add `coalescing INTEGER NOT NULL DEFAULT 0 CHECK(typeof(coalescing)='integer'
AND coalescing IN(0,1))`, then replace these visibility trigger triples.

Each name is `_fathomdb_read_visibility_<short>_<ai|au|ad>` for AFTER
INSERT/UPDATE/DELETE respectively: exactly19 triples /57 triggers.

| Short | Real owning table |
| --- | --- |
| cn | canonical_nodes |
| ce | canonical_edges |
| ar | _fathomdb_artifact_revisions |
| sv | _fathomdb_source_versions |
| sl | _fathomdb_source_links |
| sd | _fathomdb_source_dependencies |
| dc | _fathomdb_dependency_closures |
| pr | _fathomdb_projection_registry |
| ca | canonical_attributes |
| ps | _fathomdb_projection_state |
| pt | _fathomdb_projection_terminal |
| vk | _fathomdb_vector_kinds |
| vr | _fathomdb_vector_rows |
| ep | _fathomdb_embedder_profiles |
| pg | _fathomdb_projection_generations |
| pc | _fathomdb_projection_generation_current |
| oc | operational_collections |
| os | operational_state |
| vs | _fathomdb_vector_spans |

A new generator supplies step37; engine validation expects the exact same manifest. Earlier
generators never reference `coalescing`. A trigger skips only when a real singleton
contains integer1. Otherwise its body requires an existing integer0 singleton or
raises ABORT, then performs the existing checked generation/random nonce update.
Missing/NULL/noninteger state cannot turn a WHEN condition silently false.
The authenticated vector-span mapping DELETE cleanup trigger is unchanged and
remains active during coalescing. External SQLite writers with clean committed
state retain ordinary row-trigger invalidation; no connection-local UDF is used.

## Ownership and completion

Mechanically rename the private `TriggerStateGuard` seam to
`VisibilityCoalescingGuard`, `disable` to `arm`, before behavior REDs. Its constructor
borrows an owned `rusqlite::Transaction` created with IMMEDIATE. It retains the
read-only native ENABLE_TRIGGER refusal and checks clean singleton state, then
changes0→1 inside that transaction after the unchanged fresh main/TEMP custom
trigger admission. Custom-trigger fallback never arms and uses ordinary triggers.

Canonical and projection writers retain their current mutation/fence calls.
The existing checked visibility advance also clears the flag in the same UPDATE.
For projection outcomes with no existing `visibility_changed`, restore clears the
flag without inventing a visibility advance. Successful commit requires clean0.
This preserves the current relevant-change predicate and one advance per relevant
coalesced transaction. Non-coalesced explicit advances remain supported.

`restore` never issues SQL in autocommit mode: SQLite may already have executed
RAISE(ROLLBACK). Active owned-transaction restore may clear1→0; a prior successful
advance already left0. A missing/malformed active state fails through the existing
restore/poison path. The guard destructor issues no SQL: transaction RAII rollback
restores the original clean state on every earlier error. Exhaustion retains its
existing Storage/retry behavior; pre-disabled native triggers retain refusal and
primary-writer poison behavior. Commit abort, savepoints and explicit/automatic
rollback cannot publish a cursor or a committed armed flag.

## Fresh admission seams

`require_clean` reads integer0 from the singleton; it caches no value. It is an
outer-boundary check, not a blanket addition to `root`, `state`, per-row
`guard_plain`, `guard_no_pending_physical`, or visibility-generation helpers that
legitimately run inside an armed transaction.

| Actual ingress / shared seam | Placement and covered mutation families |
| --- | --- |
| `write_inner`, `commit_batch_transaction` | Before pretransaction effects and after IMMEDIATE; canonical, admin/op-store and custom-trigger fallback batches |
| `guard_admin`, `maintain_before_writer` | Outer configuration, dependency registration, lifecycle transitions, consolidation, mean recompute, rebuild and importance mutation; before opportunistic closure writes |
| `actuate` | Initial receipt/guard lookup and owned IMMEDIATE transaction before receipts or operations; `apply_batch_in_transaction` remains an owned lower helper |
| `commit_projection_outcomes` | After IMMEDIATE before guard/publication or suppression, including custom-trigger fallback |
| `physical_erasure::transaction`, `physical_metrics::writer` | Standalone sanitation/finalization and foreground/autonomous physical writer admission; never a nested coalesced helper |
| `withdraw_queued_erasure`, maintenance handle queue/reattach/complete | Direct primary admissions and owned queue transaction, before durable/file effects; existing service/queue fences remain |
| `begin_erasure_maintenance` | After cooperative drain, before returning ownership; refusal unwinds process admission/freeze |
| `enable_telemetry` | Before sink creation and sink registration SQL; callback-only logging/atomic configuration acquires no new database lock |
| `boot_graft_declared_vector_backfill`, `reenqueue_stranded_vector_rows` | Their standalone transaction admission; fresh initialization occurs only after step37 |
| Frozen `binding_for_snapshot`, `validate_on_open` | Clean state before binding/validation; historical private migration fixtures below37 retain historical manifests |
| Offline recovery admission | Existing exact-version preflight plus current visibility-state/manifest validation before maintenance mutation |

Pure reads, subscriber bookkeeping and shutdown are not new SQL mutation
admissions. Close must still release all resources when persisted state is bad.
Every existing mutating transaction repeats `require_clean` after IMMEDIATE has
acquired the SQLite writer lock, before its first effect. Pretransaction checks
also remain where needed before maintenance/replay/file effects. Existing
autocommit mutations receive an owned IMMEDIATE boundary and the same check; a
clean SELECT outside that boundary alone is insufficient. In particular,
`mark_physical_incomplete` checks its own transaction, and telemetry registration
holds the new boundary before touching its sink. Implementation review checks
every primary/runtime write callsite against this table and adds any uncovered
concrete ingress before GREEN.

## Decisive tests and release truth

Prepare and execute a real sentinel SELECT before IMMEDIATE, execute it while
armed and after restore, and require `StatementStatus::RePrepare==0`. The existing
DBCONFIG seam must fail this oracle. Tests use `Transaction::new_unchecked` only to
retain an immutable pre-prepared statement borrow while owning IMMEDIATE.

Behavior REDs cover constrained/default flag37, exact57 manifest, malformed and
committed-armed entry refusal across canonical/fallback/noncanonical, actuation,
maintenance, projection and frozen binding; mutation-free36 rejection with WAL;
external invalidation and active vector cleanup; generation exhaustion, ABORT,
ROLLBACK, savepoints, commit-hook abort, retry/reopen and cursor/nonce atomicity.
A two-connection control performs a clean precheck, commits armed1 externally,
then requires owned-transaction admission to fail before effects.
Existing meaningful recovery/property suites remain; add a compact rollback
sequence property only where the changed guard state needs it. Freeze RED tests.

Update maintained interfaces/layout/migrations/engine and pending .28 compatibility,
getting-started/release notes plus the target schema. Historical proof records and
versioned token fixtures stay unchanged. Root owns final full verification,
unchanged5s/60s AC011 qualification and publication disposition; no worker perf run.
