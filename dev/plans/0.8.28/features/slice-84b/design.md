---
title: Slice 84b draft design — connection-bound certification and envelope deltas
status: APPROVED
---

# Slice 84b draft design

This draft chooses a bounded **process-local, connection-bound cleanup-job
certificate**, not a new durable counter format. The existing root journal stays
authoritative. The certificate accelerates supported foreground admission only;
it never authorizes durable physical completion. All full validators remain
available as the reference/fallback and run at admission, reopen and finalization.
Astra must review the completed source-level effect inventory before code.

## Owner and transaction lifecycle

One private owner lives with the Engine's primary connection/cleanup lifecycle.
It holds resource totals and, separately, an optional structural-zero witness.
All access occurs while holding that same primary connection mutex, in one
fixed lock order; no globals, thread-local authority or per-worker copies.
No pending root means no certificate. Close discards it. This is not a general
cache of arbitrary queries and requires no new public API or durable format.

Bind it to connection identity, root version/immutable operation and denied
scope, dependency generation, certified safe watermark and the known sanitation
phase. Acquire the existing `BEGIN IMMEDIATE` writer transaction before consuming a
certificate or comparing drift tokens. Inside that transaction compare external
`data_version`, schema cookie and a centrally tracked same-connection mutation
epoch with the certificate binding. Initial construction reads exact totals,
but does not enforce thresholds earlier than original post-effect certification:
retention may bring an oversized metadata total back under its cap. A missing
certificate takes this path, not an additional refusal. An unsuccessful optional
early structural preproof discards the witness and runs the original full
post-effect validator; it must not introduce an early failure.

A transaction creates a private working copy and delta recorder. Eligibility
checks run before effects. Publish updated totals/witness only **after SQLite
COMMIT succeeds, while the primary mutex is still held**; the committed root
safe watermark and certificate must describe the same state. Capture the external
version token **before COMMIT while the SQLite writer lock is still held** and
publish that same token after commit; never rebaseline from a post-commit read
that could bless an intervening external commit. On validation error,
rollback, deferred COMMIT failure or uncertain commit outcome, discard/invalidate
the working certificate. Process exit between commit and publication loses only
the cache; reopen reconstructs from exact database authority. Never persist a
speculative watermark or expose a working copy to the worker.

`total_changes`, `PRAGMA data_version` and schema/version observations are
invalidation hints, not proof of effect coverage. `total_changes` includes rolled
back work and does not identify semantic effects; `data_version` excludes commits
on the same connection. Every supported mutation path needs an explicit effect
record. All same-connection mutation entry points, including direct SQL/test
hooks, must advance the central mutation epoch or invalidate before execution;
a path that cannot be observed centrally disables certificate reuse. This drift
fence is an implementation prerequisite, not an assumption that hints provide
coverage. Unrecognized paths invalidate before returning as well. External commits, replacement connections,
unknown schema, changed scope/generation, unexpected journal/phase transitions,
missing/corrupt debt or uncertain effect coverage force exact validation and
reconstruction or the original refusal. A known sanitation-only transition may
refresh the binding only after proving its effects preserve the certified facts.

## Eligibility and structural-proof work

Decode and validate root/denial data once within a transaction and pass an
immutable context to its checks. Retain per-row database queries after earlier
batch effects. Preflight results outside the transaction are not reusable proof.
Existing logical/provenance indexes and equivalent query forms are the first
optimization candidates; no new index or head-only predicate is assumed safe.
A durable index proposal must separately document schema version, existing-store
creation, storage/write cost and reviewed ADR/interface impact before adoption.

The structural-zero fast path is conditional on a complete mutation-effect proof:

| Effect | Required invariant before omitting a repeated scan |
| --- | --- |
| Canonical inserts/supersession | New committed cursors exceed every captured erased cursor; old/new source, stable identity and endpoints pass unchanged guards, including earlier batch members. Updating surviving history cannot recreate erased rows. |
| Registered projections and attributes | Enumerate every create/update/delete call. New rows carry proved surviving cursors; deletions cannot recreate debt. Every registered family stays covered, including orphan/displaced FTS cursor values and property multiplicity. |
| Vector mappings/storage | Prove whether the admitted path can allocate/reuse a vector rowid or publish a projection. Never infer safety from owner-cursor ordering: captured mapped IDs may differ. Any unproved effect runs the full exact validator. |
| Provenance/dependencies | Plain legacy artifact registration and any allowed retention effect are covered. Registration/admin/governed/publication paths remain fenced. Dependency generation is checked, not guessed from the root. |
| Maintenance/metadata | Enumerate sanitation, closure/blocker and journal changes. Only proved noninterference can carry the witness forward; every other transition invalidates. Redacted finalization never consumes this cache. |

After a modeled transaction, advance the existing root safe watermark exactly as
before. The witness can be retained only when all effects preserve the invariant.
An effect not yet modeled uses the full validator, preserving the supported write
class. Finalization always performs the complete authoritative scan, original
sink obligations and successful TRUNCATE. A cached boolean cannot discharge debt.
Do not replace FTS `write_cursor` by `rowid` or MATCH: migrated body rows can have
independent rowids, and property rows are not one-to-one with owner cursors.

## Envelope totals and actual deltas

The certificate stores exact canonical row count, body-byte sum and maximum,
vector-mapping count, and metadata-byte totals for every table currently counted
by `envelope`. Initial values come from that exact SQL reference without applying its admission
thresholds early; enforce thresholds at the original post-effect boundary. Keep resource
caps and error precedence unchanged; review any necessary enforcement-order
correction explicitly rather than silently changing public refusals.

Calculate deltas from **actual old/new SQLite rows**, not batch length, submitted
strings, change counts or estimated file growth. Reuse SQL expressions equivalent
to `length(CAST(value AS BLOB))` and NULL/COALESCE for every counted column. Preserve
SQLite conversion/overflow behavior, saturation and exact threshold comparisons.
Use checked exact internal totals; never subtract from a saturated aggregate.
Unknown columns/schema invalidate the expression/cache and take the full path.

| Counted effects | Concrete delta source and required coverage |
| --- | --- |
| Canonical node/edge rows and bodies | Current plain writes insert rows and retain superseded history; supersession changes metadata, not body totals. Read actual inserted rows by returned cursor and check affected-row success. Include unnamed edges and same-batch supersession. Any body update/delete requires actual old/new values. |
| Legacy artifact metadata | `register_artifact_identity` inserts a real legacy revision. Measure every column of that inserted row using SQLite expressions; conflict/failure publishes no delta. Do not assume supplied identifiers are the entire metadata cost. |
| Provenance/receipt/operational metadata | Inventory all nine tables in the current envelope. Even plain writes call `enforce_provenance_retention`, which can delete old `operational_mutations`: capture the exact affected rows in the same transaction, retaining audit/pending-redaction exemptions. Cover maintenance closure/blocker updates and any actual old/new column lengths. |
| Vector mappings | Count actual `_fathomdb_vector_rows` inserts/deletes from synchronous projection helpers, including retained-revision policy. An absent effect must be demonstrated from the called path; otherwise exact fallback. |
| Maximum body length | Inserts take the exact maximum; deletes/updates of the last maximal row recompute exact MAX as the initial bounded fallback. Do not add a multiset/index unless profiling justifies it. Retained historical maxima continue to count. |
| Physical pages | FTS/vector shadow allocation is independent of logical deltas. Query actual `page_count * page_size` after relevant transaction growth at the certification boundary; never infer it from payload bytes. |

Implement a small transaction effect recorder around the actual mutation helpers,
using keyed before/after SQL or appropriate RETURNING results. Do not build a
second persistence framework. Trigger fallback remains refused. Preupdate hooks
are not a universal substitute: SQLite excludes virtual/system tables, and their
callback values do not themselves establish every envelope conversion or physical
allocation. Every mutation touching counted state either supplies a proved delta
or invalidates the certificate before the next certified operation.

## Verification boundaries

Retain the unoptimized full validator/envelope as a test oracle. On real stores,
generate insert, supersede, unnamed-edge, retention, failed/deferred commit,
rollback, unknown-write, phase/generation, external-commit and reopen sequences.
Include a cold/hot certificate case where retention takes metadata from above
the envelope cap to below it, and a rendezvous where an external writer commits
after our COMMIT but before cache publication. After each transition compare
optimized admission/refusal, totals and survivors
with the full reference; independently inspect physical residue. Include corrupt
or missing certificate/journal cases and reused/non-head vector IDs. Assert exact
threshold behavior and fail-closed handling without weakening existing oracles.

RED work tests must count executed work and prove both targets independently:
less repeated eligibility/proof work, and envelope cost proportional to actual
changed rows rather than all surviving data on the modeled path. Keep worst-case
fallback observable. Cold/reopen reconstruction cost and invalidation frequency
are part of service qualification; a fast warm path with constant invalidations
is not a successful optimization. See the plan for staged screening and final
paired acceptance, not an automatic repeat of the previous campaign.

## Source reconciliation and selected implementation — 2026-11-03

Baseline is retained Slice 84 plus the reviewed WAL harness selector correction
`77e2de796`. Production scheduling, checkpoints and erasure ADR are identical to
Slice 83; 84's scheduling candidate remains rejected. Astra reviewed the actual
commit/projector/retention paths and this implementation mechanism before code.

The primary mutex owns access to a certificate slot on PhysicalMaintenance. Only
managed canonical commits consume it, inside BEGIN IMMEDIATE. Drift compares
connection identity, total_changes, main/temp schema cookies, data_version and
exact root/phase binding. These are invalidation fences; explicit effects supply
accounting. No blob-writing or writable-schema path exists in the engine.

| Source effect | Accounting and proof disposition |
| --- | --- |
| Canonical insert | Query actual returned cursor range for row/body/max deltas. New cursors must exceed prior safe boundary and every captured cursor. |
| Supersession | Bodies/history retained; existing vector eligibility updated, attributes/property projections deleted. No erased cursor is recreated. |
| Legacy registration | Query actual inserted artifact rows, all columns using SQLite BLOB lengths. |
| Retention | Sum exact rows selected by existing exemption/order/limit immediately before deletion in the same transaction. No early threshold check. |
| Other seven metadata tables | Unchanged on plain legacy path; provenance guards preclude source soft-closure creation. |
| Vector mappings | Synchronous projection enqueues work; existing mapping metadata changes only. Enrollment/stranded repair uses full fallback. |
| Worker/sanitation/blockers | Conservatively invalidate on any drift. No carry across these transitions in the first candidate; report resulting cold cost. |
| Finalization/reopen | Original full validators, sink obligations and TRUNCATE remain authoritative. |

No TEMP delta triggers: application triggers remain refused and managed commits
disable triggers. Cold totals are unchecked until the original post-effect
certification point. Optional early zero proof failure selects original postproof.
Unknown effects use the unchanged full path. Publish only after successful COMMIT
using the version captured while the writer lock was held.

## Final disposition

Candidate `1e9c6d584` passed focused correctness and work-reduction checks, but
failed the edge service screen. Astra recommends rejection: retain the controller,
negative evidence and candidate in Git; restore baseline production code and its
original tests. Node passed only its first-pair screen, and property was not run.
Neither target has a qualified integrated benefit.

In the service screen, all 33 node and 21 edge modeled commits reconstructed
totals; there were no warm reconstruction savings. Isolated warm tests prove the
mechanism, but the conservative validity boundary did not survive the real
workload. Exact invalidation causes were not instrumented, so no particular
worker/maintenance transition is asserted as the cause. A successor would first
attribute those invalidations and prove any authority-preserving transitions;
that is separate work, not an automatic replacement campaign.
