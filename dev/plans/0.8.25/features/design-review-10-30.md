---
title: 0.8.25 Slice 10–30 design review — cycle 0
status: COMPLETE
review_cycle: 0
reviewed_on: 2026-09-01
verdict: FAIL
scope: dev/plans/0.8.25/features/slice-{10,15,20,25,30}/design.md
---

# Slice 10–30 independent design review — cycle 0

## Verdict

**FAIL.** The five designs are well bounded and use the correct architecture
direction, but unresolved P1/P2 findings remain. They must stay `DRAFT_REVIEW`.
This review evaluates implementation-shaping contract decisions only; it does
not request prose cleanup and does not edit the reviewed designs.

## Findings

### P1 — Slice 10 cannot represent the exact GLOBAL-01 execution paths

**File/section:** `slice-10/design.md`, **Contract** and **Migration and data
flow**.

`engine_search` is one run-global record and names `witness_ids`, but the schema
defines no witness objects for those IDs. A comparison in which only one arm
calls `Engine.search`, or in which both arms call it with different caller-side
processing, cannot state execution per arm/call path. An overall `executed`
value loses the distinction R25-10 exists to preserve; an unwitnessed
`not_executed` value is likewise not auditable.

**Correction:** add versioned `execution_witnesses` with ID, comparison arm,
component/call-path ID, execution state, call count, instrumentation/evidence
kind, and immutable source hash. Metrics and comparison arms must reference
these witnesses. Define `not_executed` as requiring a coverage witness, not
absence. Pin the historical classification exactly:

- first GLOBAL-01 FathomDB storage-backed arm bypassed `Engine.search`;
- its native GraphRAG arm is an external system path, not a FathomDB search;
- the 39-question held-out control and treatment both used `Engine.search`, but
  their answer metrics remain end to end because caller planning and answer
  generation contributed; and
- the new native direct-search witness is data-plane only.

Sidecars must identify the exact source receipt/result hashes for each path.

### P1 — Slice 15 conflates artifact revision with source revision

**File/section:** `slice-15/design.md`, **Public identity/provenance contract**.

`ProvenanceV1` has one `record_revision_id`, while `SourceLocator` is described
as a byte range “over a revision.” A caller-derived passage/fact/summary has its
own immutable artifact revision and locates bytes in a different canonical
source revision. The proposed shape cannot represent both without ambiguity,
so Slice 50 could resolve the derived body instead of exact source bytes.

**Correction:** separate `artifact_revision_id` from
`source_revision_id`. Bind `SourceVersionId`, `SourceLocator`, and canonical
source hash to the canonical source revision. A derived artifact retains its
own immutable revision and one or more provenance/dependency links to source
revisions. Define whole-body as the canonical source revision's full byte
range. Update uniqueness, insert, reindex, supersession, and codec fixtures for
both identities.

### P1 — Slice 15 does not define a total legacy provenance migration

**File/section:** `slice-15/design.md`, **Persistence and migration**.

The migration assigns every existing row a source version and locator, but
historical row classes include missing/legacy source identity and Engine-owned
derived projections. `ProvenanceV1` requires `source_id`, while the migration
does not say which classes are canonical, caller-derived, or projections, how
NULL/legacy source identity is handled, or how a synthetic version remains
stable without falsely claiming exact provenance. A per-row
`legacy:<sha256>` can also collapse identical content across different sources
unless its identity inputs are specified.

**Correction:** provide a table-by-table migration matrix. Canonical rows with a
known source get a deterministic version scoped by source and immutable row
identity. Caller-derived rows migrate through registered provenance/dependency
links or remain explicitly `migrated_incomplete` and ineligible for complete
evidence until repaired. Engine-owned projections do not become canonical
source records. Define the deterministic namespace/input bytes, collision and
reserved-prefix rules, restart behavior, erasure handle, and typed open/resolve
outcomes for irrecoverable legacy provenance. Do not fabricate “complete”
source locators.

### P2 — Slice 15 leaves opaque identity validation and scope unspecified

**File/section:** `slice-15/design.md`, **Public identity/provenance contract**.

`RecordRevisionId` is caller-supplied or Engine-minted and called globally
unique, but its database/global scope, maximum length, opaque/non-PII rule,
reserved namespace, and conflict behavior across artifact classes are not
defined. These choices determine indexes, erasure-safe telemetry, and wire
validation.

**Correction:** define one cross-artifact uniqueness scope, bounded opaque ASCII
syntax, caller and Engine namespace rules, non-PII requirement, maximum encoded
length, and typed duplicate/conflict behavior. Keep ULID as the Engine format
only if it satisfies the selected scope; callers must not mint the reserved
Engine namespace.

### P1 — Slice 20 liveness varies with caller-visible `ReadView`

**File/section:** `slice-20/design.md`, **Persistence, transactions, and
liveness**.

A member “survives” when visible/active under the operation snapshot. Visibility
can include caller access predicates and relaxed historical/inactive views;
those must not change structural dependency liveness. Valid-time passage also
needs one deterministic effective instant and consequence rule. Otherwise two
callers can derive different closure for the same dependency set.

**Correction:** define an Engine-owned `DependencyLivenessView`: fixed effective
validity instant and strict lifecycle/currentness rules, independent of caller
eligibility, access filtering, or `include_inactive/include_superseded`
relaxations. Registration records the reference mode; evaluation/closure uses
the closure operation's fixed instant. Specify how a validity boundary becoming
effective is detected and fenced before retrieval (write-triggered,
boundary-check, or scheduled mechanism), with Slice 30 applying consequences.

### P2 — Slice 20 reference and set-version semantics are not persistable as written

**File/section:** `slice-20/design.md`, **Public/wire contract** and
**Persistence, transactions, and liveness**.

An `ArtifactRefV1` without a revision “binds” the validity-selected revision at
the snapshot, but it is unclear whether storage preserves a logical-current
reference or normalizes to that immutable revision. Replacement claims a new
immutable set revision, yet the proposed tables do not define set revision,
currentness, historical query, or delete semantics. Cycle validation depends
on which revisions are active.

**Correction:** make reference mode explicit and closed, for example
`LogicalCurrent { id: IdSpace } | PinnedRevision { revision_id }`, and define
whether logical-current follows future revisions. Persist immutable
`dependency_set_revision_id` plus stable set ID/current pointer and lifecycle.
Replacement atomically retires the old revision; deletion retires rather than
destroying audit history. Cycle/liveness/query rules operate on the active
prospective set revisions, while historical reads name a revision explicitly.

### P1 — Slice 25 depends on Slice 35/40 contracts that do not exist yet

**File/section:** `slice-25/design.md`, **Public/wire contract** and
**Persistence and transaction flow**.

`expected_snapshot`, `resulting_snapshot`, and `projection_work` imply the
future frozen-snapshot and durable projection-generation contracts. The release
ladder implements Slice 25 before Slices 35 and 40, so the implementer cannot
produce the specified public receipt without either inventing those later
contracts or violating sequencing. Slice 30 similarly refers to generations.

**Correction:** define the Slice-25-owned baseline now:
`ExpectedWriteBoundaryV1`/`ResultingWriteBoundaryV1` using available database
identity and canonical write boundary, plus a versioned
`ProjectionWorkIntentV1` using current projection name/cursor/terminal state.
Slices 35 and 40 add frozen-read and generation-bound successor fields
additively. State explicitly that Slice 25 cannot emit or accept those future
types. Apply the same staging rule to Slice 30's status/projection drain.

### P1 — Slice 25 refusal idempotency is not crash-stable

**File/section:** `slice-25/design.md`, **Persistence and transaction flow**.

Domain rollback is followed by a refusal receipt in a separate transaction. A
crash between them leaves no recorded refusal; retry can validate against a
different database state and return a different outcome for the same
operation/digest. That violates the idempotent operation identity and complete
receipt requirement.

**Correction:** specify a durable journal state machine that reserves
`(operation_id, request_sha256)` and captures the validation boundary before
execution. Recovery must deterministically finish or return the same committed
or refused outcome. Domain mutations and the committed receipt remain one
transaction; refusal finalization must be recoverable from the admitted
request/boundary without re-deciding against later state. Define concurrent
same-ID behavior and terminal journal retention.

### P2 — Slice 25 has an undefined large-closure branch

**File/section:** `slice-25/design.md`, **Persistence and transaction flow**.

“Closure too large for the transaction” selects materially different visibility
and receipt behavior but has no threshold or deterministic preflight. An
implementer must invent when a batch becomes `committed_closure_pending`.

**Correction:** make lifecycle operations always create the Slice 30 closure
intent/fence path, or define an exact, testable threshold based only on
pre-commit counts. In either case, the fence and intent must commit atomically
with the semantic batch and the receipt must name the closure operation ID.

### P1 — Slice 30 `any_surviving` erasure can retain erased source bytes

**File/section:** `slice-30/design.md`, **Liveness, transactions, and recovery**.

For source erasure, the design preserves a dependent under `any_surviving` and
drops only the erased member. A caller-authored fact/summary/passage body may
still contain bytes or semantic material derived from that erased source.
FathomDB cannot prove source-separable content, so removing the dependency edge
does not prove erasure and destroys the forward audit link.

**Correction:** separate lifecycle liveness from physical-erasure consequence.
Unless the artifact is Engine-owned and has a proved source-separable
projection representation, any dependent that may contain erased-source
material is fenced/inactivated or erased. A surviving-source caller-derived
artifact becomes visible only after the external semantic layer submits a new
revision whose bytes and dependencies exclude the erased source. Preserve the
removed dependency in non-content audit history; never silently detach and
retain the same body.

### P1 — Slice 30's total-impact cap can make erasure impossible

**File/section:** `slice-30/design.md`, **Invariants, failures, compatibility,
and limits**.

Refusing before fencing above 100,000 impacted artifacts leaves no release-owned
path to erase a larger dependency closure, contradicting resumable erasure and
lifecycle closure. A work-chunk limit is appropriate; a total closure limit is
not.

**Correction:** remove the total-impact refusal. Use durable paged frontier/work
records and bounded propagation transactions, while the initial operation
atomically installs a conservative root/source barrier. Limits apply per
transaction/page and produce resumable progress, not permanent refusal. Define
an operator recovery route for resource exhaustion without lifting visibility
fences.

### P1 — Slice 30 does not close post-enumeration dependency races

**File/section:** `slice-30/design.md`, **Liveness, transactions, and recovery**.

The impact set is computed at one snapshot, then fenced. “Conflicting mutation
of fenced artifacts” does not prohibit a new dependent from referencing a
fenced/erasing root after enumeration, because the new dependent itself was not
in the impact set. Completion proof can therefore race a newly added orphan.

**Correction:** persist a root/source closure barrier in the same writer
transaction that admits closure. Every write, dependency registration,
projection, and actuation path must reject references to or derivation from a
barriered artifact/source. Fence insertion validates the impact boundary under
the single writer before commit. Final proof runs at a later recorded write
boundary after projection/WAL work and checks no post-plan dependency exists.
Only proof may retire the barrier; failed/incomplete operations retain it.

## Required FIX-1 outcome

FIX-1 should correct the twelve findings without changing accepted historical
designs. Re-review must confirm:

1. GLOBAL-01 is classified per exact arm/path with auditable witnesses.
2. Artifact and canonical-source revisions are distinct and legacy migration is
   total without fabricated provenance.
3. Dependency reference, set-version, cycle, and structural-liveness semantics
   are deterministic.
4. Slice 25/30 expose only contracts available at their dependency stage.
5. Refusal replay is crash-stable and large closure is deterministic/resumable.
6. Erasure never retains possibly source-derived bytes and closure barriers
   prevent post-enumeration races.

No Slice 10–30 design may advance to READY while any P1/P2 above is unresolved.
